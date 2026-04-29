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

    let socket = locate_socket()?;

    let req = match args.command {
        Top::Term(t) => build_request(t),
    };

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

/// Locate the per-window socket via the env var injected by the kiri PTY.
fn locate_socket() -> Result<PathBuf> {
    let value = std::env::var("KIRI_SOCKET").map_err(|_| {
        anyhow!("KIRI_SOCKET not set — this command must be run from inside a kiri terminal")
    })?;
    if value.is_empty() {
        return Err(anyhow!("KIRI_SOCKET is set but empty"));
    }
    Ok(PathBuf::from(value))
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
        },
        TermCmd::Close(p) => Request::Close {
            pane: cli::parse_pane(&p),
        },
    }
}
