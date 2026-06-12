//! Request handlers — one async fn per `Request` variant.

use super::dispatch::DispatchContext;
use super::run_logic::{extract_output, tail_lines, Sentinel};
use super::signals::{now_ms, Signal, MAX_SIGNAL_WAIT_SECS};
use crate::commands::lock_ext::LockExt;
use kiri_cli_proto::{ErrorCode, PaneRef, Request, Response, SignalTarget, SplitDirection};
use tauri::Emitter;
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

const MAX_RUN_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

pub async fn handle(ctx: &DispatchContext, req: Request) -> Vec<Response> {
    match req {
        Request::WhoAmI => vec![whoami(ctx).await],
        Request::Ls => vec![ls(ctx).await],
        Request::Send { pane, data, submit } => vec![send(ctx, pane, data, submit).await],
        Request::Read { pane, since, tail } => vec![read(ctx, pane, since, tail).await],
        Request::Cancel { pane } => vec![cancel(ctx, pane).await],
        Request::Run {
            pane,
            cmd,
            timeout_secs,
            full_output,
        } => vec![run(ctx, pane, cmd, timeout_secs, full_output).await],
        Request::Split {
            pane,
            direction,
            name,
            color,
            minimized,
        } => vec![split(ctx, pane, direction, name, color, minimized).await],
        Request::Close { pane } => vec![close_pane(ctx, pane).await],
        Request::Minimize { pane } => vec![set_collapsed(ctx, pane, true).await],
        Request::Restore { pane } => vec![set_collapsed(ctx, pane, false).await],
        Request::Follow { pane } => follow(ctx, pane).await,
        Request::SetLabel {
            pane,
            set_name,
            clear_name,
            set_color,
            clear_color,
        } => vec![set_label(ctx, pane, set_name, clear_name, set_color, clear_color).await],
        Request::SignalSend {
            from,
            target,
            name,
            data,
        } => vec![signal_send(ctx, from, target, name, data).await],
        Request::SignalWait {
            pane,
            name,
            timeout_secs,
        } => vec![signal_wait(ctx, pane, name, timeout_secs).await],
        Request::SignalList { pane } => vec![signal_list(ctx, pane).await],
        Request::OpenWindow { dir, force_new } => vec![open_window(ctx, dir, force_new).await],
        Request::AgentStatus { pane, lines } => vec![agent_status(ctx, pane, lines).await],
    }
}

/// Report the window label and currently-open project path for this socket.
///
/// The CLI uses this to refuse acting on a window that belongs to a
/// different project than the user's current working directory. If no
/// `AppHandle` is available (only happens in unit tests) or no project
/// has been registered for this window, `project_path` is `None`.
async fn whoami(ctx: &DispatchContext) -> Response {
    let project_path = ctx
        .app
        .as_ref()
        .and_then(|app| project_path_for_label(app, &ctx.label));
    Response::WhoAmI {
        window_label: ctx.label.clone(),
        project_path,
    }
}

fn project_path_for_label(app: &tauri::AppHandle, label: &str) -> Option<String> {
    use crate::commands::window::WindowRegistryState;
    use tauri::Manager;
    let registry = app.try_state::<WindowRegistryState>()?;
    let guard = registry.lock().ok()?;
    guard.get_path_for_label(label).cloned()
}

async fn ls(ctx: &DispatchContext) -> Response {
    let entries = ctx.pane_map.snapshot();
    let mut panes = Vec::with_capacity(entries.len());
    for e in entries {
        let (process_name, memory_bytes, running) = process_info_for(&ctx.terminals, e.terminal_id);
        let cwd = cwd_for(&ctx.terminals, e.terminal_id);
        let ai_kind = ai_kind_from_process_name(&process_name);
        panes.push(kiri_cli_proto::PaneInfo {
            index: e.index,
            id: e.pane_id,
            terminal_id: e.terminal_id,
            cwd,
            process_name,
            running,
            memory_bytes,
            focused: e.focused,
            name: e.name,
            color: e.color,
            minimized: e.collapsed,
            ai_kind,
        });
    }
    Response::Ls { panes }
}

/// Classify a foreground process name as a known interactive AI
/// assistant. Pure so it can be unit-tested and reused by `ls` /
/// `agent_status` without a process snapshot. Kept in sync with the
/// `is_ai_process_for_shell_pid` allow-list (claude / codex).
fn ai_kind_from_process_name(process_name: &str) -> Option<String> {
    match process_name.to_lowercase().as_str() {
        "claude" => Some("claude".into()),
        "codex" => Some("codex".into()),
        _ => None,
    }
}

fn process_info_for(
    state: &crate::commands::terminal::TerminalState,
    id: u32,
) -> (String, u64, bool) {
    // Phase 1: hold the TerminalState mutex only for the cheap lookup.
    // Sysinfo's full process scan is slow (50–200ms on macOS) and we
    // must not block create_terminal/write_terminal/etc. for that long.
    let shell_pid = {
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
        match instance.shell_pid {
            Some(pid) => pid,
            None => return ("Terminal".into(), 0, false),
        }
    };

    // Phase 2: process snapshot is shared with the frontend polling path,
    // so multiple panes sampled together do not each refresh the full table.
    let info = crate::commands::terminal_commands::process_info_for_shell_pid(shell_pid);
    let running = crate::commands::terminal_commands::shell_has_child_process(shell_pid);
    (info.name, info.memory_bytes, running)
}

