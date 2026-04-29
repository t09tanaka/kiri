//! Request handlers — one async fn per `Request` variant.

use super::dispatch::DispatchContext;
use super::run_logic::{extract_output, tail_lines, Sentinel};
use kiri_cli_proto::{ErrorCode, PaneRef, Request, Response, SplitDirection};
use tauri::Emitter;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

pub async fn handle(ctx: &DispatchContext, req: Request) -> Vec<Response> {
    match req {
        Request::Ls => vec![ls(ctx).await],
        Request::Send { pane, data } => vec![send(ctx, pane, data).await],
        Request::Read { pane, since, tail } => vec![read(ctx, pane, since, tail).await],
        Request::Cancel { pane } => vec![cancel(ctx, pane).await],
        Request::Run {
            pane,
            cmd,
            timeout_secs,
            full_output,
        } => vec![run(ctx, pane, cmd, timeout_secs, full_output).await],
        Request::Split { pane, direction } => vec![split(ctx, pane, direction).await],
        Request::Close { pane } => vec![close_pane(ctx, pane).await],
        Request::Follow { pane } => follow(ctx, pane).await,
    }
}

async fn ls(ctx: &DispatchContext) -> Response {
    let entries = ctx.pane_map.snapshot();
    let mut panes = Vec::with_capacity(entries.len());
    for e in entries {
        let (process_name, memory_bytes, running) =
            process_info_for(&ctx.terminals, e.terminal_id);
        let cwd = cwd_for(&ctx.terminals, e.terminal_id);
        panes.push(kiri_cli_proto::PaneInfo {
            index: e.index,
            id: e.pane_id,
            terminal_id: e.terminal_id,
            cwd,
            process_name,
            running,
            memory_bytes,
            focused: e.focused,
        });
    }
    Response::Ls { panes }
}

fn process_info_for(
    state: &crate::commands::terminal::TerminalState,
    id: u32,
) -> (String, u64, bool) {
    use sysinfo::{Pid, ProcessesToUpdate, System};
    let mut manager = match state.lock() {
        Ok(g) => g,
        Err(_) => return ("Terminal".into(), 0, false),
    };
    let Some(instance) = manager.instances.get_mut(&id) else {
        return ("Terminal".into(), 0, false);
    };
    if matches!(instance.child.try_wait(), Ok(Some(_))) {
        return ("Terminal".into(), 0, false);
    }
    let Some(shell_pid) = instance.shell_pid else {
        return ("Terminal".into(), 0, false);
    };
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All);
    let mut total: u64 = 0;
    if let Some(p) = sys.process(Pid::from_u32(shell_pid)) {
        total += p.memory();
    }
    let children: Vec<_> = sys
        .processes()
        .values()
        .filter(|p| {
            p.parent()
                .map(|pp| pp.as_u32() == shell_pid)
                .unwrap_or(false)
        })
        .collect();
    for c in &children {
        total += c.memory();
    }
    let name = if let Some(c) = children.first() {
        c.name().to_string_lossy().to_string()
    } else if let Some(p) = sys.process(Pid::from_u32(shell_pid)) {
        p.name().to_string_lossy().to_string()
    } else {
        "Terminal".to_string()
    };
    (name, total, !children.is_empty())
}

fn cwd_for(state: &crate::commands::terminal::TerminalState, id: u32) -> Option<String> {
    let mut manager = state.lock().ok()?;
    let instance = manager.instances.get_mut(&id)?;
    if matches!(instance.child.try_wait(), Ok(Some(_))) {
        return None;
    }
    let pid = instance.shell_pid?;
    crate::commands::terminal::get_process_cwd(pid)
}

async fn send(ctx: &DispatchContext, p: PaneRef, data: String) -> Response {
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let mut manager = match ctx.terminals.lock() {
        Ok(g) => g,
        Err(_) => return internal("terminal state poisoned"),
    };
    let Some(instance) = manager.instances.get_mut(&pane.terminal_id) else {
        return pane_not_found(p);
    };
    use std::io::Write;
    if let Err(e) = instance.writer.write_all(data.as_bytes()) {
        return pty_error(format!("write failed: {e}"));
    }
    if let Err(e) = instance.writer.flush() {
        return pty_error(format!("flush failed: {e}"));
    }
    Response::Send
}

async fn cancel(ctx: &DispatchContext, p: PaneRef) -> Response {
    match send(ctx, p, "\x03".into()).await {
        Response::Send => Response::Cancel,
        other => other,
    }
}

