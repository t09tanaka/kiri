use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PaneRef {
    Index(u32),
    Id(String),
}

impl PaneRef {
    pub const FOCUSED_SENTINEL: &'static str = "focused";

    pub fn focused() -> Self {
        Self::Id(Self::FOCUSED_SENTINEL.to_string())
    }

    pub fn is_focused(&self) -> bool {
        matches!(self, Self::Id(s) if s == Self::FOCUSED_SENTINEL)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneColor {
    Sky,
    Iris,
    Jade,
    Amber,
    Coral,
    Rose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NoKiriWindow,
    CwdOutsideProject,
    PaneNotFound,
    PaneBusy,
    Timeout,
    PtyError,
    FrontendUnresponsive,
    ProtocolError,
    InvalidArgument,
    InternalError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_ref_index_serializes_as_integer() {
        let v = serde_json::to_value(PaneRef::Index(3)).unwrap();
        assert_eq!(v, serde_json::json!(3));
    }

    #[test]
    fn pane_ref_id_serializes_as_string() {
        let v = serde_json::to_value(PaneRef::Id("abc".into())).unwrap();
        assert_eq!(v, serde_json::json!("abc"));
    }

    #[test]
    fn pane_ref_focused_sentinel() {
        let p = PaneRef::focused();
        assert!(p.is_focused());
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v, serde_json::json!("focused"));
    }

    #[test]
    fn pane_ref_round_trip_integer() {
        let parsed: PaneRef = serde_json::from_value(serde_json::json!(7)).unwrap();
        assert_eq!(parsed, PaneRef::Index(7));
    }

    #[test]
    fn pane_ref_round_trip_string() {
        let parsed: PaneRef = serde_json::from_value(serde_json::json!("abc")).unwrap();
        assert_eq!(parsed, PaneRef::Id("abc".into()));
    }

    #[test]
    fn split_direction_serializes_snake_case() {
        let v = serde_json::to_value(SplitDirection::Horizontal).unwrap();
        assert_eq!(v, serde_json::json!("horizontal"));
    }

    #[test]
    fn error_code_serializes_snake_case() {
        let v = serde_json::to_value(ErrorCode::NoKiriWindow).unwrap();
        assert_eq!(v, serde_json::json!("no_kiri_window"));
    }

    #[test]
    fn pane_color_serializes_snake_case() {
        let v = serde_json::to_value(PaneColor::Sky).unwrap();
        assert_eq!(v, serde_json::Value::String("sky".into()));
        let v = serde_json::to_value(PaneColor::Coral).unwrap();
        assert_eq!(v, serde_json::Value::String("coral".into()));
    }

    #[test]
    fn pane_color_deserializes_known() {
        let c: PaneColor = serde_json::from_value(serde_json::json!("amber")).unwrap();
        assert_eq!(c, PaneColor::Amber);
    }

    #[test]
    fn pane_color_rejects_unknown() {
        let r: Result<PaneColor, _> = serde_json::from_value(serde_json::json!("magenta"));
        assert!(r.is_err());
    }
}