fn cwd_for(state: &crate::commands::terminal::TerminalState, id: u32) -> Option<String> {
    // Same locking discipline as process_info_for: extract pid under
    // the lock, release, then call into get_process_cwd which does its
    // own (slow) /proc lookup.
    let pid = {
        let mut manager = state.lock().ok()?;
        let instance = manager.instances.get_mut(&id)?;
        if matches!(instance.child.try_wait(), Ok(Some(_))) {
            return None;
        }
        instance.shell_pid?
    };
    crate::commands::terminal::get_process_cwd(pid)
}

async fn send(ctx: &DispatchContext, p: PaneRef, data: String, submit: bool) -> Response {
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };

    // For AI panes (claude / codex) with submit enabled, write the body
    // and the submit `\r` as two separate PTY writes with a brief gap
    // in between. Coalesced `body\r` bytes read in a single PTY chunk
    // get interpreted by the TUI as a paste with a trailing newline —
    // the input buffer ends up with an extra empty line and never
    // commits. Splitting the writes makes the TUI see the `\r` as a
    // distinct Enter keystroke.
    //
    // We strip a single trailing `\r` from `data` first so callers can
    // either rely on auto-submit (`kiri term send "msg"`) or include
    // the `\r` themselves (`kiri term send $'msg\r'`) and get the same
    // split-write behavior. `\n` is *not* a submit byte for these TUIs
    // (it's a soft newline within the input box, i.e. Shift+Enter), so
    // it stays in the body untouched.
    //
    // The shell-pid lookup is slow (~50–200ms) so we do it before
    // taking the terminal write lock.
    let shell_pid = {
        let manager = match ctx.terminals.lock() {
            Ok(g) => g,
            Err(_) => return internal("terminal state poisoned"),
        };
        let Some(instance) = manager.instances.get(&pane.terminal_id) else {
            return pane_not_found(p);
        };
        instance.shell_pid
    };
    let separate_submit = submit
        && shell_pid
            .map(is_ai_process_for_shell_pid)
            .unwrap_or(false);
    let body: &[u8] = if separate_submit {
        data.strip_suffix('\r').unwrap_or(&data).as_bytes()
    } else {
        data.as_bytes()
    };

    {
        let mut manager = match ctx.terminals.lock() {
            Ok(g) => g,
            Err(_) => return internal("terminal state poisoned"),
        };
        let Some(instance) = manager.instances.get_mut(&pane.terminal_id) else {
            return pane_not_found(p);
        };
        use std::io::Write;
        if let Err(e) = instance.writer.write_all(body) {
            return pty_error(format!("write failed: {e}"));
        }
        if let Err(e) = instance.writer.flush() {
            return pty_error(format!("flush failed: {e}"));
        }
    }

    let submitted = if separate_submit {
        // Brief gap so the AI TUI debounces the paste before the `\r`
        // arrives as a separate keypress. Without this, even with a
        // separate write_all call the bytes can still be read together
        // and treated as one paste.
        tokio::time::sleep(Duration::from_millis(80)).await;
        let mut manager = match ctx.terminals.lock() {
            Ok(g) => g,
            Err(_) => return internal("terminal state poisoned"),
        };
        let Some(instance) = manager.instances.get_mut(&pane.terminal_id) else {
            return pane_not_found(p);
        };
        use std::io::Write;
        if let Err(e) = instance.writer.write_all(b"\r") {
            return pty_error(format!("submit write failed: {e}"));
        }
        if let Err(e) = instance.writer.flush() {
            return pty_error(format!("flush failed: {e}"));
        }
        true
    } else {
        false
    };
    Response::Send { submitted }
}

/// True when the foreground process under `shell_pid` is an interactive
/// AI assistant whose input box accepts a single `\r` as submit. Kept in
/// sync with the frontend's `isAiProcess` allow list — claude / codex.
fn is_ai_process_for_shell_pid(shell_pid: u32) -> bool {
    let info = crate::commands::terminal_commands::process_info_for_shell_pid(shell_pid);
    matches!(info.name.to_lowercase().as_str(), "claude" | "codex")
}