async fn read(
    ctx: &DispatchContext,
    p: PaneRef,
    since: Option<u64>,
    tail: Option<usize>,
) -> Response {
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let rb = ctx.buffers.ensure_subscribed(pane.terminal_id, &ctx.bus);
    let buf = rb.lock().expect("ring buffer mutex poisoned");
    let cursor = buf.cursor();
    let (bytes, dropped) = if let Some(n) = tail {
        let (b, _) = buf.tail_lines(n);
        (b, 0u64)
    } else {
        buf.read_since(since.unwrap_or(0))
    };
    Response::Read {
        output: String::from_utf8_lossy(&bytes).into_owned(),
        cursor,
        bytes_dropped: dropped,
    }
}

async fn follow(ctx: &DispatchContext, p: PaneRef) -> Vec<Response> {
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return vec![pane_not_found(p)];
    };
    let rb = ctx.buffers.ensure_subscribed(pane.terminal_id, &ctx.bus);
    // v1: snapshot + end. Real streaming will land in a follow-up.
    let (bytes, cursor) = {
        let guard = rb.lock().expect("rb");
        let cursor = guard.cursor();
        let (bytes, _) = guard.read_since(0);
        (bytes, cursor)
    };
    vec![
        Response::FollowChunk {
            data: String::from_utf8_lossy(&bytes).into_owned(),
            cursor,
        },
        Response::FollowEnd,
    ]
}

async fn run(
    ctx: &DispatchContext,
    p: PaneRef,
    cmd: String,
    timeout_secs: u64,
    full_output: bool,
) -> Response {
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };

    // Busy-check: if the shell currently has child processes, refuse.
    {
        use sysinfo::{ProcessesToUpdate, System};
        let pid_opt = {
            let mut manager = match ctx.terminals.lock() {
                Ok(g) => g,
                Err(_) => return internal("terminal state poisoned"),
            };
            let Some(instance) = manager.instances.get_mut(&pane.terminal_id) else {
                return pane_not_found(p);
            };
            if matches!(instance.child.try_wait(), Ok(Some(_))) {
                return pty_error("shell exited".into());
            }
            instance.shell_pid
        };
        if let Some(pid) = pid_opt {
            let mut sys = System::new();
            sys.refresh_processes(ProcessesToUpdate::All);
            let mut name: Option<String> = None;
            let busy = sys.processes().values().any(|proc| {
                let is_child = proc
                    .parent()
                    .map(|pp| pp.as_u32() == pid)
                    .unwrap_or(false);
                if is_child && name.is_none() {
                    name = Some(proc.name().to_string_lossy().to_string());
                }
                is_child
            });
            if busy {
                return Response::Error {
                    code: ErrorCode::PaneBusy,
                    message: format!(
                        "pane {} is running '{}'",
                        pane.index,
                        name.clone().unwrap_or_else(|| "unknown".into())
                    ),
                    detail: Some(serde_json::json!({ "process": name })),
                };
            }
        }
    }

    let nonce = format!("{:08x}", rand_nonce());
    let sentinel = Sentinel::new(nonce);
    let payload = sentinel.payload(&cmd);

    // Subscribe BEFORE writing so we can't miss the sentinel chunk.
    let mut rx: broadcast::Receiver<Vec<u8>> = ctx.bus.subscribe(pane.terminal_id);

    {
        use std::io::Write;
        let mut manager = match ctx.terminals.lock() {
            Ok(g) => g,
            Err(_) => return internal("terminal state poisoned"),
        };
        let Some(instance) = manager.instances.get_mut(&pane.terminal_id) else {
            return pane_not_found(p);
        };
        if let Err(e) = instance.writer.write_all(payload.as_bytes()) {
            return pty_error(format!("write failed: {e}"));
        }
        let _ = instance.writer.flush();
    }

    let collect = async {
        let mut acc: Vec<u8> = Vec::new();
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    acc.extend_from_slice(&chunk);
                    if let Some((exit, end)) = sentinel.find(&acc) {
                        let text = extract_output(&acc, &cmd, end);
                        return (Some(exit), text);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return (None, String::from_utf8_lossy(&acc).into_owned());
                }
            }
        }
    };

    let (exit_code, text, timed_out) =
        match timeout(Duration::from_secs(timeout_secs), collect).await {
            Ok((exit, text)) => (exit, text, false),
            Err(_) => (None, String::new(), true),
        };

    let cursor = ctx
        .buffers
        .get(pane.terminal_id)
        .map(|b| b.lock().expect("rb").cursor())
        .unwrap_or(0);

    let (final_text, lines_omitted) = if full_output {
        (text, 0)
    } else {
        tail_lines(&text, 1000)
    };

    Response::Run {
        exit_code,
        output: final_text,
        output_truncated: lines_omitted > 0,
        lines_omitted,
        timed_out,
        cursor,
    }
}

