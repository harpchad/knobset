//! Pager operations shared by the `g5` CLI and the desktop app.

pub mod backup;
pub mod clone;
pub mod device;
pub mod write;

use serde::Serialize;

/// Progress report for long operations.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub message: String,
    /// Units done and total (bytes or files); both 0 when indeterminate.
    pub done: usize,
    pub total: usize,
}

impl Progress {
    pub fn step(message: impl Into<String>) -> Self {
        Self { message: message.into(), done: 0, total: 0 }
    }

    pub fn of(message: impl Into<String>, done: usize, total: usize) -> Self {
        Self { message: message.into(), done, total }
    }
}
