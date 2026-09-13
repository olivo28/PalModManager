use std::io::BufRead;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use crate::save_scanner::SaveScanProgressPayload;
use crate::worker::protocol::{WorkerMessage, WorkerTask};

/// Spawns a headless child worker to execute `task` in an isolated process.
/// Streams progress events to the frontend and parses the resulting payload.
/// Falls back to in-process `spawn_blocking` if process creation is unavailable.
pub async fn run_worker_task<T: serde::de::DeserializeOwned + Send + 'static>(
    app: Option<Arc<AppHandle>>,
    task: WorkerTask,
    event_name: &'static str,
) -> Result<T, String> {
    let task_clone = task.clone();
    let app_clone = app.clone();

    // 1. Attempt isolated subprocess execution
    let worker_res = tauri::async_runtime::spawn_blocking(move || {
        execute_in_subprocess(app_clone, task_clone, event_name)
    })
    .await
    .map_err(|e| format!("Async runtime join error: {e}"))?;

    match worker_res {
        Ok(json_payload) => {
            serde_json::from_str::<T>(&json_payload)
                .map_err(|e| format!("Failed to parse worker output: {e}"))
        }
        Err(spawn_or_exec_err) => {
            crate::logger::log(&format!(
                "[PMM-WORKER] Isolated subprocess error: {spawn_or_exec_err}. Attempting in-process fallback."
            ));
            // 2. Graceful in-process fallback
            execute_in_process_fallback(app, task).await
        }
    }
}

fn execute_in_subprocess(
    app: Option<Arc<AppHandle>>,
    task: WorkerTask,
    event_name: &'static str,
) -> Result<String, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Unable to locate current executable path: {e}"))?;

    let temp_dir = std::env::temp_dir().join("PalModManager_workers");
    let _ = std::fs::create_dir_all(&temp_dir);

    let task_id = uuid::Uuid::new_v4().to_string();
    let task_file = temp_dir.join(format!("task_{task_id}.json"));
    let task_json = serde_json::to_string(&task)
        .map_err(|e| format!("Failed to serialize task: {e}"))?;

    std::fs::write(&task_file, &task_json)
        .map_err(|e| format!("Failed to write task payload to temp file: {e}"))?;

    let mut cmd = std::process::Command::new(&current_exe);
    cmd.arg("--pmm-worker")
       .arg("--task-file")
       .arg(&task_file)
       .stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());

    // Configure low priority and suppress window on Windows
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }

    crate::logger::log(&format!(
        "[PMM-WORKER] Spawning isolated child worker: {} --pmm-worker",
        current_exe.display()
    ));

    let mut child = cmd.spawn().map_err(|e| {
        let _ = std::fs::remove_file(&task_file);
        format!("Failed to spawn child worker process: {e}")
    })?;

    let stdout = child.stdout.take()
        .ok_or_else(|| "Failed to capture child worker stdout pipe".to_string())?;

    let reader = std::io::BufReader::new(stdout);
    let mut final_payload: Option<String> = None;
    let mut last_error: Option<String> = None;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        match WorkerMessage::from_line(&line) {
            Ok(WorkerMessage::Progress { stage, percent }) => {
                if let Some(ref h) = app {
                    let _ = h.emit(event_name, SaveScanProgressPayload {
                        stage,
                        percent,
                    });
                }
            }
            Ok(WorkerMessage::Success { payload_json }) => {
                final_payload = Some(payload_json);
            }
            Ok(WorkerMessage::Error { message }) => {
                last_error = Some(message);
            }
            Err(_) => {
                // Non-NDJSON diagnostic output from the child
                crate::logger::log(&format!("[PMM-WORKER-RAW] {line}"));
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for worker process: {e}"))?;

    // Best-effort cleanup in case child didn't delete it
    let _ = std::fs::remove_file(&task_file);

    if !status.success() {
        let exit_desc = match status.code() {
            Some(code) => format!("code {code}"),
            None => "terminated by signal / crash".to_string(),
        };

        if let Some(err_msg) = last_error {
            return Err(format!("Worker error ({exit_desc}): {err_msg}"));
        }

        return Err(format!(
            "Worker process terminated abnormally ({exit_desc}). The file may be critically corrupt or exceeded memory limits."
        ));
    }

    final_payload.ok_or_else(|| "Worker finished with exit code 0 but emitted no result payload".to_string())
}

async fn execute_in_process_fallback<T: serde::de::DeserializeOwned + Send + 'static>(
    app: Option<Arc<AppHandle>>,
    task: WorkerTask,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || match task {
        WorkerTask::SaveDeepScan {
            world_dir,
            active_mod_names,
            program_path,
        } => {
            let report = crate::save_scanner::deep_scan_save(
                &world_dir,
                &active_mod_names,
                program_path.as_deref(),
                app,
                None,
            )?;
            let json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
            serde_json::from_str::<T>(&json).map_err(|e| e.to_string())
        }
        WorkerTask::SaveRepair {
            world_dir,
            program_path,
        } => {
            let res = crate::save_scanner::repair_and_sanitize_save(
                &world_dir,
                &program_path,
                app,
                None,
            )?;
            let json = serde_json::to_string(&res).map_err(|e| e.to_string())?;
            serde_json::from_str::<T>(&json).map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| format!("Fallback join error: {e}"))?
}