async fn cancel(ctx: &DispatchContext, p: PaneRef) -> Response {
    // Cancel always sends a raw Ctrl-C, never auto-submits.
    match send(ctx, p, "\x03".into(), false).await {
        Response::Send { .. } => Response::Cancel,
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
    let buf = rb.lock_recover();
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
        let guard = rb.lock_recover();
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
            let busy = crate::commands::terminal_commands::shell_has_child_process(pid);
            if busy {
                let info = crate::commands::terminal_commands::process_info_for_shell_pid(pid);
                return Response::Error {
                    code: ErrorCode::PaneBusy,
                    message: format!("pane {} is running '{}'", pane.index, info.name),
                    detail: Some(serde_json::json!({ "process": info.name })),
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
        let mut capture_truncated = false;
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    acc.extend_from_slice(&chunk);
                    if acc.len() > MAX_RUN_CAPTURE_BYTES {
                        let overflow = acc.len() - MAX_RUN_CAPTURE_BYTES;
                        acc.drain(0..overflow);
                        capture_truncated = true;
                    }
                    if let Some((exit, start, end)) = sentinel.find(&acc) {
                        let text = extract_output(&acc, &cmd, start, end);
                        return (Some(exit), text, capture_truncated);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return (
                        None,
                        String::from_utf8_lossy(&acc).into_owned(),
                        capture_truncated,
                    );
                }
            }
        }
    };

    let (exit_code, text, timed_out, capture_truncated) =
        match timeout(Duration::from_secs(timeout_secs), collect).await {
            Ok((exit, text, capture_truncated)) => (exit, text, false, capture_truncated),
            Err(_) => (None, String::new(), true, false),
        };

    let cursor = ctx
        .buffers
        .get(pane.terminal_id)
        .map(|b| b.lock().expect("rb").cursor())
        .unwrap_or(0);

    let (final_text, mut lines_omitted) = if full_output {
        (text, 0)
    } else {
        tail_lines(&text, 1000)
    };
    if capture_truncated && lines_omitted == 0 {
        lines_omitted = 1;
    }

    Response::Run {
        exit_code,
        output: final_text,
        output_truncated: lines_omitted > 0,
        lines_omitted,
        timed_out,
        cursor,
    }
}

/// Validate a pane label name received over the wire.
///
/// Mirrors the rules enforced by the CLI's clap parser (1–32 unicode scalar
/// values, no control characters). Defense-in-depth: clients that speak the
/// raw JSON protocol bypass clap, so the server must re-check.
fn validate_pane_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("name must not be empty");
    }
    if name.chars().count() > 32 {
        return Err("name must be 32 characters or fewer");
    }
    if name.chars().any(|c| c.is_control()) {
        return Err("name must not contain control characters");
    }
    Ok(())
}

async fn split(
    ctx: &DispatchContext,
    p: PaneRef,
    direction: SplitDirection,
    name: Option<String>,
    color: Option<kiri_cli_proto::PaneColor>,
    minimized: bool,
) -> Response {
    if let Some(n) = name.as_deref() {
        if let Err(reason) = validate_pane_name(n) {
            return Response::Error {
                code: ErrorCode::InvalidArgument,
                message: reason.into(),
                detail: None,
            };
        }
    }
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
        "name": name,
        "color": color,
        "minimized": minimized,
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-split", payload) {
        ctx.pending.cancel(&request_id);
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let err_code = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(code) = err_code {
                return frontend_error_to_response(&code, value, "split");
            }
            let new_pane_id = value
                .get("newPaneId")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let new_pane_index = value
                .get("newPaneIndex")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            // Record the parent → child link so future
            // `signal send --target parent|children` calls can route
            // correctly. Skipped on empty pane id (defensive: shouldn't
            // happen, but never insert a bogus key).
            if !new_pane_id.is_empty() {
                ctx.signals
                    .register_parent(pane.pane_id.clone(), new_pane_id.clone());
            }
            Response::Split {
                new_pane_id,
                new_pane_index,
            }
        }
        _ => {
            ctx.pending.cancel(&request_id);
            Response::Error {
                code: ErrorCode::FrontendUnresponsive,
                message: "frontend did not reply within 2s".into(),
                detail: None,
            }
        }
    }
}

/// Map a frontend-reported error code (sent in the `error` field of the
/// `cli_resolve_pending` payload) to a wire-level [`ErrorCode`].
///
/// The frontend currently only emits `"no_focused_pane"`, but the helper
/// keeps unknown codes from being silently coerced to `PaneNotFound` and
/// surfaces them as `InternalError` so they show up as protocol-level
/// problems instead of looking like normal CLI usage errors.
fn frontend_error_to_response(
    err_code: &str,
    raw_payload: serde_json::Value,
    op: &str,
) -> Response {
    let code = match err_code {
        "no_focused_pane" => ErrorCode::PaneNotFound,
        _ => ErrorCode::InternalError,
    };
    Response::Error {
        code,
        message: format!("frontend rejected {op}: {err_code}"),
        detail: Some(raw_payload),
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
        ctx.pending.cancel(&request_id);
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let err_code = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(code) = err_code {
                return frontend_error_to_response(&code, value, "close");
            }
            Response::Close
        }
        _ => {
            ctx.pending.cancel(&request_id);
            Response::Error {
                code: ErrorCode::FrontendUnresponsive,
                message: "frontend did not reply within 2s".into(),
                detail: None,
            }
        }
    }
}

