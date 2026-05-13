//! clap definitions for the `kiri` CLI binary.
//!
//! The binary lives at `~/.kiri/bin/kiri` (installed by the kiri app at
//! startup) and is automatically on PATH inside kiri terminals via
//! `KIRI_SOCKET`-driven env injection. Outside of a kiri terminal the
//! command will report an error because `KIRI_SOCKET` is not set.

use clap::{Args, Parser, Subcommand, ValueEnum};
use kiri_cli_proto::{PaneColor, PaneRef};

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum PaneColorArg {
    Sky,
    Iris,
    Jade,
    Amber,
    Coral,
    Rose,
}

impl From<PaneColorArg> for PaneColor {
    fn from(a: PaneColorArg) -> Self {
        match a {
            PaneColorArg::Sky => PaneColor::Sky,
            PaneColorArg::Iris => PaneColor::Iris,
            PaneColorArg::Jade => PaneColor::Jade,
            PaneColorArg::Amber => PaneColor::Amber,
            PaneColorArg::Coral => PaneColor::Coral,
            PaneColorArg::Rose => PaneColor::Rose,
        }
    }
}

/// Clap value parser for `--name`.
///
/// Rules: non-empty, ≤32 chars (counting Unicode scalar values), no
/// ASCII control characters (\x00–\x1f or \x7f). Newlines or NULs
/// would otherwise become part of the terminal header.
fn parse_pane_name(s: &str) -> Result<String, String> {
    if s.is_empty() {
        return Err("name must not be empty".into());
    }
    if s.chars().count() > 32 {
        return Err("name must be 32 characters or fewer".into());
    }
    if s.chars().any(|c| c.is_control()) {
        return Err("name must not contain control characters".into());
    }
    Ok(s.to_owned())
}

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
    /// Optional pane label shown in the terminal header (1–32 chars, no control characters).
    #[arg(long, value_parser = parse_pane_name)]
    pub name: Option<String>,
    /// Optional pane color shown in the terminal header.
    #[arg(long, value_enum)]
    pub color: Option<PaneColorArg>,
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
    fn parses_valid_color() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "split", "--color", "coral",
        ])
        .unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.color, Some(PaneColorArg::Coral));
    }

    #[test]
    fn rejects_unknown_color() {
        let err = Cli::try_parse_from([
            "kiri", "term", "split", "--color", "magenta",
        ]);
        assert!(err.is_err(), "should reject unknown color");
    }

    #[test]
    fn parses_valid_name() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "split", "--name", "build",
        ])
        .unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.name.as_deref(), Some("build"));
    }

    #[test]
    fn rejects_empty_name() {
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", ""]);
        assert!(err.is_err(), "should reject empty name");
    }

    #[test]
    fn rejects_name_over_32_chars() {
        let long = "a".repeat(33);
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", &long]);
        assert!(err.is_err(), "should reject 33-char name");
    }

    #[test]
    fn accepts_name_at_32_chars() {
        let edge = "a".repeat(32);
        let cli = Cli::try_parse_from(["kiri", "term", "split", "--name", &edge]).unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.name.as_ref().unwrap().chars().count(), 32);
    }

    #[test]
    fn rejects_control_char_name() {
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", "ab\nc"]);
        assert!(err.is_err(), "should reject name with newline");
    }
}
