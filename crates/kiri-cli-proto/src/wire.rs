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
    },
    Close {
        pane: PaneRef,
    },
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
        });
    }

    #[test]
    fn request_split_without_label_omits_fields() {
        let v = serde_json::to_value(Request::Split {
            pane: PaneRef::focused(),
            direction: SplitDirection::Horizontal,
            name: None,
            color: None,
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
        };
        let v = serde_json::to_value(&info).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }
}
