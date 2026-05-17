//! Human-readable rendering for `--pretty` output.

use crate::EnvSnapshot;
use kiri_cli_proto::{PaneColor, PaneInfo, Response, SignalEntry};

pub fn render_response_pretty(resp: &Response) {
    match resp {
        Response::Ls { panes } => render_ls(panes),
        Response::Run {
            exit_code,
            output,
            output_truncated,
            lines_omitted,
            timed_out,
            ..
        } => {
            print!("{output}");
            if !output.ends_with('\n') {
                println!();
            }
            if *timed_out {
                eprintln!("(timed out — exit code unknown)");
            } else if let Some(c) = exit_code {
                eprintln!("(exit {c})");
            }
            if *output_truncated {
                eprintln!("(output truncated; {lines_omitted} earlier lines omitted — re-run with --full)");
            }
        }
        Response::Send { submitted } => {
            if *submitted {
                println!("ok (submitted)");
            } else {
                println!("ok");
            }
        }
        Response::Read {
            output,
            cursor,
            bytes_dropped,
        } => {
            print!("{output}");
            if !output.ends_with('\n') {
                println!();
            }
            eprintln!("(cursor {cursor})");
            if *bytes_dropped > 0 {
                eprintln!(
                    "(warning: {bytes_dropped} bytes dropped from buffer before requested cursor)"
                );
            }
        }
        Response::FollowChunk { data, .. } => {
            print!("{data}");
        }
        Response::FollowEnd => {}
        Response::Cancel => println!("ok"),
        Response::Split {
            new_pane_id,
            new_pane_index,
        } => {
            println!("created pane {new_pane_index} ({new_pane_id})");
        }
        Response::Close => println!("ok"),
        Response::Minimize => println!("ok"),
        Response::Restore => println!("ok"),
        Response::SetLabel => println!("ok"),
        Response::WhoAmI {
            window_label,
            project_path,
        } => {
            let project = project_path.as_deref().unwrap_or("(no project)");
            println!("window {window_label} — {project}");
        }
        Response::SignalSend { delivered } => println!("delivered to {delivered} pane(s)"),
        Response::SignalWait {
            name,
            data,
            sender_pane_id,
            sent_at_ms,
        } => {
            println!("received '{name}' from {sender_pane_id} (sent at {sent_at_ms} ms)");
            if let Some(v) = data {
                println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
            }
        }
        Response::SignalList { signals } => render_signal_list(signals),
        Response::Error { code, message, .. } => {
            eprintln!("error [{code:?}]: {message}");
        }
    }
}

pub fn render_env_pretty(snap: &EnvSnapshot) {
    println!(
        "KIRI_TERMINAL  {}",
        snap.kiri_terminal.as_deref().unwrap_or("(unset)")
    );
    println!(
        "KIRI_SOCKET    {}",
        snap.kiri_socket.as_deref().unwrap_or("(unset)")
    );
    println!(
        "  in kiri terminal:   {}",
        if snap.in_kiri_terminal { "yes" } else { "no" }
    );
    println!(
        "  socket alive:       {}",
        if snap.configured_socket_alive {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "  cwd project:        {}",
        snap.cwd_project.as_deref().unwrap_or("(none)")
    );
    println!(
        "  instances dir:      {}",
        snap.instances_dir.as_deref().unwrap_or("(unknown)")
    );
    println!(
        "  resolved socket:    {}",
        snap.resolved_socket.as_deref().unwrap_or("(none)")
    );
    println!("  resolution:         {}", snap.resolution);
    if snap.discovered_windows.is_empty() {
        println!("  discovered windows: (none)");
    } else {
        println!("  discovered windows:");
        for w in &snap.discovered_windows {
            let project = w.project.as_deref().unwrap_or("(no project)");
            println!("    {} — {project}", w.socket);
        }
    }
}

fn render_signal_list(signals: &[SignalEntry]) {
    if signals.is_empty() {
        println!("(no signals queued)");
        return;
    }
    for s in signals {
        let data = s
            .data
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{} from={} at={}ms data={}",
            s.name, s.sender_pane_id, s.sent_at_ms, data
        );
    }
}

fn render_ls(panes: &[PaneInfo]) {
    if panes.is_empty() {
        println!("(no terminal panes in this window)");
        return;
    }
    println!(
        "{:<5} {:<14} {:<10} {:<6} {:<32} {:<16} {:<7} MEM",
        "INDEX", "ID", "NAME", "COLOR", "CWD", "PROCESS", "RUNNING"
    );
    for p in panes {
        let focused = if p.focused { " (focused)" } else { "" };
        let name = p.name.as_deref().unwrap_or("-");
        let color = p.color.as_ref().map(color_label).unwrap_or("-");
        println!(
            "{:<5} {:<14} {:<10} {:<6} {:<32} {:<16} {:<7} {}{}",
            p.index,
            p.id,
            name,
            color,
            p.cwd.clone().unwrap_or_default(),
            p.process_name,
            if p.running { "yes" } else { "no" },
            human_bytes(p.memory_bytes),
            focused
        );
    }
}

fn color_label(c: &PaneColor) -> &'static str {
    match c {
        PaneColor::Sky => "sky",
        PaneColor::Iris => "iris",
        PaneColor::Jade => "jade",
        PaneColor::Amber => "amber",
        PaneColor::Coral => "coral",
        PaneColor::Rose => "rose",
    }
}

fn human_bytes(b: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{:.0}{}", v, UNITS[i])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_thresholds() {
        assert_eq!(human_bytes(0), "0B");
        assert_eq!(human_bytes(1023), "1023B");
        assert_eq!(human_bytes(1024), "1KB");
        assert_eq!(human_bytes(1024 * 1024), "1MB");
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1GB");
    }

    #[test]
    fn color_label_covers_all_variants() {
        assert_eq!(color_label(&PaneColor::Sky), "sky");
        assert_eq!(color_label(&PaneColor::Iris), "iris");
        assert_eq!(color_label(&PaneColor::Jade), "jade");
        assert_eq!(color_label(&PaneColor::Amber), "amber");
        assert_eq!(color_label(&PaneColor::Coral), "coral");
        assert_eq!(color_label(&PaneColor::Rose), "rose");
    }
}
