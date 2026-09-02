// Strip JSONC (JSON with comments) comments for parsing
pub fn strip_jsonc_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            output.push(c);
        } else {
            if c == '"' {
                in_string = true;
                output.push(c);
            } else if c == '/' {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '/' {
                        chars.next();
                        while let Some(nc) = chars.next() {
                            if nc == '\n' || nc == '\r' {
                                output.push(nc);
                                break;
                            }
                        }
                    } else if next_c == '*' {
                        chars.next();
                        while let Some(nc) = chars.next() {
                            if nc == '*' {
                                if let Some(&next_nc) = chars.peek() {
                                    if next_nc == '/' {
                                        chars.next();
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        output.push(c);
                    }
                } else {
                    output.push(c);
                }
            } else {
                output.push(c);
            }
        }
    }
    output
}

pub fn collect_files_with_extensions(dir: &std::path::Path, extensions: &[&str], files: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_files_with_extensions(&path, extensions, files);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        }
    }
}
