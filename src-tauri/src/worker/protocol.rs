use serde::{Deserialize, Serialize};

/// Task dispatched from the parent process to the headless worker process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum WorkerTask {
    SaveDeepScan {
        world_dir: String,
        active_mod_names: Vec<String>,
        program_path: Option<String>,
    },
    SaveRepair {
        world_dir: String,
        program_path: String,
    },
}

/// NDJSON message streamed from the worker process back to the parent process over stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkerMessage {
    Progress {
        stage: String,
        percent: u8,
    },
    Success {
        payload_json: String,
    },
    Error {
        message: String,
    },
}

impl WorkerMessage {
    /// Format message as a single-line JSON string followed by a newline for NDJSON streaming.
    pub fn to_ndjson(&self) -> Result<String, serde_json::Error> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }

    /// Parse a single NDJSON line.
    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_serialization_roundtrip() {
        let task = WorkerTask::SaveDeepScan {
            world_dir: "C:/Palworld/Save".to_string(),
            active_mod_names: vec!["ModA".to_string(), "ModB".to_string()],
            program_path: Some("C:/ProgramData".to_string()),
        };
        let json = serde_json::to_string(&task).expect("serialize task");
        let parsed: WorkerTask = serde_json::from_str(&json).expect("deserialize task");
        match parsed {
            WorkerTask::SaveDeepScan { world_dir, active_mod_names, program_path } => {
                assert_eq!(world_dir, "C:/Palworld/Save");
                assert_eq!(active_mod_names.len(), 2);
                assert_eq!(program_path.as_deref(), Some("C:/ProgramData"));
            }
            _ => panic!("Expected SaveDeepScan task"),
        }
    }

    #[test]
    fn test_message_ndjson_roundtrip() {
        let msg = WorkerMessage::Progress {
            stage: "decompressing".to_string(),
            percent: 45,
        };
        let ndjson = msg.to_ndjson().expect("to ndjson");
        assert!(ndjson.ends_with('\n'));
        let parsed = WorkerMessage::from_line(&ndjson).expect("from line");
        assert_eq!(msg, parsed);

        let err_msg = WorkerMessage::Error {
            message: "Failed to open save file".to_string(),
        };
        let err_ndjson = err_msg.to_ndjson().expect("to ndjson");
        let parsed_err = WorkerMessage::from_line(&err_ndjson).expect("from line");
        assert_eq!(err_msg, parsed_err);
    }
}
