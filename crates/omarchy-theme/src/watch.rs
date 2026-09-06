//! Push-style theme-change watching: one background thread holding an
//! inotify watch on the `current/` state directory (the parent that survives
//! the atomic swap), debounced so a whole theme switch surfaces as exactly
//! one callback.
//!
//! Apps that render continuously should prefer the poll tier
//! ([`crate::Theme::changed`]) — zero threads. This module is for event-driven
//! apps (retained-mode GUIs, daemons).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::{EventKind, RecommendedWatcher, Watcher};

use crate::theme::{current_state_dir, Theme};
use crate::{Error, Result};

/// Coalescing window for one swap's event burst.
const DEBOUNCE: Duration = Duration::from_millis(120);
/// How often the watcher thread checks its stop flag between events.
const TICK: Duration = Duration::from_millis(200);

/// A running theme watcher. Call [`ThemeWatcher::stop`] (or drop) to end the
/// background thread.
pub struct ThemeWatcher {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    /// Held to keep the inotify watch alive; never read directly.
    #[allow(dead_code)]
    watcher: Option<RecommendedWatcher>,
}

impl ThemeWatcher {
    /// Watch the live theme. `on_change` runs on the watcher thread: once
    /// immediately with the current theme, then again whenever a completed
    /// theme switch settles. Return false from the callback to stop.
    pub fn spawn(on_change: impl FnMut(&Theme) -> bool + Send + 'static) -> Result<ThemeWatcher> {
        let dir = current_state_dir()?;
        Self::spawn_state_dir(&dir, on_change)
    }

    /// Watch an explicit Omarchy *state* directory (the one containing
    /// `theme/` and `theme.name`) — the same path a live session stages at.
    /// Public for tests, sandboxes, and multi-instance previews.
    pub fn spawn_state_dir(
        dir: &std::path::Path,
        mut on_change: impl FnMut(&Theme) -> bool + Send + 'static,
    ) -> Result<ThemeWatcher> {
        if !dir.is_dir() {
            return Err(Error::NoTheme);
        }
        let theme = Theme::from_state_dir(dir)?;

        let (tx, rx) = mpsc::channel::<()>();
        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
                let Ok(ev) = ev else { return };
                if !matches!(ev.kind, EventKind::Access(_)) {
                    let _ = tx.send(());
                }
            })
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        watcher
            .watch(dir, notify::RecursiveMode::NonRecursive)
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;

        if !on_change(&theme) {
            return Ok(ThemeWatcher {
                stop: Arc::new(AtomicBool::new(true)),
                handle: None,
                watcher: Some(watcher),
            });
        }

        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let dir = dir.to_path_buf();
        let handle = thread::spawn(move || {
            let mut theme = theme;
            let mut last_event: Option<Instant> = None;
            loop {
                match rx.recv_timeout(TICK) {
                    Ok(()) => {
                        last_event = Some(Instant::now());
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                    Err(RecvTimeoutError::Timeout) => {}
                }
                if stop_thread.load(Ordering::Relaxed) {
                    return;
                }
                let Some(at) = last_event else { continue };
                if at.elapsed() < DEBOUNCE {
                    continue;
                }
                last_event = None;
                let signature = theme.signature().clone();
                match Theme::from_state_dir(&dir) {
                    Ok(fresh) => {
                        if fresh.signature() != &signature {
                            theme = fresh;
                            if !on_change(&theme) {
                                return;
                            }
                        }
                    }
                    // swap in flight (directory momentarily gone): back off;
                    // pending events will re-trigger.
                    Err(_) => {
                        last_event = Some(Instant::now());
                    }
                }
            }
        });

        Ok(ThemeWatcher {
            stop,
            handle: Some(handle),
            watcher: Some(watcher),
        })
    }

    /// Signal the thread to exit and join it (bounded by one tick).
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ThemeWatcher {
    fn drop(&mut self) {
        self.shutdown();
    }
}
