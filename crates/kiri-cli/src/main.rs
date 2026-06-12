mod cli;
mod render;
mod transport;

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::{Cli, SignalCmd, SignalTargetArg, TermCmd, Top, WindowCmd};
use kiri_cli_proto::{PaneColor, Request, Response, SignalTarget, SplitDirection};
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

    // `env` does not need a live kiri socket, so handle it before
    // resolve_socket() — that lets users run it outside a kiri
    // terminal or when KIRI_SOCKET is stale.
    if matches!(&args.command, Top::Env) {
        let snapshot = collect_env_snapshot().await;
        if pretty {
            render::render_env_pretty(&snapshot);
        } else {
            println!("{}", serde_json::to_string(&snapshot)?);
        }
        return Ok(0);
    }

    // `signal wait --print-data` decodes the payload only on the success
    // path; capture it now so we can branch on it after the response
    // arrives.
    let print_signal_data = matches!(
        &args.command,
        Top::Term(TermCmd::Signal(SignalCmd::Wait(w))) if w.print_data
    );

    let window_selector = args.window.clone();

    // Resolve the request and the socket together: `window` is an
    // app-global command (relay through any live window), `term` honors
    // the `--window` selector when present, otherwise the normal
    // project-scoped resolution.
    let (req, socket) = match args.command {
        Top::Env => unreachable!("env is handled above"),
        Top::Window(w) => {
            let req = build_window_request(w);
            let socket = resolve_any_socket().await?;
            (req, socket)
        }
        Top::Term(t) => {
            let req = build_request(t)?;
            let socket = match window_selector.as_deref() {
                Some(sel) => resolve_socket_for_window(sel).await?,
                None => resolve_socket().await?,
            };
            (req, socket)
        }
    };

    let responses = transport::send(&socket, &req).await?;
    let mut last_was_error = false;
    for resp in &responses {
        if pretty {
            render::render_response_pretty(resp);
        } else {
            println!("{}", serde_json::to_string(resp)?);
        }
        if print_signal_data {
            if let Response::SignalWait { data: Some(v), .. } = resp {
                println!("{}", serde_json::to_string(v)?);
            }
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

/// Snapshot of the kiri environment as seen by the CLI.
///
/// Returned by `kiri env`. Captures the env vars the CLI relies on, the
/// state of the configured socket, and every kiri window the CLI can
/// see, so callers can debug "why can't `kiri term split` find my
/// window" without running a privileged subcommand.
#[derive(Debug, serde::Serialize)]
pub struct EnvSnapshot {
    /// `KIRI_SOCKET` env var as set by the parent process, or `None`
    /// when unset. Inside a kiri terminal this points at the parent
    /// window's UDS.
    pub kiri_socket: Option<String>,
    /// `KIRI_TERMINAL` env var. The kiri host sets this to `"1"` in
    /// every PTY it spawns, so it is the cheapest "am I running inside
    /// kiri?" check.
    pub kiri_terminal: Option<String>,
    /// True when this process is running inside a kiri-spawned shell
    /// (`KIRI_TERMINAL=1`). The host sets this on every PTY, so the
    /// check is reliable even when `KIRI_SOCKET` is missing.
    pub in_kiri_terminal: bool,
    /// True when `KIRI_SOCKET` points at a file that exists on disk
    /// **and** accepts a connection. Stale sockets (file present but
    /// no listener) appear as `false` here.
    pub configured_socket_alive: bool,
    /// `~/.kiri/instances/` if the home directory can be resolved.
    pub instances_dir: Option<String>,
    /// All live kiri windows found by scanning `instances_dir`. Each
    /// entry includes the absolute socket path and the project the
    /// window has open, when the host reports one.
    pub discovered_windows: Vec<DiscoveredWindow>,
    /// Project root inferred from the current working directory by
    /// walking up looking for `.git`. `None` outside any repo.
    pub cwd_project: Option<String>,
    /// Path of the socket that `kiri term` would target right now,
    /// computed with the same priority order as `resolve_socket`. Null
    /// when no usable target exists.
    pub resolved_socket: Option<String>,
    /// Human-readable explanation of how `resolved_socket` was chosen
    /// (or why it could not be). Stable enough to grep in scripts but
    /// not part of the JSON schema contract.
    pub resolution: String,
}

/// One row in `EnvSnapshot::discovered_windows`.
#[derive(Debug, serde::Serialize)]
pub struct DiscoveredWindow {
    pub socket: String,
    pub project: Option<String>,
}

/// Build an [`EnvSnapshot`] without sending any state-changing
/// requests. Probes are best-effort: a failure to read a directory or
/// query a window leaves the corresponding field empty rather than
/// aborting.
pub async fn collect_env_snapshot() -> EnvSnapshot {
    let kiri_socket = std::env::var("KIRI_SOCKET").ok().filter(|s| !s.is_empty());
    let kiri_terminal = std::env::var("KIRI_TERMINAL")
        .ok()
        .filter(|s| !s.is_empty());
    let in_kiri_terminal = kiri_terminal.as_deref() == Some("1");

    let configured_socket_alive = match kiri_socket.as_deref() {
        Some(p) => {
            let path = PathBuf::from(p);
            path.exists() && socket_alive(&path).await
        }
        None => false,
    };

    let instances_dir = dirs::home_dir().map(|h| h.join(".kiri").join("instances"));
    let instances_dir_str = instances_dir.as_ref().map(|p| p.display().to_string());

    let mut discovered: Vec<DiscoveredWindow> = Vec::new();
    if let Some(dir) = instances_dir.as_ref() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("sock") {
                    continue;
                }
                if !socket_alive(&path).await {
                    continue;
                }
                let project = query_window_project(&path).await;
                discovered.push(DiscoveredWindow {
                    socket: path.display().to_string(),
                    project: project.map(|p| p.display().to_string()),
                });
            }
        }
    }

    let cwd = std::env::current_dir().ok();
    let cwd_project = cwd.as_deref().and_then(current_project_root);

    let (resolved_socket, resolution) = match resolve_socket().await {
        Ok(p) => (Some(p.display().to_string()), "resolved".to_string()),
        Err(e) => (None, format!("{e:#}")),
    };

    EnvSnapshot {
        kiri_socket,
        kiri_terminal,
        in_kiri_terminal,
        configured_socket_alive,
        instances_dir: instances_dir_str,
        discovered_windows: discovered,
        cwd_project: cwd_project.map(|p| p.display().to_string()),
        resolved_socket,
        resolution,
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

fn build_request(cmd: TermCmd) -> Result<Request> {
    Ok(match cmd {
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
            submit: !a.no_submit,
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
            name: Some(a.name.clone()),
            color: Some(PaneColor::from(a.color)),
            // New panes default to minimized; `--no-minimized` opts out.
            minimized: !a.no_minimized,
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
        TermCmd::SetLabel(a) => {
            if a.is_empty_update() {
                return Err(anyhow!(
                    "set-label requires at least one of --name, --clear-name, --color, --clear-color"
                ));
            }
            Request::SetLabel {
                pane: cli::parse_pane(&a.pane),
                set_name: a.name.clone(),
                clear_name: a.clear_name,
                set_color: a.color.map(PaneColor::from),
                clear_color: a.clear_color,
            }
        }
        TermCmd::Status(a) => Request::AgentStatus {
            pane: cli::parse_pane(&a.pane),
            lines: a.lines,
        },
        TermCmd::Signal(s) => match s {
            SignalCmd::Send(a) => Request::SignalSend {
                from: cli::parse_pane_string(&a.from),
                target: match a.target {
                    Some(SignalTargetArg::Parent) => SignalTarget::Parent,
                    Some(SignalTargetArg::Children) => SignalTarget::Children,
                    None => SignalTarget::Pane(cli::parse_pane_string(&a.pane)),
                },
                name: a.name,
                data: a.data,
            },
            SignalCmd::Wait(a) => Request::SignalWait {
                pane: cli::parse_pane_string(&a.pane),
                name: a.name,
                timeout_secs: a.timeout,
            },
            SignalCmd::List(a) => Request::SignalList {
                pane: cli::parse_pane_string(&a.pane),
            },
        },
    })
}

fn build_window_request(cmd: WindowCmd) -> Request {
    match cmd {
        WindowCmd::Open(a) => Request::OpenWindow {
            dir: absolutize(&a.dir),
            force_new: a.new,
        },
    }
}

/// Resolve `dir` to an absolute path relative to the CLI's current
/// directory. The kiri host canonicalizes again, but it runs with a
/// different cwd than the user's shell, so a relative path must be made
/// absolute here or it would resolve against the host process.
fn absolutize(dir: &str) -> String {
    let p = Path::new(dir);
    if p.is_absolute() {
        return dir.to_string();
    }
    std::env::current_dir()
        .map(|c| c.join(dir).to_string_lossy().into_owned())
        .unwrap_or_else(|_| dir.to_string())
}

/// All live per-window sockets under `~/.kiri/instances`, with no project
/// filtering. Returns an empty vec (not an error) when the directory is
/// missing so callers can produce their own "no windows" message.
async fn live_sockets() -> Result<Vec<PathBuf>> {
    let dir = dirs::home_dir()
        .map(|h| h.join(".kiri").join("instances"))
        .ok_or_else(|| anyhow!("no home directory"))?;
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(_) => return Ok(Vec::new()),
    };
    let mut alive = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sock") {
            continue;
        }
        if socket_alive(&path).await {
            alive.push(path);
        }
    }
    Ok(alive)
}