async fn split(ctx: &DispatchContext, p: PaneRef, direction: SplitDirection) -> Response {
    let Some(app) = ctx.app.as_ref() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let request_id = format!("split-{}", uuid::Uuid::new_v4());
    let rx = ctx.pending.register(request_id.clone());
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "direction": match direction {
            SplitDirection::Horizontal => "horizontal",
            SplitDirection::Vertical => "vertical",
        },
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-split", payload) {
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let new_pane_id = value
                .get("newPaneId")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let new_pane_index = value
                .get("newPaneIndex")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            Response::Split {
                new_pane_id,
                new_pane_index,
            }
        }
        _ => Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: "frontend did not reply within 2s".into(),
            detail: None,
        },
    }
}

async fn close_pane(ctx: &DispatchContext, p: PaneRef) -> Response {
    let Some(app) = ctx.app.as_ref() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let request_id = format!("close-{}", uuid::Uuid::new_v4());
    let rx = ctx.pending.register(request_id.clone());
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-close", payload) {
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(_)) => Response::Close,
        _ => Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: "frontend did not reply within 2s".into(),
            detail: None,
        },
    }
}

fn pane_not_found(p: PaneRef) -> Response {
    Response::Error {
        code: ErrorCode::PaneNotFound,
        message: format!("no pane matches {p:?}"),
        detail: None,
    }
}

fn internal(msg: &str) -> Response {
    Response::Error {
        code: ErrorCode::InternalError,
        message: msg.into(),
        detail: None,
    }
}

fn pty_error(msg: String) -> Response {
    Response::Error {
        code: ErrorCode::PtyError,
        message: msg,
        detail: None,
    }
}

fn rand_nonce() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::super::dispatch::{DispatchContext, TerminalBuffers};
    use super::super::frontend_bridge::PendingReplies;
    use super::super::pane_map::{PaneEntry, PaneMap};
    use super::*;
    use crate::commands::terminal::{TerminalManager, TerminalOutputBus, TerminalOutputBusState, TerminalState};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;

    fn make_ctx(
        pane_entries: Vec<PaneEntry>,
    ) -> (DispatchContext, TerminalOutputBusState) {
        let terminals: TerminalState = Arc::new(StdMutex::new(TerminalManager::new()));
        let bus: TerminalOutputBusState = Arc::new(TerminalOutputBus::new());
        let pane_map = Arc::new(PaneMap::new());
        pane_map.replace(pane_entries);
        let ctx = DispatchContext {
            label: "test".into(),
            app: None,
            terminals,
            bus: bus.clone(),
            pane_map,
            pending: Arc::new(PendingReplies::new()),
            buffers: Arc::new(TerminalBuffers::new()),
        };
        (ctx, bus)
    }

    #[tokio::test]
    async fn ls_with_no_panes_returns_empty_list() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = ls(&ctx).await;
        match resp {
            Response::Ls { panes } => assert!(panes.is_empty()),
            other => panic!("expected Ls, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_returns_buffered_bytes() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
        };
        let (ctx, bus) = make_ctx(vec![entry]);
        // Touch the buffer so the subscriber is installed before publish.
        let _ = ctx.buffers.ensure_subscribed(1, &ctx.bus);
        tokio::time::sleep(Duration::from_millis(20)).await;
        bus.publish(1, b"hello world");
        tokio::time::sleep(Duration::from_millis(20)).await;

        let resp = read(&ctx, PaneRef::Index(0), None, None).await;
        match resp {
            Response::Read {
                output,
                cursor,
                bytes_dropped,
            } => {
                assert_eq!(output, "hello world");
                assert_eq!(cursor, 11);
                assert_eq!(bytes_dropped, 0);
            }
            other => panic!("expected Read, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_to_unknown_pane_returns_pane_not_found() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = send(&ctx, PaneRef::Index(99), "data".into()).await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