async fn set_label(
    ctx: &DispatchContext,
    p: PaneRef,
    set_name: Option<String>,
    clear_name: bool,
    set_color: Option<kiri_cli_proto::PaneColor>,
    clear_color: bool,
) -> Response {
    // Defence-in-depth: the CLI already enforces these, but raw JSON clients
    // bypass clap and must be rejected at the server boundary too.
    if set_name.is_some() && clear_name {
        return Response::Error {
            code: ErrorCode::InvalidArgument,
            message: "set_name and clear_name are mutually exclusive".into(),
            detail: None,
        };
    }
    if set_color.is_some() && clear_color {
        return Response::Error {
            code: ErrorCode::InvalidArgument,
            message: "set_color and clear_color are mutually exclusive".into(),
            detail: None,
        };
    }
    if set_name.is_none() && !clear_name && set_color.is_none() && !clear_color {
        return Response::Error {
            code: ErrorCode::InvalidArgument,
            message: "set_label requires at least one of set_name, clear_name, set_color, clear_color".into(),
            detail: None,
        };
    }
    if let Some(n) = set_name.as_deref() {
        if let Err(reason) = validate_pane_name(n) {
            return Response::Error {
                code: ErrorCode::InvalidArgument,
                message: reason.into(),
                detail: None,
            };
        }
    }

    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let Some(app) = ctx.app.as_ref() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };
    let request_id = format!("set-label-{}", uuid::Uuid::new_v4());
    let rx = ctx.pending.register(request_id.clone());
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "setName": set_name,
        "clearName": clear_name,
        "setColor": set_color,
        "clearColor": clear_color,
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-set-label", payload) {
        ctx.pending.cancel(&request_id);
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let err_code = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(code) = err_code {
                return frontend_error_to_response(&code, value, "set_label");
            }
            Response::SetLabel
        }
        _ => {
            ctx.pending.cancel(&request_id);
            Response::Error {
                code: ErrorCode::FrontendUnresponsive,
                message: "frontend did not reply within 2s".into(),
                detail: None,
            }
        }
    }
}

async fn set_collapsed(ctx: &DispatchContext, p: PaneRef, minimized: bool) -> Response {
    let Some(app) = ctx.app.as_ref() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let request_id = format!("minimize-{}", uuid::Uuid::new_v4());
    let rx = ctx.pending.register(request_id.clone());
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "minimized": minimized,
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-minimize", payload) {
        ctx.pending.cancel(&request_id);
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let err_code = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(code) = err_code {
                return frontend_error_to_response(
                    &code,
                    value,
                    if minimized { "minimize" } else { "restore" },
                );
            }
            if minimized {
                Response::Minimize
            } else {
                Response::Restore
            }
        }
        _ => {
            ctx.pending.cancel(&request_id);
            Response::Error {
                code: ErrorCode::FrontendUnresponsive,
                message: "frontend did not reply within 2s".into(),
                detail: None,
            }
        }
    }
}

/// Validate a signal name received over the wire.
///
/// Mirrors the CLI's clap parser: 1–64 chars, `[a-zA-Z0-9_.-]` only.
/// Defense-in-depth so raw-JSON clients can't inject odd characters
/// into queue keys or output.
fn validate_signal_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("name must not be empty");
    }
    if name.len() > 64 {
        return Err("name must be 64 characters or fewer");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err("name may only contain [a-zA-Z0-9_.-]");
    }
    Ok(())
}

async fn signal_send(
    ctx: &DispatchContext,
    from: PaneRef,
    target: SignalTarget,
    name: String,
    data: Option<serde_json::Value>,
) -> Response {
    if let Err(reason) = validate_signal_name(&name) {
        return Response::Error {
            code: ErrorCode::InvalidArgument,
            message: reason.into(),
            detail: None,
        };
    }

    let Some(sender) = ctx.pane_map.resolve(&from) else {
        return pane_not_found(from);
    };

    let target_ids: Vec<String> = match target {
        SignalTarget::Pane(ref pr) => {
            let Some(entry) = ctx.pane_map.resolve(pr) else {
                return pane_not_found(pr.clone());
            };
            vec![entry.pane_id]
        }
        SignalTarget::Parent => match ctx.signals.parent_of(&sender.pane_id) {
            Some(p) => vec![p],
            None => Vec::new(),
        },
        SignalTarget::Children => ctx.signals.children_of(&sender.pane_id),
    };

    let timestamp = now_ms();
    let mut delivered: u32 = 0;
    for target_id in target_ids {
        let signal = Signal {
            name: name.clone(),
            data: data.clone(),
            sender_pane_id: sender.pane_id.clone(),
            sent_at_ms: timestamp,
        };
        ctx.signals.enqueue(&target_id, signal);
        delivered += 1;
    }
    Response::SignalSend { delivered }
}

async fn signal_wait(
    ctx: &DispatchContext,
    p: PaneRef,
    name: String,
    timeout_secs: u64,
) -> Response {
    if let Err(reason) = validate_signal_name(&name) {
        return Response::Error {
            code: ErrorCode::InvalidArgument,
            message: reason.into(),
            detail: None,
        };
    }

    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };

    // Clamp absurd timeouts before we hold the connection open. Raw
    // JSON clients can bypass the CLI's parser, so this is the last
    // line of defense.
    let secs = timeout_secs.min(MAX_SIGNAL_WAIT_SECS);
    let deadline = Duration::from_secs(secs);

    match ctx.signals.wait_for(&pane.pane_id, &name, deadline).await {
        Some(signal) => Response::SignalWait {
            name: signal.name,
            data: signal.data,
            sender_pane_id: signal.sender_pane_id,
            sent_at_ms: signal.sent_at_ms,
        },
        None => Response::Error {
            code: ErrorCode::Timeout,
            message: format!("no signal named '{name}' arrived within {secs}s"),
            detail: Some(serde_json::json!({ "timeout_secs": secs, "name": name })),
        },
    }
}

async fn signal_list(ctx: &DispatchContext, p: PaneRef) -> Response {
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let signals = ctx
        .signals
        .list(&pane.pane_id)
        .into_iter()
        .map(Into::into)
        .collect();
    Response::SignalList { signals }
}