/// Resolve a socket for an app-global request (`window open`). The
/// request reaches the shared `AppHandle`, so any live window will do —
/// prefer `$KIRI_SOCKET`, fall back to any discovered live window.
async fn resolve_any_socket() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("KIRI_SOCKET") {
        if !value.is_empty() {
            let p = PathBuf::from(&value);
            if p.exists() && socket_alive(&p).await {
                return Ok(p);
            }
        }
    }
    let alive = live_sockets().await?;
    alive
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("no kiri windows are running — open a kiri window first"))
}

/// Resolve a socket from a `--window <project>` selector, bypassing the
/// normal project guard. Matches a window by its open project path or by
/// that path's basename. Ambiguous matches are an error.
async fn resolve_socket_for_window(selector: &str) -> Result<PathBuf> {
    let alive = live_sockets().await?;
    if alive.is_empty() {
        return Err(anyhow!(
            "no kiri windows are running — open a kiri window first"
        ));
    }
    // If the selector is itself an existing path, canonicalize it once so
    // we can match it against the window's canonical project path.
    let canonical_selector = std::fs::canonicalize(selector)
        .ok()
        .map(|p| p.to_string_lossy().into_owned());

    let mut matches: Vec<(PathBuf, String)> = Vec::new();
    for sock in &alive {
        if let Some((label, project)) = query_window_identity(sock).await {
            if window_matches(selector, canonical_selector.as_deref(), project.as_deref()) {
                matches.push((sock.clone(), label));
            }
        }
    }
    match matches.len() {
        1 => Ok(matches.into_iter().next().unwrap().0),
        0 => Err(anyhow!("no kiri window is open for '{selector}'")),
        _ => {
            let labels: Vec<String> = matches.iter().map(|(_, l)| l.clone()).collect();
            let socks: Vec<String> = matches
                .iter()
                .map(|(s, _)| s.display().to_string())
                .collect();
            Err(anyhow!(
                "multiple kiri windows match '{selector}' ({}). Set KIRI_SOCKET explicitly to one of: {:?}",
                labels.join(", "),
                socks
            ))
        }
    }
}

