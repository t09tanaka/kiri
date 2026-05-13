//! Human-readable rendering for `--pretty` output.

use kiri_cli_proto::{PaneInfo, Response};

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
        Response::Send => println!("ok"),
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
                eprintln!("(warning: {bytes_dropped} bytes dropped from buffer before requested cursor)");
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
        Response::Error { code, message, .. } => {
            eprintln!("error [{code:?}]: {message}");
        }
    }
}

fn render_ls(panes: &[PaneInfo]) {
    if panes.is_empty() {
        println!("(no terminal panes in this window)");
        return;
    }
    println!(
        "{:<5} {:<14} {:<32} {:<16} {:<7} MEM",
        "INDEX", "ID", "CWD", "PROCESS", "RUNNING"
    );
    for p in panes {
        let focused = if p.focused { " (focused)" } else { "" };
        println!(
            "{:<5} {:<14} {:<32} {:<16} {:<7} {}{}",
            p.index,
            p.id,
            p.cwd.clone().unwrap_or_default(),
            p.process_name,
            if p.running { "yes" } else { "no" },
            human_bytes(p.memory_bytes),
            focused
        );
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
}
