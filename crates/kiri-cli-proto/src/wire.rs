use crate::types::{ErrorCode, PaneRef, SplitDirection};
use serde::{Deserialize, Serialize};

/// Routing target for `Request::SignalSend`.
///
/// The wire form is JSON-untagged: `{ "pane": <ref> }` for a specific
/// pane, or the bare strings `"parent"` / `"children"` for relatives of
/// the sender pane. Keep the variants tight — anything looser would
/// collide with `PaneRef::Id`'s string form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalTarget {
    /// Route to a single pane (by index or id).
    Pane(PaneRef),
    /// Route to the sender pane's parent (the pane it was split from).
    Parent,
    /// Route to every pane this sender pane spawned via `split`.
    Children,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// Ask the server which window/project this socket belongs to.
    /// Used by the CLI to refuse acting on a different project's window.
    WhoAmI,
    Ls,
    Run {
        pane: PaneRef,
        cmd: String,
        #[serde(default = "default_run_timeout")]
        timeout_secs: u64,
        #[serde(default)]
        full_output: bool,
    },
    Send {
        pane: PaneRef,
        data: String,
        /// When true (the default), the backend appends a submit sequence
        /// (`\r`) after `data` if the target pane's foreground process is
        /// an interactive AI assistant (claude / codex). Non-AI panes
        /// still receive `data` but the submit sequence is skipped — the
        /// response's `submitted` field reports whether the appended `\r`
        /// was actually written. Pass `false` (CLI: `--no-submit`) to
        /// keep the data uncommitted in the AI's input box, e.g. when
        /// building up multi-step input before the user presses Enter.
        #[serde(default = "default_true")]
        submit: bool,
    },
    Read {
        pane: PaneRef,
        #[serde(default)]
        since: Option<u64>,
        #[serde(default)]
        tail: Option<usize>,
    },
    Follow {
        pane: PaneRef,
    },
    Cancel {
        pane: PaneRef,
    },
    Split {
        pane: PaneRef,
        direction: SplitDirection,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<crate::PaneColor>,
        #[serde(default)]
        minimized: bool,
    },
    Close {
        pane: PaneRef,
    },
    Minimize {
        pane: PaneRef,
    },
    Restore {
        pane: PaneRef,
    },
    /// Update the label (name and/or color) of an existing pane.
    ///
    /// `set_name` / `set_color` install a new value. `clear_name` /
    /// `clear_color` remove an existing one. Exactly-one-of within each
    /// pair: `set_name` and `clear_name` must not both be set in the
    /// same request, and the same for color. At least one of the four
    /// must be present — a request that touches nothing is rejected as
    /// `InvalidArgument` so consumers don't silently no-op.
    SetLabel {
        pane: PaneRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        set_name: Option<String>,
        #[serde(default, skip_serializing_if = "is_false")]
        clear_name: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        set_color: Option<crate::PaneColor>,
        #[serde(default, skip_serializing_if = "is_false")]
        clear_color: bool,
    },
    /// Enqueue a named signal on `target`'s queue. `from` is the sender
    /// pane (used to resolve `target = Parent | Children`).
    SignalSend {
        from: PaneRef,
        target: SignalTarget,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    /// Block until a signal with `name` lands in `pane`'s queue, or
    /// until `timeout_secs` elapses. Server clamps to ≤600s.
    SignalWait {
        pane: PaneRef,
        name: String,
        #[serde(default = "default_signal_wait_timeout")]
        timeout_secs: u64,
    },
    /// Non-blocking peek at every signal currently queued on `pane`.
    SignalList {
        pane: PaneRef,
    },
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

fn default_true() -> bool {
    true
}

fn default_run_timeout() -> u64 {
    300
}

fn default_signal_wait_timeout() -> u64 {
    60
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneInfo {
    pub index: u32,
    pub id: String,
    pub terminal_id: u32,
    pub cwd: Option<String>,
    pub process_name: String,
    pub running: bool,
    pub memory_bytes: u64,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<crate::PaneColor>,
    #[serde(default)]
    pub minimized: bool,
}

/// One entry from `Response::SignalList`'s `signals` array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SignalEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    pub sender_pane_id: String,
    pub sent_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    WhoAmI {
        window_label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        project_path: Option<String>,
    },
    Ls {
        panes: Vec<PaneInfo>,
    },
    Run {
        exit_code: Option<i32>,
        output: String,
        output_truncated: bool,
        lines_omitted: usize,
        timed_out: bool,
        cursor: u64,
    },
    Send {
        /// True when the submit sequence (`\r`) was appended after the
        /// data. Only set for `Request::Send { submit: true, .. }` whose
        /// target pane was running claude / codex at the time. Omitted
        /// from the wire when false so older clients (which decode into
        /// the field-less `Response::Send`) see the same JSON shape they
        /// always did.
        #[serde(default, skip_serializing_if = "is_false")]
        submitted: bool,
    },
    Read {
        output: String,
        cursor: u64,
        bytes_dropped: u64,
    },
    FollowChunk {
        data: String,
        cursor: u64,
    },
    FollowEnd,
    Cancel,
    Split {
        new_pane_id: String,
        new_pane_index: u32,
    },
    Close,
    Minimize,
    Restore,
    SetLabel,
    /// Result of a `SignalSend`. `delivered` is the number of distinct
    /// pane queues the signal landed on (0 if the target had no matching
    /// pane, e.g. no parent or no children).
    SignalSend {
        delivered: u32,
    },
    /// Result of a `SignalWait` that completed before its timeout. The
    /// timeout path is reported as a `Response::Error` with code
    /// `Timeout`, not this variant.
    SignalWait {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
        sender_pane_id: String,
        sent_at_ms: u64,
    },
    SignalList {
        signals: Vec<SignalEntry>,
    },
    Error {
        code: ErrorCode,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<serde_json::Value>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: Serialize + for<'a> Deserialize<'a> + PartialEq + std::fmt::Debug>(v: &T) {
        let s = serde_json::to_string(v).unwrap();
        let parsed: T = serde_json::from_str(&s).unwrap();
        assert_eq!(&parsed, v, "roundtrip mismatch via {}", s);
    }

    #[test]
    fn request_ls_serializes() {
        let req = Request::Ls;
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "ls" }));
        roundtrip(&req);
    }

    #[test]
    fn request_run_uses_default_timeout() {
        let parsed: Request =
            serde_json::from_value(serde_json::json!({ "type": "run", "pane": 0, "cmd": "ls" }))
                .unwrap();
        assert_eq!(
            parsed,
            Request::Run {
                pane: PaneRef::Index(0),
                cmd: "ls".into(),
                timeout_secs: 300,
                full_output: false,
            }
        );
    }

    #[test]
    fn request_send_roundtrip() {
        roundtrip(&Request::Send {
            pane: PaneRef::Index(1),
            data: "echo hi\n".into(),
            submit: false,
        });
        roundtrip(&Request::Send {
            pane: PaneRef::Index(1),
            data: "echo hi".into(),
            submit: true,
        });
    }

    #[test]
    fn request_send_submit_defaults_to_true() {
        // Auto-submit is the default — older clients that omit the
        // field opt into the new behavior so `kiri term send "..."`
        // works as a one-shot when the target pane runs claude / codex.
        let parsed: Request = serde_json::from_value(
            serde_json::json!({ "type": "send", "pane": 0, "data": "hi" }),
        )
        .unwrap();
        assert_eq!(
            parsed,
            Request::Send {
                pane: PaneRef::Index(0),
                data: "hi".into(),
                submit: true,
            }
        );
    }

    #[test]
    fn request_split_roundtrip() {
        roundtrip(&Request::Split {
            pane: PaneRef::focused(),
            direction: SplitDirection::Horizontal,
            name: None,
            color: None,
            minimized: false,
        });
    }

    #[test]
    fn response_run_roundtrip() {
        roundtrip(&Response::Run {
            exit_code: Some(0),
            output: "hi".into(),
            output_truncated: false,
            lines_omitted: 0,
            timed_out: false,
            cursor: 42,
        });
    }

    #[test]
    fn response_error_serializes() {
        let v = serde_json::to_value(Response::Error {
            code: ErrorCode::PaneBusy,
            message: "busy".into(),
            detail: None,
        })
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "type": "error", "code": "pane_busy", "message": "busy" })
        );
    }

    #[test]
    fn response_send_default_submitted_omits_field() {
        // submitted defaults to false; serializing the default omits
        // the field so older clients that don't know about it see the
        // same JSON shape as before this PR.
        let v = serde_json::to_value(Response::Send { submitted: false }).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "send" }));
        let parsed: Response = serde_json::from_value(v).unwrap();
        assert_eq!(parsed, Response::Send { submitted: false });
    }

    #[test]
    fn response_send_submitted_true_emits_field() {
        let v = serde_json::to_value(Response::Send { submitted: true }).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "send", "submitted": true }));
        let parsed: Response = serde_json::from_value(v).unwrap();
        assert_eq!(parsed, Response::Send { submitted: true });
    }

    #[test]
    fn response_send_submitted_roundtrip() {
        roundtrip(&Response::Send { submitted: true });
        roundtrip(&Response::Send { submitted: false });
    }

    #[test]
    fn request_split_with_name_and_color_roundtrip() {
        roundtrip(&Request::Split {
            pane: PaneRef::focused(),
            direction: SplitDirection::Horizontal,
            name: Some("build".into()),
            color: Some(crate::PaneColor::Coral),
            minimized: false,
        });
    }

    #[test]
    fn request_split_without_label_omits_fields() {
        let v = serde_json::to_value(Request::Split {
            pane: PaneRef::focused(),
            direction: SplitDirection::Horizontal,
            name: None,
            color: None,
            minimized: false,
        })
        .unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }

    #[test]
    fn request_split_back_compat_without_fields_parses() {
        let parsed: Request = serde_json::from_value(
            serde_json::json!({ "type": "split", "pane": "focused", "direction": "vertical" }),
        )
        .unwrap();
        assert_eq!(
            parsed,
            Request::Split {
                pane: PaneRef::focused(),
                direction: SplitDirection::Vertical,
                name: None,
                color: None,
                minimized: false,
            }
        );
    }

    #[test]
    fn pane_info_with_label_roundtrip() {
        let info = PaneInfo {
            index: 0,
            id: "pane-1".into(),
            terminal_id: 1,
            cwd: Some("/p".into()),
            process_name: "zsh".into(),
            running: false,
            memory_bytes: 0,
            focused: true,
            name: Some("agent".into()),
            color: Some(crate::PaneColor::Iris),
            minimized: false,
        };
        let s = serde_json::to_string(&info).unwrap();
        let back: PaneInfo = serde_json::from_str(&s).unwrap();
        assert_eq!(back.name.as_deref(), Some("agent"));
        assert_eq!(back.color, Some(crate::PaneColor::Iris));
    }

    #[test]
    fn pane_info_without_label_omits_fields() {
        let info = PaneInfo {
            index: 0,
            id: "pane-1".into(),
            terminal_id: 1,
            cwd: None,
            process_name: "zsh".into(),
            running: false,
            memory_bytes: 0,
            focused: false,
            name: None,
            color: None,
            minimized: false,
        };
        let v = serde_json::to_value(&info).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }

    #[test]
    fn pane_info_minimized_defaults_to_false_when_absent() {
        let parsed: PaneInfo = serde_json::from_value(serde_json::json!({
            "index": 0,
            "id": "pane-1",
            "terminal_id": 1,
            "cwd": null,
            "process_name": "zsh",
            "running": false,
            "memory_bytes": 0,
            "focused": true
        }))
        .unwrap();
        assert!(!parsed.minimized);
    }

    #[test]
    fn pane_info_minimized_round_trips() {
        let info = PaneInfo {
            index: 0,
            id: "pane-1".into(),
            terminal_id: 1,
            cwd: None,
            process_name: "zsh".into(),
            running: false,
            memory_bytes: 0,
            focused: true,
            name: None,
            color: None,
            minimized: true,
        };
        roundtrip(&info);
    }

    #[test]
    fn request_minimize_round_trip() {
        roundtrip(&Request::Minimize {
            pane: PaneRef::Index(2),
        });
    }

    #[test]
    fn request_restore_round_trip() {
        roundtrip(&Request::Restore {
            pane: PaneRef::focused(),
        });
    }

    #[test]
    fn response_minimize_serializes_as_unit() {
        let v = serde_json::to_value(Response::Minimize).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "minimize" }));
    }

    #[test]
    fn response_restore_serializes_as_unit() {
        let v = serde_json::to_value(Response::Restore).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "restore" }));
    }

    #[test]
    fn request_split_defaults_minimized_to_false() {
        let parsed: Request = serde_json::from_value(serde_json::json!({
            "type": "split",
            "pane": 0,
            "direction": "horizontal"
        }))
        .unwrap();
        assert_eq!(
            parsed,
            Request::Split {
                pane: PaneRef::Index(0),
                direction: SplitDirection::Horizontal,
                name: None,
                color: None,
                minimized: false,
            }
        );
    }

    #[test]
    fn request_split_with_minimized_round_trip() {
        roundtrip(&Request::Split {
            pane: PaneRef::Index(1),
            direction: SplitDirection::Vertical,
            name: None,
            color: None,
            minimized: true,
        });
    }

    #[test]
    fn request_whoami_serializes() {
        let v = serde_json::to_value(Request::WhoAmI).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "who_am_i" }));
        roundtrip(&Request::WhoAmI);
    }

    #[test]
    fn response_whoami_with_project_round_trip() {
        roundtrip(&Response::WhoAmI {
            window_label: "window-1".into(),
            project_path: Some("/Users/u/projects/kiri".into()),
        });
    }

    #[test]
    fn response_whoami_without_project_omits_field() {
        let v = serde_json::to_value(Response::WhoAmI {
            window_label: "window-2".into(),
            project_path: None,
        })
        .unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("project_path"));
        assert_eq!(obj.get("window_label").unwrap(), "window-2");
    }

    #[test]
    fn request_set_label_only_name_round_trip() {
        roundtrip(&Request::SetLabel {
            pane: PaneRef::focused(),
            set_name: Some("build".into()),
            clear_name: false,
            set_color: None,
            clear_color: false,
        });
    }

    #[test]
    fn request_set_label_only_color_round_trip() {
        roundtrip(&Request::SetLabel {
            pane: PaneRef::Index(2),
            set_name: None,
            clear_name: false,
            set_color: Some(crate::PaneColor::Coral),
            clear_color: false,
        });
    }

    #[test]
    fn request_set_label_clears_round_trip() {
        roundtrip(&Request::SetLabel {
            pane: PaneRef::Id("pane-7".into()),
            set_name: None,
            clear_name: true,
            set_color: None,
            clear_color: true,
        });
    }

    #[test]
    fn request_set_label_omits_default_fields_in_json() {
        let v = serde_json::to_value(Request::SetLabel {
            pane: PaneRef::focused(),
            set_name: Some("agent".into()),
            clear_name: false,
            set_color: None,
            clear_color: false,
        })
        .unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj.contains_key("set_name"));
        assert!(!obj.contains_key("clear_name"));
        assert!(!obj.contains_key("set_color"));
        assert!(!obj.contains_key("clear_color"));
    }

    #[test]
    fn request_set_label_parses_from_minimal_json() {
        let parsed: Request = serde_json::from_value(serde_json::json!({
            "type": "set_label",
            "pane": "focused",
            "set_color": "iris"
        }))
        .unwrap();
        assert_eq!(
            parsed,
            Request::SetLabel {
                pane: PaneRef::focused(),
                set_name: None,
                clear_name: false,
                set_color: Some(crate::PaneColor::Iris),
                clear_color: false,
            }
        );
    }

    #[test]
    fn response_set_label_serializes_as_unit() {
        let v = serde_json::to_value(Response::SetLabel).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "set_label" }));
    }

    // --- signal wire types ---

    #[test]
    fn signal_target_parent_serializes_as_bare_string() {
        let v = serde_json::to_value(SignalTarget::Parent).unwrap();
        assert_eq!(v, serde_json::json!("parent"));
        let back: SignalTarget = serde_json::from_value(v).unwrap();
        assert_eq!(back, SignalTarget::Parent);
    }

    #[test]
    fn signal_target_children_serializes_as_bare_string() {
        let v = serde_json::to_value(SignalTarget::Children).unwrap();
        assert_eq!(v, serde_json::json!("children"));
    }

    #[test]
    fn signal_target_pane_serializes_with_key() {
        let v = serde_json::to_value(SignalTarget::Pane(PaneRef::Id("pane-1".into()))).unwrap();
        assert_eq!(v, serde_json::json!({ "pane": "pane-1" }));
        let back: SignalTarget = serde_json::from_value(v).unwrap();
        assert_eq!(back, SignalTarget::Pane(PaneRef::Id("pane-1".into())));
    }

    #[test]
    fn signal_target_pane_with_index() {
        let v = serde_json::to_value(SignalTarget::Pane(PaneRef::Index(2))).unwrap();
        assert_eq!(v, serde_json::json!({ "pane": 2 }));
    }

    #[test]
    fn request_signal_send_roundtrip_with_data() {
        roundtrip(&Request::SignalSend {
            from: PaneRef::focused(),
            target: SignalTarget::Parent,
            name: "ready".into(),
            data: Some(serde_json::json!({ "step": 1 })),
        });
    }

    #[test]
    fn request_signal_send_roundtrip_without_data() {
        roundtrip(&Request::SignalSend {
            from: PaneRef::focused(),
            target: SignalTarget::Pane(PaneRef::Index(0)),
            name: "go".into(),
            data: None,
        });
    }

    #[test]
    fn request_signal_send_omits_data_field_when_none() {
        let v = serde_json::to_value(Request::SignalSend {
            from: PaneRef::focused(),
            target: SignalTarget::Children,
            name: "ping".into(),
            data: None,
        })
        .unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("data"));
        assert_eq!(obj.get("target"), Some(&serde_json::json!("children")));
    }

    #[test]
    fn request_signal_wait_uses_default_timeout() {
        let parsed: Request = serde_json::from_value(serde_json::json!({
            "type": "signal_wait",
            "pane": "focused",
            "name": "ready"
        }))
        .unwrap();
        assert_eq!(
            parsed,
            Request::SignalWait {
                pane: PaneRef::focused(),
                name: "ready".into(),
                timeout_secs: 60,
            }
        );
    }

    #[test]
    fn request_signal_list_roundtrip() {
        roundtrip(&Request::SignalList {
            pane: PaneRef::Index(0),
        });
    }

    #[test]
    fn response_signal_send_roundtrip() {
        roundtrip(&Response::SignalSend { delivered: 3 });
    }

    #[test]
    fn response_signal_wait_roundtrip() {
        roundtrip(&Response::SignalWait {
            name: "ready".into(),
            data: Some(serde_json::json!({ "answer": 42 })),
            sender_pane_id: "pane-2".into(),
            sent_at_ms: 1_234_567_890,
        });
    }

    #[test]
    fn response_signal_wait_omits_data_when_none() {
        let v = serde_json::to_value(Response::SignalWait {
            name: "ready".into(),
            data: None,
            sender_pane_id: "pane-2".into(),
            sent_at_ms: 0,
        })
        .unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("data"));
    }

    #[test]
    fn response_signal_list_roundtrip() {
        roundtrip(&Response::SignalList {
            signals: vec![
                SignalEntry {
                    name: "a".into(),
                    data: None,
                    sender_pane_id: "pane-1".into(),
                    sent_at_ms: 1,
                },
                SignalEntry {
                    name: "b".into(),
                    data: Some(serde_json::json!(42)),
                    sender_pane_id: "pane-2".into(),
                    sent_at_ms: 2,
                },
            ],
        });
    }
}