/// Does a window with open project `project` match the `--window`
/// selector? True when the project path equals the raw selector, equals
/// its canonicalized form, or its basename equals the selector. Pure so
/// it can be unit-tested without a live socket.
fn window_matches(selector: &str, canonical_selector: Option<&str>, project: Option<&str>) -> bool {
    let Some(project) = project else {
        return false;
    };
    if project == selector {
        return true;
    }
    if canonical_selector == Some(project) {
        return true;
    }
    Path::new(project)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|base| base == selector)
        .unwrap_or(false)
}

/// Ask the window on `sock` for its label and open project. Errors are
/// swallowed (returns `None`) so one unresponsive socket does not abort
/// the scan of the others.
async fn query_window_identity(sock: &Path) -> Option<(String, Option<String>)> {
    let responses = transport::send(sock, &Request::WhoAmI).await.ok()?;
    for resp in responses {
        if let Response::WhoAmI {
            window_label,
            project_path,
        } = resp
        {
            return Some((window_label, project_path));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::fs;
    use std::path::Path;

    fn parse_term(args: &[&str]) -> TermCmd {
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Top::Term(t) => t,
            other => panic!("expected Top::Term, got {other:?}"),
        }
    }

    #[test]
    fn build_request_status_maps_pane_and_lines() {
        let req = build_request(parse_term(&[
            "kiri", "term", "status", "--pane", "2", "--lines", "12",
        ]))
        .expect("should build status request");
        assert_eq!(
            req,
            Request::AgentStatus {
                pane: kiri_cli_proto::PaneRef::Index(2),
                lines: 12,
            }
        );
    }

    #[test]
    fn build_window_request_open_absolutizes_and_passes_new() {
        let cli = Cli::try_parse_from(["kiri", "window", "open", "--dir", "/abs/proj", "--new"])
            .unwrap();
        let Top::Window(w) = cli.command else {
            panic!("expected window");
        };
        assert_eq!(
            build_window_request(w),
            Request::OpenWindow {
                dir: "/abs/proj".into(),
                force_new: true,
            }
        );
    }

    #[test]
    fn absolutize_keeps_absolute_paths() {
        assert_eq!(absolutize("/already/abs"), "/already/abs");
    }

    #[test]
    fn absolutize_joins_relative_to_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let expected = cwd.join("sub/dir").to_string_lossy().into_owned();
        assert_eq!(absolutize("sub/dir"), expected);
    }

    #[test]
    fn window_matches_exact_path() {
        assert!(window_matches("/home/me/proj", None, Some("/home/me/proj")));
    }

    #[test]
    fn window_matches_basename() {
        assert!(window_matches("proj", None, Some("/home/me/proj")));
    }

    #[test]
    fn window_matches_canonical_selector() {
        assert!(window_matches(
            "./proj",
            Some("/home/me/proj"),
            Some("/home/me/proj")
        ));
    }

    #[test]
    fn window_matches_rejects_unrelated() {
        assert!(!window_matches("other", None, Some("/home/me/proj")));
    }

    #[test]
    fn window_matches_rejects_window_without_project() {
        assert!(!window_matches("proj", None, None));
    }

    #[test]
    fn build_request_set_label_with_name_and_color() {
        let req = build_request(parse_term(&[
            "kiri",
            "term",
            "set-label",
            "--pane",
            "2",
            "--name",
            "build",
            "--color",
            "coral",
        ]))
        .expect("should build set_label request");
        assert_eq!(
            req,
            Request::SetLabel {
                pane: kiri_cli_proto::PaneRef::Index(2),
                set_name: Some("build".into()),
                clear_name: false,
                set_color: Some(PaneColor::Coral),
                clear_color: false,
            }
        );
    }

    #[test]
    fn build_request_set_label_clear_only() {
        let req = build_request(parse_term(&[
            "kiri",
            "term",
            "set-label",
            "--clear-name",
            "--clear-color",
        ]))
        .expect("should build set_label request");
        assert_eq!(
            req,
            Request::SetLabel {
                pane: kiri_cli_proto::PaneRef::focused(),
                set_name: None,
                clear_name: true,
                set_color: None,
                clear_color: true,
            }
        );
    }

    #[test]
    fn build_request_set_label_rejects_empty_update() {
        let err = build_request(parse_term(&["kiri", "term", "set-label"]));
        assert!(err.is_err(), "empty set-label should be rejected");
    }

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

    #[test]
    fn env_top_parses_with_no_args() {
        // `kiri env` is the entry point used outside a kiri terminal,
        // so it must parse without any subcommand or flag.
        let cli = Cli::try_parse_from(["kiri", "env"]).unwrap();
        assert!(matches!(cli.command, Top::Env));
    }

    #[test]
    fn env_top_supports_pretty_flag() {
        // The global --pretty flag must apply to `env` the same way it
        // applies to `term` subcommands.
        let cli = Cli::try_parse_from(["kiri", "--pretty", "env"]).unwrap();
        assert!(cli.pretty);
        assert!(matches!(cli.command, Top::Env));
    }

    #[tokio::test]
    async fn env_snapshot_serialises_outside_a_kiri_terminal() {
        // CI runs without KIRI_SOCKET / KIRI_TERMINAL set; the snapshot
        // must still serialise to valid JSON so machine callers can
        // detect "not in a kiri terminal" from a stable schema.
        //
        // Side note: collect_env_snapshot reads process-level env vars
        // and the home directory. The CI sandbox makes both predictable
        // enough for a smoke test; we do not assert specific values
        // beyond "fields are present and JSON is valid".
        let snap = collect_env_snapshot().await;
        let json = serde_json::to_value(&snap).expect("serialises");
        assert!(json.get("kiri_socket").is_some());
        assert!(json.get("in_kiri_terminal").is_some());
        assert!(json.get("discovered_windows").is_some());
        assert!(json.get("resolution").is_some());
    }
}
