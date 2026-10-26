//! Keep a pager operation running if the user leaves the app or the screen
//! would auto-lock. iOS suspends a backgrounded app almost at once, which
//! would cut a write off mid-transfer; a background task buys roughly 30s,
//! enough for the write (~5s) and the pager's restart (~10s). Auto-lock is
//! disabled meanwhile since locking is the likeliest way to leave the app.
//! A no-op on desktop.

use tauri::AppHandle;

/// Held for the length of a backup or write; released on drop.
pub struct KeepAlive {
    #[cfg(target_os = "ios")]
    app: AppHandle,
}

#[cfg(not(target_os = "ios"))]
pub fn hold(_app: &AppHandle) -> KeepAlive {
    KeepAlive {}
}

#[cfg(target_os = "ios")]
pub use ios::hold;

#[cfg(target_os = "ios")]
mod ios {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_ui_kit::{UIApplication, UIBackgroundTaskInvalid};
    use tauri::AppHandle;

    use super::KeepAlive;

    /// The outstanding background task, or `UIBackgroundTaskInvalid` (which
    /// UIKit defines as 0; a static initializer can't read the extern).
    static TASK: AtomicUsize = AtomicUsize::new(0);

    fn invalid() -> usize {
        // SAFETY: an immutable UIKit constant.
        unsafe { UIBackgroundTaskInvalid }
    }

    fn end_task(ui: &UIApplication) {
        let id = TASK.swap(invalid(), Ordering::SeqCst);
        if id != invalid() {
            ui.endBackgroundTask(id);
        }
    }

    /// Run `f` on the main thread (UIKit requires it) and wait for it.
    fn on_main(app: &AppHandle, f: impl FnOnce(&UIApplication) + Send + 'static) {
        if let Some(mtm) = MainThreadMarker::new() {
            return f(&UIApplication::sharedApplication(mtm));
        }
        let (tx, rx) = mpsc::channel();
        let queued = app.run_on_main_thread(move || {
            if let Some(mtm) = MainThreadMarker::new() {
                f(&UIApplication::sharedApplication(mtm));
            }
            let _ = tx.send(());
        });
        if queued.is_ok() {
            let _ = rx.recv();
        }
    }

    pub fn hold(app: &AppHandle) -> KeepAlive {
        on_main(app, |ui| {
            ui.setIdleTimerDisabled(true);
            if TASK.load(Ordering::SeqCst) == invalid() {
                // iOS calls this on the main thread when the time is up; the
                // task must be ended or the app is killed outright.
                let expired = RcBlock::new(|| {
                    if let Some(mtm) = MainThreadMarker::new() {
                        end_task(&UIApplication::sharedApplication(mtm));
                    }
                });
                TASK.store(ui.beginBackgroundTaskWithExpirationHandler(Some(&expired)), Ordering::SeqCst);
            }
        });
        KeepAlive { app: app.clone() }
    }

    impl Drop for KeepAlive {
        fn drop(&mut self) {
            on_main(&self.app, |ui| {
                ui.setIdleTimerDisabled(false);
                end_task(ui);
            });
        }
    }
}
