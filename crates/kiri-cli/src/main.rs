mod cli;
mod render;
mod transport;

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::{Cli, TermCmd, Top};
use kiri_cli_proto::{Request, Response, SplitDirection};
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let exit_code = match run().await {
        Ok(code) => code,
        Err(e) => {
            // Always emit a JSON error on stderr so machine consumers
            // can parse it; humans will see the message too.
            let payload = serde_json::json!({
                "type": "error",
                "code": "internal_error",
                "message": format!("{e:#}"),
            });
            eprintln!("{payload}");
            1
        }
    };
    std::process::exit(exit_code);
}

async fn run() -> Result<i32> {
    let args = Cli::parse();
    let pretty = args.pretty;

    let req = match args.command {
        Top::Term(t) => build_request(t),
    };

    let socket = resolve_socket().await?;

    let responses = transport::send(&socket, &req).await?;
    let mut last_was_error = false;
    for resp in &responses {
        if pretty {
            render::render_response_pretty(resp);
        } else {
            println!("{}", serde_json::to_string(resp)?);
        }
        last_was_error = matches!(resp, Response::Error { .. });
    }
    Ok(if last_was_error { 1 } else { 0 })
}

/// Resolve the kiri socket to use, in priority order:
///
/// 1. `$KIRI_SOCKET` — if set AND the socket file exists AND a connection
///    succeeds. This is the happy path inside a live kiri terminal.
/// 2. Discovery — scan `~/.kiri/instances/*.sock` and pick the one that
///    accepts a connection. This rescues stale-env situations (the
///    original kiri instance died, leaving `KIRI_SOCKET` pointing at a
///    deleted file) and stale-file situations (the socket file remains
///    on disk but no listener is bound).
async fn resolve_socket() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("KIRI_SOCKET") {
        if !value.is_empty() {
            let p = PathBuf::from(&value);
            if p.exists() && socket_alive(&p).await {
                return Ok(p);
            }
            eprintln!(
                "warning: KIRI_SOCKET={} is stale — searching for an active kiri window",
                p.display()
            );
        }
    }

    let dir = dirs::home_dir()
        .map(|h| h.join(".kiri").join("instances"))
        .ok_or_else(|| anyhow!("no home directory"))?;
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(_) => {
            return Err(anyhow!(
                "no kiri windows are running (no {} directory)",
                dir.display()
            ))
        }
    };

    let mut alive = Vec::new();
    let mut stale = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sock") {
            continue;
        }
        if socket_alive(&path).await {
            alive.push(path);
        } else {
            stale.push(path);
        }
    }

    match alive.len() {
        0 if stale.is_empty() => Err(anyhow!(
            "no kiri windows are running — open a project window in kiri first"
        )),
        0 => Err(anyhow!(
            "found {} stale socket file(s) but no live kiri window: {:?}",
            stale.len(),
            stale
        )),
        1 => Ok(alive.into_iter().next().unwrap()),
        _ => Err(anyhow!(
            "multiple kiri windows are running — set KIRI_SOCKET explicitly to one of: {:?}",
            alive
        )),
    }
}

/// Cheap liveness probe: try to connect, drop the connection immediately.
async fn socket_alive(path: &std::path::Path) -> bool {
    use interprocess::local_socket::tokio::prelude::*;
    use interprocess::local_socket::{GenericFilePath, ToFsName};
    let Ok(name) = path.as_os_str().to_fs_name::<GenericFilePath>() else {
        return false;
    };
    interprocess::local_socket::tokio::Stream::connect(name)
        .await
        .is_ok()
}

fn build_request(cmd: TermCmd) -> Request {
    match cmd {
        TermCmd::Ls => Request::Ls,
        TermCmd::Run(a) => Request::Run {
            pane: cli::parse_pane(&a.pane),
            cmd: a.cmd.join(" "),
            timeout_secs: a.timeout,
            full_output: a.full,
        },
        TermCmd::Send(a) => Request::Send {
            pane: cli::parse_pane(&a.pane),
            data: a.data.join(" "),
        },
        TermCmd::Read(a) => Request::Read {
            pane: cli::parse_pane(&a.pane),
            since: a.since,
            tail: a.tail,
        },
        TermCmd::Follow(a) => Request::Follow {
            pane: cli::parse_pane(&a.pane),
        },
        TermCmd::Cancel(p) => Request::Cancel {
            pane: cli::parse_pane(&p),
        },
        TermCmd::Split(a) => Request::Split {
            pane: cli::parse_pane(&a.pane),
            direction: match a.dir.to_lowercase().as_str() {
                "v" | "vertical" => SplitDirection::Vertical,
                _ => SplitDirection::Horizontal,
            },
            minimized: false, // TODO(task-3): forward args.minimized once Task 2 adds the flag
        },
        TermCmd::Close(p) => Request::Close {
            pane: cli::parse_pane(&p),
        },
    }
}
