//! clap definitions for the `kiri` CLI binary.
//!
//! The binary lives at `~/.kiri/bin/kiri` (installed by the kiri app at
//! startup) and is automatically on PATH inside kiri terminals via
//! `KIRI_SOCKET`-driven env injection. Outside of a kiri terminal the
//! command will report an error because `KIRI_SOCKET` is not set.

use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum SignalTargetArg {
    Parent,
    Children,
}

/// Clap value parser for `--name` on `term split`.
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

/// Clap value parser for `--name` on `term signal {send,wait}`.
///
/// Rules: 1–64 chars, only `[a-zA-Z0-9_.-]`. Signal names are used as
/// queue keys and exposed verbatim in JSON output, so we keep the
/// character set tight.
fn parse_signal_name(s: &str) -> Result<String, String> {
    if s.is_empty() {
        return Err("name must not be empty".into());
    }
    if s.len() > 64 {
        return Err("name must be 64 characters or fewer".into());
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err("name may only contain [a-zA-Z0-9_.-]".into());
    }
    Ok(s.to_owned())
}

/// Clap value parser for `--data` on `term signal send`.
fn parse_signal_data(s: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(s).map_err(|e| format!("invalid JSON for --data: {e}"))
}

/// Clap value parser for `--timeout` on `term signal wait`.
///
/// Server-side enforces an upper bound of 600 seconds; we reject larger
/// values at the CLI boundary so the user sees the error immediately.
fn parse_signal_timeout(s: &str) -> Result<u64, String> {
    let v: u64 = s
        .parse()
        .map_err(|e| format!("--timeout must be a non-negative integer: {e}"))?;
    if v == 0 {
        return Err("--timeout must be at least 1 second".into());
    }
    if v > 600 {
        return Err("--timeout must be 600 seconds or less".into());
    }
    Ok(v)
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
    /// Collapse the shortcut bar to a thin strip with only the restore + settings buttons.
    Minimize(PaneOpt),
    /// Expand a minimized shortcut bar back to its full layout.
    Restore(PaneOpt),
    /// Rename and/or recolor an existing pane (including the focused one).
    SetLabel(SetLabelArgs),
    /// Exchange named messages between this pane and its parent / children.
    #[command(subcommand)]
    Signal(SignalCmd),
}

#[derive(Subcommand, Debug)]
pub enum SignalCmd {
    /// Send a named signal to a specific pane or to relatives.
    Send(SignalSendArgs),
    /// Block until a named signal arrives in this pane's queue.
    Wait(SignalWaitArgs),
    /// Print the signals currently queued on this pane.
    List(SignalListArgs),
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
    /// Pane label shown in the terminal header (1–32 chars, no control characters).
    #[arg(long, value_parser = parse_pane_name, required = true)]
    pub name: String,
    /// Pane color shown in the terminal header.
    #[arg(long, value_enum, required = true)]
    pub color: PaneColorArg,
    /// Create the new pane with its shortcut bar fully expanded.
    ///
    /// New panes are minimized by default — pass this flag for user-facing
    /// side panes the user is expected to interact with.
    #[arg(long = "no-minimized")]
    pub no_minimized: bool,
}

#[derive(Args, Debug)]
#[command(group(
    ArgGroup::new("signal_send_target")
        .required(true)
        .args(["pane", "target"]),
))]
pub struct SignalSendArgs {
    /// Target pane (index or id). Mutually exclusive with `--target`.
    #[arg(long, value_name = "I_OR_ID")]
    pub pane: Option<String>,
    /// Send to the sender pane's parent or all of its children.
    #[arg(long, value_enum)]
    pub target: Option<SignalTargetArg>,
    /// Override the sender pane. Defaults to the focused pane.
    ///
    /// Mostly useful when the sending pane is not focused — e.g. an
    /// agent running inside a minimized side pane that wants its
    /// `--target parent` resolution to use the side pane as the sender.
    #[arg(long, value_name = "I_OR_ID")]
    pub from: Option<String>,
    /// Signal name (1–64 chars, [a-zA-Z0-9_.-] only).
    #[arg(long, value_parser = parse_signal_name, required = true)]
    pub name: String,
    /// Optional JSON payload delivered alongside the signal.
    #[arg(long, value_parser = parse_signal_data)]
    pub data: Option<serde_json::Value>,
}

