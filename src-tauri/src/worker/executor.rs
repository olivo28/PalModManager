use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use crate::worker::protocol::{WorkerMessage, WorkerTask};

/// Run the headless worker execution loop.
/// Reads task payload, executes requested routine with progress streaming, and exits with code 0 on success.
pub fn execute_worker_cli(args: &[String]) -> i32 {
    let mut task_file: Option<PathBuf> = None;
    let mut use_stdin = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--task-file" => {
                if i + 1 < args.len() {
                    task_file = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--stdin" => {
                use_stdin = true;
            }
            _ => {}
        }
        i += 1;
    }

    let task: WorkerTask = if let Some(ref path) = task_file {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Best-effort cleanup of temporary task payload file
                let _ = std::fs::remove_file(path);
                match serde_json::from_str(&content) {
                    Ok(t) => t,
                    Err(e) => {
                        emit_error(&format!("Failed to parse task JSON from file: {e}"));
                        return 1;
                    }
                }
            }
            Err(e) => {
                emit_error(&format!("Failed to read task file '{}': {e}", path.display()));
                return 1;
            }
        }
    } else if use_stdin {
        use std::io::Read;
        let mut buffer = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut buffer) {
            emit_error(&format!("Failed to read task payload from stdin: {e}"));
            return 1;
        }
        match serde_json::from_str(&buffer) {
            Ok(t) => t,
            Err(e) => {
                emit_error(&format!("Failed to parse task JSON from stdin: {e}"));
                return 1;
            }
        }
    } else {
        emit_error("Missing required --task-file <path> or --stdin argument for worker");
        return 1;
    };

    let progress_cb = Arc::new(|stage: &str, percent: u8| {
        let msg = WorkerMessage::Progress {
            stage: stage.to_string(),
            percent,
        };
        emit_message(&msg);
    });

    // Run execution with panic isolation
    let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match task {
        WorkerTask::SaveDeepScan {
            world_dir,
            active_mod_names,
            program_path,
        } => match crate::save_scanner::deep_scan_save(
            &world_dir,
            &active_mod_names,
            program_path.as_deref(),
            None,
            Some(progress_cb),
        ) {
            Ok(report) => match serde_json::to_string(&report) {
                Ok(json) => {
                    emit_message(&WorkerMessage::Success { payload_json: json });
                    0
                }
                Err(e) => {
                    emit_error(&format!("Failed to serialize report: {e}"));
                    1
                }
            },
            Err(e) => {
                emit_error(&e);
                1
            }
        },
        WorkerTask::SaveRepair {
            world_dir,
            program_path,
        } => match crate::save_scanner::repair_and_sanitize_save(
            &world_dir,
            &program_path,
            None,
            Some(progress_cb),
        ) {
            Ok(res) => match serde_json::to_string(&res) {
                Ok(json) => {
                    emit_message(&WorkerMessage::Success { payload_json: json });
                    0
                }
                Err(e) => {
                    emit_error(&format!("Failed to serialize repair result: {e}"));
                    1
                }
            },
            Err(e) => {
                emit_error(&e);
                1
            }
        },
    }));

    match run_res {
        Ok(code) => code,
        Err(panic_payload) => {
            let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                format!("Worker thread panicked: {s}")
            } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                format!("Worker thread panicked: {s}")
            } else {
                "Worker thread panicked with unknown payload".to_string()
            };
            emit_error(&msg);
            1
        }
    }
}

fn emit_message(msg: &WorkerMessage) {
    if let Ok(line) = msg.to_ndjson() {
        let mut out = std::io::stdout();
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
    }
}

fn emit_error(err_str: &str) {
    emit_message(&WorkerMessage::Error {
        message: err_str.to_string(),
    });
}