/// Open (or focus) a kiri window for `dir`.
///
/// App-global: serviced by whichever window's socket the CLI relayed
/// through, reaching the shared `AppHandle`. Window creation must happen
/// on the main thread (`WebviewWindowBuilder::build`), so both the focus
/// and create paths hop onto it via `run_on_main_thread`.
async fn open_window(ctx: &DispatchContext, dir: String, force_new: bool) -> Response {
    use crate::commands::window::{create_window_impl, WindowRegistryState};
    use tauri::Manager;

    let Some(app) = ctx.app.clone() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };

    // Canonicalize and validate the target directory.
    let canonical = match std::fs::canonicalize(&dir) {
        Ok(p) => p,
        Err(e) => {
            return Response::Error {
                code: ErrorCode::InvalidArgument,
                message: format!("cannot open '{dir}': {e}"),
                detail: None,
            };
        }
    };
    if !canonical.is_dir() {
        return Response::Error {
            code: ErrorCode::InvalidArgument,
            message: format!("not a directory: {}", canonical.display()),
            detail: None,
        };
    }
    let project = canonical.to_string_lossy().to_string();

    // A window may already be open for this path. Reuse it unless the
    // caller forced a new one or the window has since been closed.
    let existing_label = {
        let registry = app.state::<WindowRegistryState>();
        let guard = match registry.lock() {
            Ok(g) => g,
            Err(_) => return internal("window registry poisoned"),
        };
        guard.get_label_for_path(&project).cloned()
    };
    let focus_label = match (force_new, existing_label) {
        (false, Some(label)) if app.get_webview_window(&label).is_some() => Some(label),
        _ => None,
    };

    let (label, created) = if let Some(label) = focus_label {
        let app_for_focus = app.clone();
        let label_for_focus = label.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(win) = app_for_focus.get_webview_window(&label_for_focus) {
                let _ = win.set_focus();
            }
        });
        (label, false)
    } else {
        // Ferry the generated label back out of the main-thread closure.
        let (tx, rx) = tokio::sync::oneshot::channel();
        let app_for_create = app.clone();
        let project_for_create = project.clone();
        if let Err(e) = app.run_on_main_thread(move || {
            let registry = app_for_create.state::<WindowRegistryState>();
            let res = create_window_impl(
                &app_for_create,
                Some(&registry),
                None,
                None,
                None,
                None,
                Some(project_for_create),
            );
            let _ = tx.send(res);
        }) {
            return internal(&format!("failed to schedule window creation: {e}"));
        }
        match rx.await {
            Ok(Ok(label)) => (label, true),
            Ok(Err(e)) => return internal(&format!("window creation failed: {e}")),
            Err(_) => return internal("window creation task was dropped"),
        }
    };

    let socket = crate::commands::cli_install::socket_path_for(&label)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let socket_ready = wait_for_cli_server(&app, &label, Duration::from_secs(10)).await;

    Response::OpenWindow {
        label,
        socket,
        project,
        created,
        socket_ready,
    }
}

