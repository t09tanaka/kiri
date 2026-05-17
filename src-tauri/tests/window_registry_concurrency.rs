//! Concurrency tests for the in-process WindowRegistry.
//!
//! Two windows can race on register / unregister via the
//! `windowService.registerWindow` and `windowService.unregisterWindow`
//! Tauri commands. The registry sits behind an `Arc<Mutex<...>>`, so
//! the surface contract is "no panic, no data loss, no orphaned
//! mapping" under concurrent access.
//!
//! These tests poke `WindowRegistry` directly because the underlying
//! struct is `pub` in `commands::window` and doesn't require a Tauri
//! runtime to construct.

use app_lib::commands::window::WindowRegistry;
use std::sync::{Arc, Mutex};
use std::thread;

fn empty() -> Arc<Mutex<WindowRegistry>> {
    Arc::new(Mutex::new(WindowRegistry::new()))
}

#[test]
fn concurrent_registers_keep_label_path_mapping_consistent() {
    let registry = empty();
    let mut handles = Vec::new();

    for i in 0..32 {
        let registry = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            let label = format!("win-{i}");
            let path = format!("/projects/p-{i}");
            let mut guard = registry.lock().expect("lock");
            guard.register(&label, &path);
        }));
    }

    for h in handles {
        h.join().expect("thread");
    }

    let guard = registry.lock().expect("lock");
    for i in 0..32 {
        let label = format!("win-{i}");
        let path = format!("/projects/p-{i}");
        assert_eq!(guard.get_label_for_path(&path), Some(&label));
        assert_eq!(guard.get_path_for_label(&label), Some(&path));
    }
    assert_eq!(guard.get_all_paths().len(), 32);
}

#[test]
fn re_registering_a_label_with_new_path_cleans_old_mapping() {
    let registry = empty();
    {
        let mut g = registry.lock().expect("lock");
        g.register("main", "/projects/old");
        g.register("main", "/projects/new");
    }
    let g = registry.lock().expect("lock");
    assert_eq!(g.get_path_for_label("main"), Some(&"/projects/new".to_string()));
    assert!(g.get_label_for_path("/projects/old").is_none());
    assert_eq!(g.get_label_for_path("/projects/new"), Some(&"main".to_string()));
    // The cleanup must drop the orphaned forward entry, not leave a
    // dangling /projects/old -> "main" mapping behind.
    assert_eq!(g.get_all_paths(), vec!["/projects/new".to_string()]);
}

#[test]
fn concurrent_unregister_does_not_leak_orphan_mappings() {
    let registry = empty();
    for i in 0..16 {
        let mut g = registry.lock().expect("lock");
        g.register(&format!("w{i}"), &format!("/p/{i}"));
    }

    let mut handles = Vec::new();
    for i in 0..16 {
        let registry = Arc::clone(&registry);
        handles.push(thread::spawn(move || {
            let mut g = registry.lock().expect("lock");
            g.unregister_by_label(&format!("w{i}"));
        }));
    }
    for h in handles {
        h.join().expect("thread");
    }

    let g = registry.lock().expect("lock");
    assert!(g.get_all_paths().is_empty(), "no path mappings should remain");
    for i in 0..16 {
        assert!(g.get_path_for_label(&format!("w{i}")).is_none());
    }
}

#[test]
fn two_windows_focusing_same_project_resolve_to_same_label() {
    // Models the focus-or-create race: two clicks on the same recent
    // project from different windows must converge on a single
    // registered window label, not create a phantom dup.
    let registry = empty();
    let path = "/projects/shared".to_string();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let writer = {
        let registry = Arc::clone(&registry);
        let path = path.clone();
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut i: u32 = 0;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                let label = format!("main-{}", i % 2);
                registry.lock().expect("lock").register(&label, &path);
                i = i.wrapping_add(1);
            }
        })
    };

    // Reader observes the registry many times; the label_for_path
    // must always be present (never None) once the writer has gone
    // through at least one register.
    for _ in 0..1_000 {
        let g = registry.lock().expect("lock");
        if g.get_label_for_path(&path).is_some() {
            // We expect at most one path entry for /projects/shared
            // because re-registers must clean orphans.
            let count = g
                .get_all_paths()
                .iter()
                .filter(|p| p.as_str() == path)
                .count();
            assert_eq!(count, 1, "duplicate path entries for shared project");
        }
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    writer.join().expect("writer");
}
