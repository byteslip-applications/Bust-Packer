//! task.rs — Streamlined concurrency message schemas and thread channels.

/// Operational jobs dispatched downstream to the worker thread thread
pub enum Job {
    Pack {
        path: String,
        excludes: Vec<String>,
        for_ai: bool,
    },
    Unpack(String),
}

/// Progress metrics returned back up to the primary user interface thread
#[derive(Debug)]
pub enum TaskMsg {
    Progress {
        files_done: usize,
        files_total: usize,
    },
    PackDone {
        files: usize,
        output: String,
        log_append: String,
    },
    UnpackDone {
        output: String,
        log_append: String,
    },
    Error(String),
}
