use crate::types::{ErrorCode, PaneRef, SplitDirection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
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
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

fn default_run_timeout() -> u64 {
    300
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
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
    Send,
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
        });
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
    fn response_send_serializes_as_unit() {
        let v = serde_json::to_value(Response::Send).unwrap();
        assert_eq!(v, serde_json::json!({ "type": "send" }));
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
}