/// Poll the shared CLI-server registry until `label`'s per-window server
/// is up, or `max` elapses. A new window boots its webview, calls
/// `register_window`, and that inserts its server into the registry —
/// that insertion is the readiness signal. Returns immediately true for a
/// window whose server is already running (the focus path).
async fn wait_for_cli_server(app: &tauri::AppHandle, label: &str, max: Duration) -> bool {
    use crate::commands::cli_server::CliServerRegistryState;
    use tauri::Manager;
    let Some(registry) = app.try_state::<CliServerRegistryState>() else {
        return false;
    };
    let deadline = tokio::time::Instant::now() + max;
    loop {
        if let Ok(map) = registry.handles.lock() {
            if map.contains_key(label) {
                return true;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Snapshot a pane's current on-screen text via the frontend's xterm
/// buffer (same round-trip pattern as `split`). `kind` is the cheap
/// foreground-process classification; `busy` is a best-effort heuristic
/// over the returned screen text.
async fn agent_status(ctx: &DispatchContext, p: PaneRef, lines: usize) -> Response {
    let Some(app) = ctx.app.as_ref() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };

    let (process_name, _mem, _running) = process_info_for(&ctx.terminals, pane.terminal_id);
    let kind = ai_kind_from_process_name(&process_name).unwrap_or_else(|| "none".into());

    let lines = lines.clamp(1, kiri_cli_proto::MAX_STATUS_LINES);

    let request_id = format!("snapshot-{}", uuid::Uuid::new_v4());
    let rx = ctx.pending.register(request_id.clone());
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "lines": lines,
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-snapshot", payload) {
        ctx.pending.cancel(&request_id);
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let err_code = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(code) = err_code {
                return frontend_error_to_response(&code, value, "snapshot");
            }
            let screen = value
                .get("screen")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let busy = screen_indicates_busy(&kind, &screen);
            Response::AgentStatus { kind, busy, screen }
        }
        _ => {
            ctx.pending.cancel(&request_id);
            Response::Error {
                code: ErrorCode::FrontendUnresponsive,
                message: "frontend did not reply within 2s".into(),
                detail: None,
            }
        }
    }
}

/// Best-effort "is the agent working" signal from the visible screen.
/// claude / codex render an "esc to interrupt" affordance while a turn is
/// in flight; its presence is a reliable-enough busy marker. Pure for
/// unit testing; never authoritative — callers read `screen` for truth.
fn screen_indicates_busy(kind: &str, screen: &str) -> bool {
    if kind == "none" {
        return false;
    }
    screen.to_lowercase().contains("esc to interrupt")
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

/// Process-wide monotonic counter so concurrent `Run` requests can never
/// collide on the same sentinel — even within the same nanosecond.
static NONCE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn rand_nonce() -> u64 {
    NONCE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::super::dispatch::{DispatchContext, TerminalBuffers};
    use super::super::frontend_bridge::PendingReplies;
    use super::super::pane_map::{PaneEntry, PaneMap};
    use super::super::signals::SignalRegistry;
    use super::*;
    use crate::commands::terminal::{
        TerminalManager, TerminalOutputBus, TerminalOutputBusState, TerminalState,
    };
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;

    fn make_ctx(pane_entries: Vec<PaneEntry>) -> (DispatchContext, TerminalOutputBusState) {
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
            signals: Arc::new(SignalRegistry::new()),
        };
        (ctx, bus)
    }

    fn pane_entry(index: u32, id: &str, terminal_id: u32, focused: bool) -> PaneEntry {
        PaneEntry {
            index,
            pane_id: id.into(),
            terminal_id,
            focused,
            name: None,
            color: None,
            collapsed: false,
        }
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

    #[test]
    fn ai_kind_classifies_known_agents() {
        assert_eq!(ai_kind_from_process_name("claude").as_deref(), Some("claude"));
        assert_eq!(ai_kind_from_process_name("codex").as_deref(), Some("codex"));
        // Case-insensitive, mirrors is_ai_process_for_shell_pid.
        assert_eq!(ai_kind_from_process_name("Claude").as_deref(), Some("claude"));
        assert!(ai_kind_from_process_name("zsh").is_none());
        assert!(ai_kind_from_process_name("node").is_none());
    }

    #[test]
    fn busy_heuristic_requires_ai_kind_and_interrupt_marker() {
        assert!(screen_indicates_busy(
            "claude",
            "✶ Thinking… (esc to interrupt)"
        ));
        // Marker present but pane is not an agent → not busy.
        assert!(!screen_indicates_busy("none", "esc to interrupt"));
        // Agent present but idle (no marker) → not busy.
        assert!(!screen_indicates_busy("codex", "> waiting for input"));
    }

    #[tokio::test]
    async fn whoami_without_app_handle_returns_label_and_no_project() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = whoami(&ctx).await;
        match resp {
            Response::WhoAmI {
                window_label,
                project_path,
            } => {
                assert_eq!(window_label, "test");
                assert!(project_path.is_none());
            }
            other => panic!("expected WhoAmI, got {other:?}"),
        }
    }

    #[test]
    fn window_registry_label_to_path_round_trip() {
        let mut reg = crate::commands::window::WindowRegistry::new();
        reg.register("window-7", "/Users/u/projects/kiri");
        assert_eq!(
            reg.get_path_for_label("window-7").map(String::as_str),
            Some("/Users/u/projects/kiri"),
        );
        assert!(reg.get_path_for_label("missing").is_none());
    }

    #[test]
    fn validate_pane_name_rules() {
        assert!(validate_pane_name("").is_err());
        assert!(validate_pane_name(&"a".repeat(33)).is_err());
        assert!(validate_pane_name("ab\nc").is_err());
        assert!(validate_pane_name("ab\0c").is_err());
        assert!(validate_pane_name("ab\x7fc").is_err());
        assert!(validate_pane_name("build").is_ok());
        assert!(validate_pane_name(&"a".repeat(32)).is_ok());
        assert!(validate_pane_name("ビルド").is_ok());
    }

    #[tokio::test]
    async fn split_rejects_oversize_name() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: None,
            color: None,
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = split(
            &ctx,
            PaneRef::focused(),
            SplitDirection::Horizontal,
            Some("a".repeat(33)),
            None,
            false,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn split_rejects_control_char_name() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: None,
            color: None,
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = split(
            &ctx,
            PaneRef::focused(),
            SplitDirection::Horizontal,
            Some("ab\nc".into()),
            None,
            false,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ls_returns_name_and_color_when_present() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: Some("agent".into()),
            color: Some(kiri_cli_proto::PaneColor::Iris),
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = ls(&ctx).await;
        match resp {
            Response::Ls { panes } => {
                assert_eq!(panes.len(), 1);
                assert_eq!(panes[0].name.as_deref(), Some("agent"));
                assert_eq!(panes[0].color, Some(kiri_cli_proto::PaneColor::Iris));
            }
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
            name: None,
            color: None,
            collapsed: false,
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
    async fn set_label_rejects_empty_update() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: None,
            color: None,
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = set_label(&ctx, PaneRef::focused(), None, false, None, false).await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_label_rejects_conflicting_name_flags() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: None,
            color: None,
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = set_label(
            &ctx,
            PaneRef::focused(),
            Some("build".into()),
            true,
            None,
            false,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_label_rejects_conflicting_color_flags() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: None,
            color: None,
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = set_label(
            &ctx,
            PaneRef::focused(),
            None,
            false,
            Some(kiri_cli_proto::PaneColor::Coral),
            true,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_label_rejects_bad_name() {
        let entry = PaneEntry {
            index: 0,
            pane_id: "p-0".into(),
            terminal_id: 1,
            focused: true,
            name: None,
            color: None,
            collapsed: false,
        };
        let (ctx, _bus) = make_ctx(vec![entry]);
        let resp = set_label(
            &ctx,
            PaneRef::focused(),
            Some("ab\nc".into()),
            false,
            None,
            false,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn set_label_returns_pane_not_found_for_unknown_pane() {
        // No app handle, but pane_not_found is checked before app dereference.
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = set_label(
            &ctx,
            PaneRef::Index(42),
            Some("agent".into()),
            false,
            None,
            false,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected PaneNotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn send_to_unknown_pane_returns_pane_not_found() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = send(&ctx, PaneRef::Index(99), "data".into(), false).await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    // --- signal_send / signal_wait / signal_list tests ---

    #[test]
    fn validate_signal_name_rules() {
        assert!(validate_signal_name("").is_err());
        assert!(validate_signal_name(&"a".repeat(65)).is_err());
        assert!(validate_signal_name("bad name").is_err());
        assert!(validate_signal_name("bad/name").is_err());
        assert!(validate_signal_name("ok").is_ok());
        assert!(validate_signal_name("build.done-1_step").is_ok());
        assert!(validate_signal_name(&"a".repeat(64)).is_ok());
    }

    #[tokio::test]
    async fn signal_send_to_specific_pane_enqueues_one() {
        let (ctx, _bus) = make_ctx(vec![
            pane_entry(0, "p-0", 1, true),
            pane_entry(1, "p-1", 2, false),
        ]);
        let resp = signal_send(
            &ctx,
            PaneRef::Id("p-0".into()),
            SignalTarget::Pane(PaneRef::Id("p-1".into())),
            "ready".into(),
            Some(serde_json::json!({"v": 1})),
        )
        .await;
        match resp {
            Response::SignalSend { delivered } => assert_eq!(delivered, 1),
            other => panic!("expected SignalSend, got {other:?}"),
        }
        let queued = ctx.signals.list("p-1");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].name, "ready");
        assert_eq!(queued[0].sender_pane_id, "p-0");
    }

    #[tokio::test]
    async fn signal_send_target_parent_uses_registered_parent() {
        let (ctx, _bus) = make_ctx(vec![
            pane_entry(0, "parent-0", 1, false),
            pane_entry(1, "child-1", 2, true),
        ]);
        ctx.signals
            .register_parent("parent-0".into(), "child-1".into());

        // Child sends to parent.
        let resp = signal_send(
            &ctx,
            PaneRef::Id("child-1".into()),
            SignalTarget::Parent,
            "done".into(),
            None,
        )
        .await;
        match resp {
            Response::SignalSend { delivered } => assert_eq!(delivered, 1),
            other => panic!("expected SignalSend, got {other:?}"),
        }
        let queued = ctx.signals.list("parent-0");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].sender_pane_id, "child-1");
    }

    #[tokio::test]
    async fn signal_send_target_parent_with_no_parent_delivers_zero() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "root", 1, true)]);
        let resp = signal_send(
            &ctx,
            PaneRef::Id("root".into()),
            SignalTarget::Parent,
            "orphan".into(),
            None,
        )
        .await;
        match resp {
            Response::SignalSend { delivered } => assert_eq!(delivered, 0),
            other => panic!("expected SignalSend, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_send_target_children_fans_out() {
        let (ctx, _bus) = make_ctx(vec![
            pane_entry(0, "p", 1, true),
            pane_entry(1, "c1", 2, false),
            pane_entry(2, "c2", 3, false),
        ]);
        ctx.signals.register_parent("p".into(), "c1".into());
        ctx.signals.register_parent("p".into(), "c2".into());

        let resp = signal_send(
            &ctx,
            PaneRef::Id("p".into()),
            SignalTarget::Children,
            "shutdown".into(),
            None,
        )
        .await;
        match resp {
            Response::SignalSend { delivered } => assert_eq!(delivered, 2),
            other => panic!("expected SignalSend, got {other:?}"),
        }
        assert_eq!(ctx.signals.list("c1").len(), 1);
        assert_eq!(ctx.signals.list("c2").len(), 1);
    }

    #[tokio::test]
    async fn signal_send_target_children_with_none_delivers_zero() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "lone", 1, true)]);
        let resp = signal_send(
            &ctx,
            PaneRef::Id("lone".into()),
            SignalTarget::Children,
            "broadcast".into(),
            None,
        )
        .await;
        match resp {
            Response::SignalSend { delivered } => assert_eq!(delivered, 0),
            other => panic!("expected SignalSend, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_send_with_unknown_sender_returns_pane_not_found() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = signal_send(
            &ctx,
            PaneRef::Id("nope".into()),
            SignalTarget::Parent,
            "x".into(),
            None,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected pane_not_found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_send_with_unknown_target_pane_returns_pane_not_found() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "p-0", 1, true)]);
        let resp = signal_send(
            &ctx,
            PaneRef::Id("p-0".into()),
            SignalTarget::Pane(PaneRef::Id("ghost".into())),
            "x".into(),
            None,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected pane_not_found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_send_rejects_invalid_name() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "p-0", 1, true)]);
        let resp = signal_send(
            &ctx,
            PaneRef::Id("p-0".into()),
            SignalTarget::Pane(PaneRef::Id("p-0".into())),
            "bad name".into(),
            None,
        )
        .await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_wait_returns_already_queued() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "p-0", 1, true)]);
        ctx.signals.enqueue(
            "p-0",
            Signal {
                name: "ready".into(),
                data: Some(serde_json::json!({"step": 7})),
                sender_pane_id: "other".into(),
                sent_at_ms: 42,
            },
        );
        let resp = signal_wait(&ctx, PaneRef::Id("p-0".into()), "ready".into(), 1).await;
        match resp {
            Response::SignalWait {
                name,
                data,
                sender_pane_id,
                sent_at_ms,
            } => {
                assert_eq!(name, "ready");
                assert_eq!(data, Some(serde_json::json!({"step": 7})));
                assert_eq!(sender_pane_id, "other");
                assert_eq!(sent_at_ms, 42);
            }
            other => panic!("expected SignalWait, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_wait_times_out_with_timeout_error() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "p-0", 1, true)]);
        let resp = signal_wait(&ctx, PaneRef::Id("p-0".into()), "never".into(), 1).await;
        match resp {
            Response::Error { code, message, .. } => {
                assert_eq!(code, ErrorCode::Timeout);
                assert!(message.contains("never"));
            }
            other => panic!("expected Timeout error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_wait_unknown_pane_returns_pane_not_found() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = signal_wait(&ctx, PaneRef::Id("ghost".into()), "ready".into(), 1).await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected pane_not_found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_wait_rejects_invalid_name() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "p-0", 1, true)]);
        let resp = signal_wait(&ctx, PaneRef::Id("p-0".into()), "bad/name".into(), 1).await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::InvalidArgument),
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_list_returns_queued_entries() {
        let (ctx, _bus) = make_ctx(vec![pane_entry(0, "p-0", 1, true)]);
        ctx.signals.enqueue(
            "p-0",
            Signal {
                name: "a".into(),
                data: None,
                sender_pane_id: "x".into(),
                sent_at_ms: 1,
            },
        );
        ctx.signals.enqueue(
            "p-0",
            Signal {
                name: "b".into(),
                data: Some(serde_json::json!(5)),
                sender_pane_id: "y".into(),
                sent_at_ms: 2,
            },
        );
        let resp = signal_list(&ctx, PaneRef::Id("p-0".into())).await;
        match resp {
            Response::SignalList { signals } => {
                assert_eq!(signals.len(), 2);
                assert_eq!(signals[0].name, "a");
                assert_eq!(signals[1].name, "b");
            }
            other => panic!("expected SignalList, got {other:?}"),
        }
        // List does not consume.
        assert_eq!(ctx.signals.list("p-0").len(), 2);
    }

    #[tokio::test]
    async fn signal_list_unknown_pane_returns_pane_not_found() {
        let (ctx, _bus) = make_ctx(vec![]);
        let resp = signal_list(&ctx, PaneRef::Id("ghost".into())).await;
        match resp {
            Response::Error { code, .. } => assert_eq!(code, ErrorCode::PaneNotFound),
            other => panic!("expected pane_not_found, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_send_then_wait_round_trip() {
        let (ctx, _bus) = make_ctx(vec![
            pane_entry(0, "parent", 1, true),
            pane_entry(1, "child", 2, false),
        ]);
        ctx.signals.register_parent("parent".into(), "child".into());

        // Spawn the wait first so we exercise the Notify path rather
        // than the fast-path pop.
        let ctx_for_wait = ctx.clone();
        let waiter = tokio::spawn(async move {
            signal_wait(&ctx_for_wait, PaneRef::Id("child".into()), "go".into(), 5).await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Parent sends to children.
        let send_resp = signal_send(
            &ctx,
            PaneRef::Id("parent".into()),
            SignalTarget::Children,
            "go".into(),
            None,
        )
        .await;
        assert!(matches!(send_resp, Response::SignalSend { delivered: 1 }));

        let resp = waiter.await.unwrap();
        match resp {
            Response::SignalWait {
                name,
                sender_pane_id,
                ..
            } => {
                assert_eq!(name, "go");
                assert_eq!(sender_pane_id, "parent");
            }
            other => panic!("expected SignalWait, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn signal_state_pruned_when_pane_disappears_from_map() {
        let (ctx, _bus) = make_ctx(vec![
            pane_entry(0, "p", 1, true),
            pane_entry(1, "c", 2, false),
        ]);
        ctx.signals.register_parent("p".into(), "c".into());
        ctx.signals.enqueue(
            "c",
            Signal {
                name: "x".into(),
                data: None,
                sender_pane_id: "p".into(),
                sent_at_ms: 0,
            },
        );
        // Child closes — pane map now only has the parent.
        ctx.pane_map.replace(vec![pane_entry(0, "p", 1, true)]);
        let known: std::collections::HashSet<String> = ["p".to_string()].into_iter().collect();
        ctx.signals.retain(&known);
        assert!(ctx.signals.list("c").is_empty());
        assert!(ctx.signals.parent_of("c").is_none());
    }
}
