//! Maps logical pane references (index, id, "focused") to physical
//! terminal IDs for one window.
//!
//! The frontend pushes the current layout via the `cli_update_pane_map`
//! Tauri command whenever panes are created, focused, split, or closed.
//! Handlers then read this map to translate `PaneRef` into a concrete
//! `terminal_id` they can act on.

use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneEntry {
    pub index: u32,
    pub pane_id: String,
    pub terminal_id: u32,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<kiri_cli_proto::PaneColor>,
}

#[derive(Default)]
pub struct PaneMap {
    inner: Mutex<BTreeMap<u32, PaneEntry>>,
}

impl PaneMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the full map with `entries`. Entries are keyed by
    /// `entry.index`. Duplicate indices keep the last one inserted.
    pub fn replace(&self, entries: Vec<PaneEntry>) {
        let mut map = self.inner.lock().expect("pane_map mutex poisoned");
        map.clear();
        for entry in entries {
            map.insert(entry.index, entry);
        }
    }

    /// Snapshot of all entries, ordered by `index`.
    pub fn snapshot(&self) -> Vec<PaneEntry> {
        let map = self.inner.lock().expect("pane_map mutex poisoned");
        map.values().cloned().collect()
    }

    pub fn resolve(&self, r: &kiri_cli_proto::PaneRef) -> Option<PaneEntry> {
        let map = self.inner.lock().expect("pane_map mutex poisoned");
        match r {
            kiri_cli_proto::PaneRef::Index(i) => map.get(i).cloned(),
            kiri_cli_proto::PaneRef::Id(s)
                if s == kiri_cli_proto::PaneRef::FOCUSED_SENTINEL =>
            {
                map.values().find(|e| e.focused).cloned()
            }
            kiri_cli_proto::PaneRef::Id(s) => {
                map.values().find(|e| &e.pane_id == s).cloned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiri_cli_proto::PaneRef;

    fn entry(index: u32, pane_id: &str, terminal_id: u32, focused: bool) -> PaneEntry {
        PaneEntry {
            index,
            pane_id: pane_id.to_string(),
            terminal_id,
            focused,
            name: None,
            color: None,
        }
    }

    #[test]
    fn replace_then_snapshot_round_trip() {
        let pm = PaneMap::new();
        pm.replace(vec![
            entry(0, "p-a", 10, true),
            entry(1, "p-b", 11, false),
        ]);
        let snap = pm.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].pane_id, "p-a");
        assert_eq!(snap[1].pane_id, "p-b");
    }

    #[test]
    fn resolve_by_index() {
        let pm = PaneMap::new();
        pm.replace(vec![entry(0, "p-a", 10, true), entry(1, "p-b", 11, false)]);
        let got = pm.resolve(&PaneRef::Index(1)).unwrap();
        assert_eq!(got.terminal_id, 11);
    }

    #[test]
    fn resolve_by_pane_id() {
        let pm = PaneMap::new();
        pm.replace(vec![entry(0, "p-a", 10, true), entry(1, "p-b", 11, false)]);
        let got = pm.resolve(&PaneRef::Id("p-b".into())).unwrap();
        assert_eq!(got.terminal_id, 11);
    }

    #[test]
    fn resolve_focused_returns_focused_entry() {
        let pm = PaneMap::new();
        pm.replace(vec![entry(0, "p-a", 10, false), entry(1, "p-b", 11, true)]);
        let got = pm.resolve(&PaneRef::focused()).unwrap();
        assert_eq!(got.terminal_id, 11);
        assert!(got.focused);
    }

    #[test]
    fn resolve_unknown_returns_none() {
        let pm = PaneMap::new();
        pm.replace(vec![entry(0, "p-a", 10, true)]);
        assert!(pm.resolve(&PaneRef::Index(99)).is_none());
        assert!(pm.resolve(&PaneRef::Id("nope".into())).is_none());
    }

    #[test]
    fn entry_with_name_color_roundtrips() {
        let e = PaneEntry {
            index: 0,
            pane_id: "pane-1".into(),
            terminal_id: 1,
            focused: true,
            name: Some("build".into()),
            color: Some(kiri_cli_proto::PaneColor::Coral),
        };
        let s = serde_json::to_string(&e).unwrap();
        let back: PaneEntry = serde_json::from_str(&s).unwrap();
        assert_eq!(back.name.as_deref(), Some("build"));
        assert_eq!(back.color, Some(kiri_cli_proto::PaneColor::Coral));
    }

    #[test]
    fn entry_without_label_omits_fields_in_json() {
        let e = entry(0, "pane-1", 1, true);
        let v = serde_json::to_value(&e).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }
}
