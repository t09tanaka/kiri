//! Newline-delimited JSON transport over a Unix Domain Socket / Named
//! Pipe (whichever `interprocess` picks for the platform).

use anyhow::{anyhow, Context, Result};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ToFsName};
use kiri_cli_proto::{Request, Response};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Send a single request and collect responses until either:
/// - A non-Follow request: one Response.
/// - A Follow request: stream until `Response::FollowEnd` or socket close.
pub async fn send(socket: &Path, req: &Request) -> Result<Vec<Response>> {
    let name = socket
        .as_os_str()
        .to_fs_name::<GenericFilePath>()
        .context("invalid socket path")?;
    let mut conn = interprocess::local_socket::tokio::Stream::connect(name)
        .await
        .with_context(|| format!("connect to kiri server at {}", socket.display()))?;

    let mut payload = serde_json::to_vec(req)?;
    payload.push(b'\n');
    conn.write_all(&payload).await?;

    let is_follow = matches!(req, Request::Follow { .. });
    let mut reader = BufReader::new(conn);
    let mut responses = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .context("read from kiri server")?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let resp: Response =
            serde_json::from_str(trimmed).with_context(|| format!("parse response: {trimmed}"))?;
        let stop = if is_follow {
            matches!(resp, Response::FollowEnd)
        } else {
            true
        };
        responses.push(resp);
        if stop {
            break;
        }
    }
    if responses.is_empty() {
        return Err(anyhow!("kiri server closed connection without responding"));
    }
    Ok(responses)
}
