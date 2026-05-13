//! clap definitions for the `kiri` CLI binary.
//!
//! The binary lives at `~/.kiri/bin/kiri` (installed by the kiri app at
//! startup) and is automatically on PATH inside kiri terminals via
//! `KIRI_SOCKET`-driven env injection. Outside of a kiri terminal the
//! command will report an error because `KIRI_SOCKET` is not set.

use clap::{Args, Parser, Subcommand};
use kiri_cli_proto::PaneRef;

#[derive(Parser, Debug)]
#[command(name = "kiri", version, about = "kiri CLI", long_about = None)]
pub struct Cli {
    /// Render output in a human-readable format instead of JSON.
    #[arg(long, global = true)]
    pub pretty: bool,

    #[command(subcommand)]
    pub command: Top,
}

#[derive(Subcommand, Debug)]
pub enum Top {
    /// Operate on terminal panes inside this kiri window.
    #[command(subcommand)]
    Term(TermCmd),
}

#[derive(Args, Debug, Clone)]
pub struct PaneOpt {
    /// Pane index (0,1,2,…) or pane id. Omit to use the focused pane.
    #[arg(long, value_name = "I_OR_ID")]
    pub pane: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum TermCmd {
    /// List the panes in this window.
    Ls,
    /// Run a command in a pane and wait for completion.
    Run(RunArgs),
    /// Send raw bytes to a pane (use $'...' for escapes).
    Send(SendArgs),
    /// Read recent output from a pane's ring buffer.
    Read(ReadArgs),
    /// Stream output from a pane until the connection closes.
    Follow(FollowArgs),
    /// Send Ctrl-C (SIGINT) to the foreground process.
    Cancel(PaneOpt),
    /// Split the pane horizontally (default) or vertically.
    Split(SplitArgs),
    /// Close the pane.
    Close(PaneOpt),
    /// Collapse the shortcut bar to a thin strip with only the restore + settings buttons.
    Minimize(PaneOpt),
    /// Expand a minimized shortcut bar back to its full layout.
    Restore(PaneOpt),
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    /// Per-request timeout in seconds. Default 300 (= 5 minutes).
    #[arg(long, default_value_t = 300)]
    pub timeout: u64,
    /// Return the full captured output instead of the last 1000 lines.
    #[arg(long)]
    pub full: bool,
    /// Command string. Multiple positional words are joined with a space.
    #[arg(num_args = 1.., required = true)]
    pub cmd: Vec<String>,
}

#[derive(Args, Debug)]
pub struct SendArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    /// Bytes to send. Multiple positional words are joined with a space.
    #[arg(num_args = 1.., required = true)]
    pub data: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ReadArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    /// Cursor (returned by a previous read). Returns bytes since cursor.
    #[arg(long)]
    pub since: Option<u64>,
    /// Return only the last N lines (mutually exclusive with --since).
    #[arg(long)]
    pub tail: Option<usize>,
}

#[derive(Args, Debug)]
pub struct FollowArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
}

#[derive(Args, Debug)]
pub struct SplitArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    /// Split direction: h (horizontal) or v (vertical).
    #[arg(long, default_value = "h")]
    pub dir: String,
    /// Create the new pane with its shortcut bar already minimized.
    #[arg(long)]
    pub minimized: bool,
}

/// Translate a `--pane` flag into a `PaneRef`.
///
/// Empty / missing → focused. Numeric → Index. Anything else → Id.
pub fn parse_pane(opt: &PaneOpt) -> PaneRef {
    match &opt.pane {
        None => PaneRef::Id(PaneRef::FOCUSED_SENTINEL.to_string()),
        Some(s) => match s.parse::<u32>() {
            Ok(i) => PaneRef::Index(i),
            Err(_) => PaneRef::Id(s.clone()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pane_focused_default() {
        let r = parse_pane(&PaneOpt { pane: None });
        assert_eq!(r, PaneRef::Id("focused".to_string()));
    }

    #[test]
    fn parse_pane_index() {
        let r = parse_pane(&PaneOpt {
            pane: Some("3".into()),
        });
        assert_eq!(r, PaneRef::Index(3));
    }

    #[test]
    fn parse_pane_id() {
        let r = parse_pane(&PaneOpt {
            pane: Some("abc".into()),
        });
        assert_eq!(r, PaneRef::Id("abc".into()));
    }

    #[test]
    fn parse_minimize_subcommand() {
        let cli = Cli::try_parse_from(["kiri", "term", "minimize"]).unwrap();
        match cli.command {
            Top::Term(TermCmd::Minimize(opt)) => {
                assert_eq!(parse_pane(&opt), PaneRef::focused());
            }
            _ => panic!("expected minimize"),
        }
    }

    #[test]
    fn parse_restore_subcommand_with_pane() {
        let cli = Cli::try_parse_from(["kiri", "term", "restore", "--pane", "pane-2"]).unwrap();
        match cli.command {
            Top::Term(TermCmd::Restore(opt)) => {
                assert_eq!(parse_pane(&opt), PaneRef::Id("pane-2".into()));
            }
            _ => panic!("expected restore"),
        }
    }

    #[test]
    fn parse_split_minimized_flag() {
        let cli = Cli::try_parse_from(["kiri", "term", "split", "--minimized"]).unwrap();
        match cli.command {
            Top::Term(TermCmd::Split(args)) => assert!(args.minimized),
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn parse_split_default_minimized_false() {
        let cli = Cli::try_parse_from(["kiri", "term", "split"]).unwrap();
        match cli.command {
            Top::Term(TermCmd::Split(args)) => assert!(!args.minimized),
            _ => panic!("expected split"),
        }
    }
}