#[derive(Args, Debug)]
pub struct SignalWaitArgs {
    /// Pane whose queue to wait on (defaults to the focused pane).
    #[arg(long, value_name = "I_OR_ID")]
    pub pane: Option<String>,
    /// Signal name to wait for.
    #[arg(long, value_parser = parse_signal_name, required = true)]
    pub name: String,
    /// Max seconds to block before returning a `timeout` error.
    #[arg(long, value_parser = parse_signal_timeout, default_value_t = 60)]
    pub timeout: u64,
    /// Print the signal's JSON `data` payload to stdout on success.
    #[arg(long)]
    pub print_data: bool,
}

#[derive(Args, Debug)]
pub struct SignalListArgs {
    /// Pane whose queue to inspect (defaults to the focused pane).
    #[arg(long, value_name = "I_OR_ID")]
    pub pane: Option<String>,
}

#[derive(Args, Debug)]
pub struct SetLabelArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    /// New label text shown in the header (1–32 chars, no control characters).
    #[arg(long, value_parser = parse_pane_name, conflicts_with = "clear_name")]
    pub name: Option<String>,
    /// Remove an existing label, leaving the pane unnamed.
    #[arg(long)]
    pub clear_name: bool,
    /// New color shown in the header.
    #[arg(long, value_enum, conflicts_with = "clear_color")]
    pub color: Option<PaneColorArg>,
    /// Remove an existing color, leaving the pane uncolored.
    #[arg(long)]
    pub clear_color: bool,
}

impl SetLabelArgs {
    /// True when none of `--name`, `--clear-name`, `--color`, `--clear-color` were given.
    pub fn is_empty_update(&self) -> bool {
        self.name.is_none() && !self.clear_name && self.color.is_none() && !self.clear_color
    }
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

/// Translate an optional `--pane` / `--from` string into a `PaneRef`.
pub fn parse_pane_string(opt: &Option<String>) -> PaneRef {
    match opt {
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

    fn split_args<'a>(extra: &'a [&'a str]) -> Vec<&'a str> {
        // Default `--name`/`--color` are now required for `term split`.
        let mut v = vec!["kiri", "term", "split", "--name", "build", "--color", "coral"];
        v.extend_from_slice(extra);
        v
    }

    #[test]
    fn parses_valid_color() {
        let cli = Cli::try_parse_from(split_args(&[])).unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.color, PaneColorArg::Coral);
    }

    #[test]
    fn rejects_unknown_color() {
        let err = Cli::try_parse_from([
            "kiri", "term", "split", "--name", "build", "--color", "magenta",
        ]);
        assert!(err.is_err(), "should reject unknown color");
    }

    #[test]
    fn parses_valid_name() {
        let cli = Cli::try_parse_from(split_args(&[])).unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.name, "build");
    }

    #[test]
    fn rejects_empty_name() {
        let err = Cli::try_parse_from([
            "kiri", "term", "split", "--name", "", "--color", "coral",
        ]);
        assert!(err.is_err(), "should reject empty name");
    }

    #[test]
    fn rejects_name_over_32_chars() {
        let long = "a".repeat(33);
        let err = Cli::try_parse_from([
            "kiri", "term", "split", "--name", &long, "--color", "coral",
        ]);
        assert!(err.is_err(), "should reject 33-char name");
    }

    #[test]
    fn accepts_name_at_32_chars() {
        let edge = "a".repeat(32);
        let cli = Cli::try_parse_from([
            "kiri", "term", "split", "--name", &edge, "--color", "coral",
        ])
        .unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.name.chars().count(), 32);
    }

    #[test]
    fn rejects_control_char_name() {
        let err = Cli::try_parse_from([
            "kiri", "term", "split", "--name", "ab\nc", "--color", "coral",
        ]);
        assert!(err.is_err(), "should reject name with newline");
    }

    #[test]
    fn split_rejects_missing_name() {
        let err = Cli::try_parse_from(["kiri", "term", "split", "--color", "coral"]);
        assert!(err.is_err(), "should reject split missing --name");
    }

