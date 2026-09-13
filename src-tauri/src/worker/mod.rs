pub mod protocol;
pub mod executor;
pub mod spawner;

pub use protocol::{WorkerMessage, WorkerTask};
pub use spawner::run_worker_task;

/// Inspects command-line arguments at early process startup.
/// If `--pmm-worker` is present, enters headless worker execution mode,
/// bypassing all GUI, WebView, and Tauri window initialization.
/// Returns `Some(exit_code)` to exit, or `None` to proceed with the normal Tauri desktop runtime.
pub fn try_handle_worker_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--pmm-worker") {
        let code = executor::execute_worker_cli(&args);
        Some(code)
    } else {
        None
    }
}
