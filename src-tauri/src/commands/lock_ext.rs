//! Lock-recovery helpers.
//!
//! `std::sync::Mutex::lock()` returns `Err(PoisonError)` if the lock was
//! held by a thread that panicked. The widespread `.expect("…poisoned")`
//! pattern across `cli_server`, `terminal`, and `terminal_commands` turns
//! one panic into a chain of secondary panics in every subsequent caller.
//!
//! The data we protect here is either:
//!
//! - **Cache-like** (process snapshots, ring buffers) — recovering and
//!   continuing is strictly better than panicking, because the next write
//!   refreshes the state anyway.
//! - **Registry-like** (window/CLI handle maps, signal tables) — the
//!   inner `HashMap` / `Vec` is a valid Rust value even after a panic
//!   mid-mutation; missing an entry is bounded fallout compared to
//!   crashing the process.
//!
//! [`LockExt::lock_recover`] does the standard `unwrap_or_else(|e|
//! e.into_inner())` dance and logs a single `warn!` when recovery
//! actually happens, so the operator still sees the poisoning event in
//! `~/.kiri/logs`.

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Extension trait for [`Mutex`]/[`RwLock`] that returns the inner guard
/// even when the lock was poisoned, logging the recovery for visibility.
pub trait LockExt<T: ?Sized> {
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> LockExt<T> for Mutex<T> {
    #[inline]
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(g) => g,
            Err(p) => recover(p),
        }
    }
}

/// Extension trait for [`RwLock`] that returns the inner guard even on
/// poison.
pub trait RwLockExt<T: ?Sized> {
    fn read_recover(&self) -> RwLockReadGuard<'_, T>;
    fn write_recover(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> RwLockExt<T> for RwLock<T> {
    #[inline]
    fn read_recover(&self) -> RwLockReadGuard<'_, T> {
        match self.read() {
            Ok(g) => g,
            Err(p) => {
                log::warn!(
                    "recovering poisoned RwLock (read) at {}:{}; previous holder panicked",
                    file!(),
                    line!()
                );
                p.into_inner()
            }
        }
    }

    #[inline]
    fn write_recover(&self) -> RwLockWriteGuard<'_, T> {
        match self.write() {
            Ok(g) => g,
            Err(p) => {
                log::warn!(
                    "recovering poisoned RwLock (write) at {}:{}; previous holder panicked",
                    file!(),
                    line!()
                );
                p.into_inner()
            }
        }
    }
}

#[inline]
fn recover<G>(p: PoisonError<G>) -> G {
    log::warn!("recovering poisoned Mutex; previous holder panicked");
    p.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn lock_recover_returns_inner_on_clean_lock() {
        let m = Mutex::new(42);
        let g = m.lock_recover();
        assert_eq!(*g, 42);
    }

    #[test]
    fn lock_recover_returns_inner_after_poison() {
        let m = Arc::new(Mutex::new(vec![1, 2, 3]));
        let m2 = m.clone();

        // Poison the mutex by panicking while holding it.
        let _ = std::thread::spawn(move || {
            let _g = m2.lock().unwrap();
            panic!("intentional poison");
        })
        .join();

        // The inner Vec is still a valid value; lock_recover must hand it
        // back without panicking.
        let g = m.lock_recover();
        assert_eq!(*g, vec![1, 2, 3]);
    }

    #[test]
    fn rwlock_write_recover_returns_inner_after_poison() {
        let l = Arc::new(RwLock::new(String::from("hello")));
        let l2 = l.clone();
        let _ = std::thread::spawn(move || {
            let _g = l2.write().unwrap();
            panic!("intentional poison");
        })
        .join();

        let mut g = l.write_recover();
        assert_eq!(*g, "hello");
        g.push_str(", world");
        assert_eq!(*g, "hello, world");
    }

    #[test]
    fn rwlock_read_recover_returns_inner_after_poison() {
        let l = Arc::new(RwLock::new(7_u32));
        let l2 = l.clone();
        let _ = std::thread::spawn(move || {
            let _g = l2.write().unwrap();
            panic!("intentional poison");
        })
        .join();

        let g = l.read_recover();
        assert_eq!(*g, 7);
    }
}
