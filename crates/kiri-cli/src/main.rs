mod cli;
mod render;
mod transport;

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::{Cli, TermCmd, Top};
use kiri_cli_proto::{PaneColor, Request, Response, SplitDirection};
use std::path::{Path, PathBuf};

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
///    succeeds. Trusted as-is: inside a kiri terminal the env var points
///    to that terminal's own window.
/// 2. Discovery — scan `~/.kiri/instances/*.sock` for live sockets. When
///    the current working directory is inside a project, only sockets
///    whose window has that same project open are considered. This
///    prevents the CLI from silently opening panes in a different
///    project's window when `KIRI_SOCKET` is stale or unset.
async fn resolve_socket() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("KIRI_SOCKET") {
        if !value.is_empty() {
            let p = PathBuf::from(&value);
            if p.exists() && socket_alive(&p).await {
                return Ok(p);
            }
            eprintln!(
                "warning: KIRI_SOCKET={} is stale — searching for a kiri window for the current project",
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

    let mut alive: Vec<PathBuf> = Vec::new();
    let mut stale: Vec<PathBuf> = Vec::new();
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

    if alive.is_empty() {
        return if stale.is_empty() {
            Err(anyhow!(
                "no kiri windows are running — open a project window in kiri first"
            ))
        } else {
            Err(anyhow!(
                "found {} stale socket file(s) but no live kiri window: {:?}",
                stale.len(),
                stale
            ))
        };
    }

    let cwd = std::env::current_dir().ok();
    let cwd_project = cwd.as_deref().and_then(current_project_root);

    if let Some(project) = cwd_project.as_deref() {
        let mut matches: Vec<PathBuf> = Vec::new();
        for sock in &alive {
            if let Some(window_project) = query_window_project(sock).await {
                if is_same_or_within_project(&window_project, project) {
                    matches.push(sock.clone());
                }
            }
        }
        return match matches.len() {
            1 => Ok(matches.into_iter().next().unwrap()),
            0 => Err(anyhow!(
                "no kiri window is open for the current project ({}). Open it in kiri, or run from a directory inside the project you want to target.",
                project.display()
            )),
            _ => Err(anyhow!(
                "multiple kiri windows are open for the current project — set KIRI_SOCKET explicitly to one of: {:?}",
                matches
            )),
        };
    }

    // No project context (cwd is outside any git repo): preserve the
    // legacy behaviour of "use the only live window, or refuse if there
    // are several".
    match alive.len() {
        1 => Ok(alive.into_iter().next().unwrap()),
        _ => Err(anyhow!(
            "multiple kiri windows are running and no project context is available — \
             set KIRI_SOCKET explicitly to one of: {:?}",
            alive
        )),
    }
}

/// Walk up from `start` looking for a `.git` entry (directory or file).
/// Returns the directory that contains `.git`, i.e. the project root.
///
/// Worktree checkouts have `.git` as a file rather than a directory, but
/// either form marks the worktree's own root — that's what we want, so
/// the same parallel/worktree branch inside the project still resolves
/// to a path covered by the kiri window's project root via
/// [`is_same_or_within_project`].
pub(crate) fn current_project_root(start: &Path) -> Option<PathBuf> {
    let mut cur: &Path = start;
    loop {
        if cur.join(".git").exists() {
            return Some(cur.to_path_buf());
        }
        match cur.parent() {
            Some(p) if p != cur => cur = p,
            _ => return None,
        }
    }
}

/// Does `cwd_project` live inside (or equal) the window's `project`?
///
/// Used to allow a `kiri term split` from a worktree subdirectory of a
/// project to still resolve to the kiri window that has the project
/// open. Paths are canonicalised before comparison so symlinks don't
/// hide a match.
pub(crate) fn is_same_or_within_project(window_project: &Path, cwd_project: &Path) -> bool {
    let a = window_project
        .canonicalize()
        .unwrap_or_else(|_| window_project.to_path_buf());
    let b = cwd_project
        .canonicalize()
        .unwrap_or_else(|_| cwd_project.to_path_buf());
    b.starts_with(&a)
}

/// Ask the kiri server on `sock` which project is open. Returns `None`
/// on transport/protocol error or if the window has no project
/// registered. Errors are swallowed so one bad socket does not abort
/// discovery of the others.
async fn query_window_project(sock: &Path) -> Option<PathBuf> {
    let responses = transport::send(sock, &Request::WhoAmI).await.ok()?;
    for resp in responses {
        if let Response::WhoAmI { project_path, .. } = resp {
            return project_path.map(PathBuf::from);
        }
    }
    None
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
            name: a.name.clone(),
            color: a.color.map(PaneColor::from),
            minimized: a.minimized,
        },
        TermCmd::Close(p) => Request::Close {
            pane: cli::parse_pane(&p),
        },
        TermCmd::Minimize(opt) => Request::Minimize {
            pane: cli::parse_pane(&opt),
        },
        TermCmd::Restore(opt) => Request::Restore {
            pane: cli::parse_pane(&opt),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{current_project_root, is_same_or_within_project};
    use std::fs;
    use std::path::Path;

    #[test]
    fn current_project_root_finds_git_directory_in_self() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let found = current_project_root(tmp.path()).expect("project root");
        assert_eq!(found, tmp.path());
    }

    #[test]
    fn current_project_root_walks_up_to_parent() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let sub = tmp.path().join("a/b/c");
        fs::create_dir_all(&sub).unwrap();
        let found = current_project_root(&sub).expect("project root");
        assert_eq!(found, tmp.path());
    }

    #[test]
    fn current_project_root_returns_none_outside_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("no/git/here");
        fs::create_dir_all(&sub).unwrap();
        assert!(current_project_root(&sub).is_none());
    }

    #[test]
    fn current_project_root_handles_git_file_for_worktrees() {
        // git worktrees place a `.git` *file* (not directory) at the
        // worktree root pointing at the parent repo's gitdir.
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".git"), "gitdir: /elsewhere\n").unwrap();
        let found = current_project_root(tmp.path()).expect("project root");
        assert_eq!(found, tmp.path());
    }

    #[test]
    fn same_or_within_matches_equal_paths() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(is_same_or_within_project(tmp.path(), tmp.path()));
    }

    #[test]
    fn same_or_within_matches_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("src/x");
        fs::create_dir_all(&sub).unwrap();
        assert!(is_same_or_within_project(tmp.path(), &sub));
    }

    #[test]
    fn same_or_within_rejects_siblings() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        assert!(!is_same_or_within_project(&a, &b));
    }

    #[test]
    fn same_or_within_handles_nonexistent_paths() {
        // The CLI may have to compare against a window's project path
        // that has since been removed from disk; this must not panic.
        let phantom = Path::new("/this/path/does/not/exist/anywhere");
        assert!(is_same_or_within_project(phantom, phantom));
    }
}