    #[test]
    fn split_rejects_missing_color() {
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", "build"]);
        assert!(err.is_err(), "should reject split missing --color");
    }

    #[test]
    fn parse_split_default_no_minimized_is_false() {
        // Default is minimized=true (i.e. no_minimized=false).
        let cli = Cli::try_parse_from(split_args(&[])).unwrap();
        match cli.command {
            Top::Term(TermCmd::Split(args)) => assert!(!args.no_minimized),
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn parse_split_no_minimized_flag_present() {
        let cli = Cli::try_parse_from(split_args(&["--no-minimized"])).unwrap();
        match cli.command {
            Top::Term(TermCmd::Split(args)) => assert!(args.no_minimized),
            _ => panic!("expected split"),
        }
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

    // --- signal subcommand tests ---

    #[test]
    fn parse_signal_send_with_pane_target() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "signal", "send", "--pane", "pane-2", "--name", "ready",
        ])
        .unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::Send(a))) => {
                assert_eq!(a.pane.as_deref(), Some("pane-2"));
                assert!(a.target.is_none());
                assert_eq!(a.name, "ready");
                assert!(a.data.is_none());
            }
            _ => panic!("expected signal send"),
        }
    }

    #[test]
    fn parse_signal_send_with_parent_target_and_data() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "signal", "send", "--target", "parent", "--name", "done", "--data",
            "{\"ok\":true}",
        ])
        .unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::Send(a))) => {
                assert_eq!(a.target, Some(SignalTargetArg::Parent));
                assert!(a.pane.is_none());
                assert_eq!(a.name, "done");
                assert_eq!(
                    a.data.unwrap(),
                    serde_json::json!({ "ok": true })
                );
            }
            _ => panic!("expected signal send"),
        }
    }

    #[test]
    fn parse_signal_send_with_children_target() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "signal", "send", "--target", "children", "--name", "shutdown",
        ])
        .unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::Send(a))) => {
                assert_eq!(a.target, Some(SignalTargetArg::Children));
            }
            _ => panic!("expected signal send"),
        }
    }

    #[test]
    fn signal_send_requires_target_or_pane() {
        let err = Cli::try_parse_from(["kiri", "term", "signal", "send", "--name", "n"]);
        assert!(err.is_err(), "must specify --pane or --target");
    }

    #[test]
    fn signal_send_rejects_pane_and_target_together() {
        let err = Cli::try_parse_from([
            "kiri", "term", "signal", "send", "--pane", "0", "--target", "parent", "--name", "n",
        ]);
        assert!(err.is_err(), "must not allow --pane and --target together");
    }

    #[test]
    fn signal_send_rejects_invalid_data_json() {
        let err = Cli::try_parse_from([
            "kiri", "term", "signal", "send", "--target", "parent", "--name", "n", "--data",
            "not-json",
        ]);
        assert!(err.is_err(), "should reject invalid JSON for --data");
    }

    #[test]
    fn signal_send_rejects_invalid_name_char() {
        let err = Cli::try_parse_from([
            "kiri", "term", "signal", "send", "--target", "parent", "--name", "bad name",
        ]);
        assert!(err.is_err(), "should reject name with space");
    }

    #[test]
    fn signal_send_accepts_dot_dash_underscore_in_name() {
        let cli = Cli::try_parse_from([
            "kiri",
            "term",
            "signal",
            "send",
            "--target",
            "parent",
            "--name",
            "build.done-1_step",
        ])
        .unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::Send(a))) => {
                assert_eq!(a.name, "build.done-1_step");
            }
            _ => panic!("expected signal send"),
        }
    }

    #[test]
    fn parse_signal_wait_with_default_timeout() {
        let cli =
            Cli::try_parse_from(["kiri", "term", "signal", "wait", "--name", "ready"]).unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::Wait(a))) => {
                assert_eq!(a.name, "ready");
                assert_eq!(a.timeout, 60);
                assert!(!a.print_data);
            }
            _ => panic!("expected signal wait"),
        }
    }

    #[test]
    fn parse_signal_wait_print_data_and_custom_timeout() {
        let cli = Cli::try_parse_from([
            "kiri",
            "term",
            "signal",
            "wait",
            "--name",
            "ready",
            "--timeout",
            "120",
            "--print-data",
        ])
        .unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::Wait(a))) => {
                assert_eq!(a.timeout, 120);
                assert!(a.print_data);
            }
            _ => panic!("expected signal wait"),
        }
    }

    #[test]
    fn signal_wait_rejects_zero_timeout() {
        let err = Cli::try_parse_from([
            "kiri", "term", "signal", "wait", "--name", "ready", "--timeout", "0",
        ]);
        assert!(err.is_err(), "timeout=0 should be rejected");
    }

    #[test]
    fn signal_wait_rejects_timeout_over_600() {
        let err = Cli::try_parse_from([
            "kiri", "term", "signal", "wait", "--name", "ready", "--timeout", "601",
        ]);
        assert!(err.is_err(), "timeout>600 should be rejected");
    }

    #[test]
    fn parse_signal_list_default_pane() {
        let cli = Cli::try_parse_from(["kiri", "term", "signal", "list"]).unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::List(a))) => {
                assert!(a.pane.is_none());
            }
            _ => panic!("expected signal list"),
        }
    }

    #[test]
    fn parse_signal_list_with_pane() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "signal", "list", "--pane", "pane-3",
        ])
        .unwrap();
        match cli.command {
            Top::Term(TermCmd::Signal(SignalCmd::List(a))) => {
                assert_eq!(a.pane.as_deref(), Some("pane-3"));
            }
            _ => panic!("expected signal list"),
        }
    }

    #[test]
    fn parse_set_label_with_name_and_color() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "set-label", "--pane", "2", "--name", "build", "--color", "coral",
        ])
        .unwrap();
        let Top::Term(TermCmd::SetLabel(args)) = cli.command else {
            panic!("expected set-label");
        };
        assert_eq!(parse_pane(&args.pane), PaneRef::Index(2));
        assert_eq!(args.name.as_deref(), Some("build"));
        assert_eq!(args.color, Some(PaneColorArg::Coral));
        assert!(!args.clear_name);
        assert!(!args.clear_color);
        assert!(!args.is_empty_update());
    }

    #[test]
    fn parse_set_label_only_clear_name() {
        let cli =
            Cli::try_parse_from(["kiri", "term", "set-label", "--clear-name"]).unwrap();
        let Top::Term(TermCmd::SetLabel(args)) = cli.command else {
            panic!("expected set-label");
        };
        assert!(args.clear_name);
        assert!(args.name.is_none());
        // Targets focused pane by default.
        assert_eq!(parse_pane(&args.pane), PaneRef::focused());
        assert!(!args.is_empty_update());
    }

    #[test]
    fn parse_set_label_only_clear_color() {
        let cli =
            Cli::try_parse_from(["kiri", "term", "set-label", "--clear-color"]).unwrap();
        let Top::Term(TermCmd::SetLabel(args)) = cli.command else {
            panic!("expected set-label");
        };
        assert!(args.clear_color);
        assert!(args.color.is_none());
        assert!(!args.is_empty_update());
    }

    #[test]
    fn set_label_rejects_name_with_clear_name() {
        let err = Cli::try_parse_from([
            "kiri", "term", "set-label", "--name", "x", "--clear-name",
        ]);
        assert!(err.is_err(), "should reject conflicting --name + --clear-name");
    }

    #[test]
    fn set_label_rejects_color_with_clear_color() {
        let err = Cli::try_parse_from([
            "kiri", "term", "set-label", "--color", "sky", "--clear-color",
        ]);
        assert!(err.is_err(), "should reject conflicting --color + --clear-color");
    }

    #[test]
    fn set_label_rejects_empty_name() {
        let err = Cli::try_parse_from(["kiri", "term", "set-label", "--name", ""]);
        assert!(err.is_err(), "should reject empty name");
    }

    #[test]
    fn set_label_no_flags_is_empty_update() {
        let cli = Cli::try_parse_from(["kiri", "term", "set-label"]).unwrap();
        let Top::Term(TermCmd::SetLabel(args)) = cli.command else {
            panic!("expected set-label");
        };
        // Clap doesn't enforce "at least one flag" here — main.rs and the
        // backend handler are the ones who reject this as InvalidArgument.
        assert!(args.is_empty_update());
    }
}
