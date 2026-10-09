use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

// Extract recent error lines from UE4SS.log and Pal.log
pub fn extract_recent_errors(game_root: &Path, max_lines: usize) -> Vec<String> {
    let mut errors = Vec::new();

    // Check UE4SS log paths
    let ue4ss_paths = [
        game_root.join("Pal").join("Binaries").join("Win64").join("UE4SS.log"),
        game_root.join("ue4ss").join("UE4SS.log"),
        game_root.join("UE4SS.log"),
    ];

    for path in &ue4ss_paths {
        if path.exists() {
            if let Ok(file) = File::open(path) {
                let reader = BufReader::new(file);
                let mut captured = Vec::new();
                for line in reader.lines().flatten() {
                    let l_lower = line.to_lowercase();
                    if l_lower.contains("error") 
                        || l_lower.contains("fatal") 
                        || l_lower.contains("crash") 
                        || l_lower.contains("exception")
                        || l_lower.contains("failed to load") {
                        captured.push(line);
                    }
                }
                let len = captured.len();
                let start = if len > max_lines { len - max_lines } else { 0 };
                errors.extend(captured[start..].to_vec());
                break;
            }
        }
    }

    errors
}
