use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkClassInfo {
    pub name: String,
    pub clean_name: String,
    pub super_class: Option<String>,
    pub clean_super_class: Option<String>,
    pub functions: HashSet<String>,
    pub properties: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkIndex {
    pub classes: HashMap<String, SdkClassInfo>,
    pub total_classes: usize,
    pub total_functions: usize,
    pub source: String,
    pub game_version: String,
}

impl SdkIndex {
    pub fn new(source: String, game_version: String) -> Self {
        Self {
            classes: HashMap::new(),
            total_classes: 0,
            total_functions: 0,
            source,
            game_version,
        }
    }

    pub fn find_class(&self, query: &str) -> Option<&SdkClassInfo> {
        let clean = clean_unreal_name(query);
        let lower = clean.to_ascii_lowercase();
        
        if let Some(c) = self.classes.get(&lower) {
            return Some(c);
        }
        
        let exact_lower = query.trim().to_ascii_lowercase();
        self.classes.get(&exact_lower)
    }

    /// Recursively check if a function/delegate exists on the class or any of its super classes
    pub fn has_function(&self, class_name: &str, func_name: &str) -> bool {
        let clean_func = func_name.trim();
        if clean_func.is_empty() {
            return false;
        }

        let mut current_class_name = clean_unreal_name(class_name);
        let mut depth = 0;

        while depth < 20 {
            let Some(class_info) = self.find_class(&current_class_name) else {
                break;
            };

            // Check direct functions on this class
            if class_info.functions.iter().any(|f| f.eq_ignore_ascii_case(clean_func)) {
                return true;
            }

            // Check direct properties on this class (e.g. delegate properties)
            if class_info.properties.iter().any(|p| p.eq_ignore_ascii_case(clean_func)) {
                return true;
            }

            // Traverse to parent class
            if let Some(ref super_cls) = class_info.clean_super_class {
                if super_cls.is_empty() || super_cls.eq_ignore_ascii_case("Object") || super_cls.eq_ignore_ascii_case("UObject") {
                    break;
                }
                current_class_name = super_cls.clone();
                depth += 1;
            } else {
                break;
            }
        }

        // Standard Unreal Engine virtual lifecycle methods not emitted by CXXHeaderDump UFUNCTION macros
        let lower_func = clean_func.to_ascii_lowercase();
        let lower_cls = clean_unreal_name(class_name).to_ascii_lowercase();

        // PlayerController virtual methods
        if lower_cls == "playercontroller" || lower_cls == "palplayercontroller" {
            if lower_func == "playertick" || lower_func == "setupinputcomponent" || lower_func == "updaterotation" || lower_func == "postprocessinput" {
                return true;
            }
        }

        // Actor / Pawn / Character lifecycle virtual methods
        if lower_func == "tick"
            || lower_func == "receivetick"
            || lower_func == "beginplay"
            || lower_func == "receivebeginplay"
            || lower_func == "endplay"
            || lower_func == "receiveendplay"
            || lower_func == "onconstruction"
            || lower_func == "userconstructionscript"
            || lower_func == "postinitializecomponents"
            || lower_func == "preinitializecomponents"
            || lower_func == "destroyed"
            || lower_func == "reset"
        {
            return true;
        }

        false
    }

    /// Collect all known functions, properties, and delegates for a class and its complete inheritance tree
    pub fn get_all_class_functions_and_properties(&self, class_name: &str) -> HashSet<String> {
        let mut all_members = HashSet::new();
        let mut current_class_name = clean_unreal_name(class_name);
        let mut depth = 0;

        while depth < 20 {
            let Some(class_info) = self.find_class(&current_class_name) else {
                break;
            };

            for f in &class_info.functions {
                all_members.insert(f.clone());
            }
            for p in &class_info.properties {
                all_members.insert(p.clone());
            }

            if let Some(ref super_cls) = class_info.clean_super_class {
                if super_cls.is_empty() || super_cls.eq_ignore_ascii_case("Object") || super_cls.eq_ignore_ascii_case("UObject") {
                    break;
                }
                current_class_name = super_cls.clone();
                depth += 1;
            } else {
                break;
            }
        }
        all_members
    }

    /// Fuzzy search closest function, delegate signature, or property on the class inheritance hierarchy
    pub fn suggest_similar_function(&self, class_name: &str, target_func: &str) -> Option<String> {
        let target = target_func.trim();
        if target.is_empty() {
            return None;
        }

        let all_members = self.get_all_class_functions_and_properties(class_name);
        if all_members.is_empty() {
            return None;
        }

        let target_norm = target.replace('_', "").to_ascii_lowercase();
        let mut best_candidate: Option<String> = None;
        let mut highest_score: f64 = 0.0;

        for cand in all_members {
            let cand_norm = cand.replace('_', "").to_ascii_lowercase();

            // Direct normalized match (e.g. difference only in underscores, delegate formatting, or casing)
            if target_norm == cand_norm {
                return Some(cand);
            }

            let max_len = target_norm.len().max(cand_norm.len());
            if max_len == 0 {
                continue;
            }

            let dist = levenshtein_distance(&target_norm, &cand_norm);
            let score = 1.0 - (dist as f64 / max_len as f64);

            // Give a boost if one is substring of the other (e.g. DelegateSignatures vs DelegateSignature)
            let boost = if target_norm.contains(&cand_norm) || cand_norm.contains(&target_norm) {
                0.15
            } else {
                0.0
            };
            let effective_score = (score + boost).min(1.0);

            if effective_score > highest_score && effective_score >= 0.55 {
                highest_score = effective_score;
                best_candidate = Some(cand);
            }
        }

        best_candidate
    }

    /// Fuzzy search closest class name in the entire SDK index
    pub fn suggest_similar_class(&self, target_class: &str) -> Option<String> {
        let clean_target = clean_unreal_name(target_class);
        let target_norm = clean_target.replace('_', "").to_ascii_lowercase();
        if target_norm.is_empty() {
            return None;
        }

        let mut best_candidate: Option<String> = None;
        let mut highest_score: f64 = 0.0;

        for info in self.classes.values() {
            let cand_clean = &info.clean_name;
            let cand_norm = cand_clean.replace('_', "").to_ascii_lowercase();

            if target_norm == cand_norm {
                return Some(info.name.clone());
            }

            let max_len = target_norm.len().max(cand_norm.len());
            if max_len == 0 {
                continue;
            }

            let dist = levenshtein_distance(&target_norm, &cand_norm);
            let score = 1.0 - (dist as f64 / max_len as f64);

            if score > highest_score && score >= 0.65 {
                highest_score = score;
                best_candidate = Some(info.name.clone());
            }
        }

        best_candidate
    }
}

pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let m = s1_chars.len();
    let n = s2_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut dp = vec![vec![0; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}


pub fn clean_unreal_name(name: &str) -> String {
    let trimmed = name.trim();
    let without_path = if let Some(last_slash) = trimmed.rfind('/') {
        &trimmed[last_slash + 1..]
    } else {
        trimmed
    };
    let without_dot = if let Some(dot_idx) = without_path.rfind('.') {
        &without_path[dot_idx + 1..]
    } else {
        without_path
    };

    let without_suffix = if without_dot.ends_with("_C") && without_dot.len() > 2 {
        &without_dot[..without_dot.len() - 2]
    } else {
        without_dot
    };

    if without_suffix.len() > 1 {
        let first_char = without_suffix.chars().next().unwrap();
        let second_char = without_suffix.chars().nth(1).unwrap();
        if (first_char == 'A' || first_char == 'U' || first_char == 'F' || first_char == 'E') && second_char.is_ascii_uppercase() {
            return without_suffix[1..].to_string();
        }
    }

    without_suffix.to_string()
}

pub fn get_sdk_dir(program_path: &str) -> PathBuf {
    let base = if !program_path.is_empty() {
        PathBuf::from(program_path)
    } else {
        #[cfg(target_os = "windows")]
        let p = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("PalModManager");
        #[cfg(not(target_os = "windows"))]
        let p = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".local").join("share"))
            .unwrap_or_else(|_| std::env::temp_dir())
            .join("PalModManager");
        p
    };
    base.join("resources").join("sdk")
}

pub fn parse_sdk_directory(dir: &Path, source_label: &str) -> Result<SdkIndex, String> {
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("SDK directory does not exist: {:?}", dir));
    }

    let mut index = SdkIndex::new(source_label.to_string(), "v0.3.0+".to_string());
    let mut hpp_files = Vec::new();

    collect_hpp_files(dir, &mut hpp_files);

    if hpp_files.is_empty() {
        return Err(format!("No C++ header (.hpp) files found in {:?}", dir));
    }

    // Prioritize main module headers first
    hpp_files.sort_by(|a, b| {
        let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let is_primary = |name: &str| name == "Pal.hpp" || name == "Engine.hpp" || name == "CoreUObject.hpp";
        let a_pri = if is_primary(a_name) { 0 } else { 1 };
        let b_pri = if is_primary(b_name) { 0 } else { 1 };
        a_pri.cmp(&b_pri).then_with(|| a_name.cmp(b_name))
    });

    let mut total_funcs = 0;

    for hpp_path in &hpp_files {
        if let Ok(content) = fs::read_to_string(hpp_path) {
            parse_hpp_content(&content, &mut index, &mut total_funcs);
        }
    }

    index.total_classes = index.classes.len();
    index.total_functions = total_funcs;

    crate::logger::log(&format!(
        "Indexed C++ SDK ({}): {} classes, {} functions across {} headers",
        source_label, index.total_classes, index.total_functions, hpp_files.len()
    ));

    Ok(index)
}

fn collect_hpp_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_hpp_files(&path, out);
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("hpp") || ext.eq_ignore_ascii_case("h") {
                    out.push(path);
                }
            }
        }
    }
}

fn parse_hpp_content(content: &str, index: &mut SdkIndex, total_funcs: &mut usize) {
    let mut current_class: Option<SdkClassInfo> = None;
    let mut in_class = false;
    let mut brace_depth = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }

        // Detect class declaration
        // e.g. class APalPlayerState : public APlayerState
        // e.g. class UPalUtility : public UBlueprintFunctionLibrary
        // e.g. class PAL_API APalCharacter : public ACharacter
        if !in_class && trimmed.starts_with("class ") {
            if let Some(colon_pos) = trimmed.find(':') {
                let class_part = trimmed[6..colon_pos].trim();
                let super_part = trimmed[colon_pos + 1..].trim();

                let class_name = class_part
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .trim_end_matches('{')
                    .trim();

                let super_name = super_part
                    .replace("public", "")
                    .replace("protected", "")
                    .replace("private", "")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('{')
                    .trim()
                    .to_string();

                if !class_name.is_empty() {
                    let clean = clean_unreal_name(class_name);
                    let clean_super = if !super_name.is_empty() {
                        Some(clean_unreal_name(&super_name))
                    } else {
                        None
                    };

                    current_class = Some(SdkClassInfo {
                        name: class_name.to_string(),
                        clean_name: clean,
                        super_class: if !super_name.is_empty() { Some(super_name) } else { None },
                        clean_super_class: clean_super,
                        functions: HashSet::new(),
                        properties: HashSet::new(),
                    });
                    in_class = true;
                    brace_depth = if trimmed.contains('{') { 1 } else { 0 };
                    continue;
                }
            } else {
                let class_part = trimmed[6..].trim();
                let class_name = class_part
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .trim_end_matches(';')
                    .trim_end_matches('{')
                    .trim();

                if !class_name.is_empty() && !trimmed.ends_with(';') {
                    let clean = clean_unreal_name(class_name);
                    current_class = Some(SdkClassInfo {
                        name: class_name.to_string(),
                        clean_name: clean,
                        super_class: None,
                        clean_super_class: None,
                        functions: HashSet::new(),
                        properties: HashSet::new(),
                    });
                    in_class = true;
                    brace_depth = if trimmed.contains('{') { 1 } else { 0 };
                    continue;
                }
            }
        }

        if in_class {
            if trimmed.contains('{') {
                brace_depth += trimmed.matches('{').count();
            }
            if trimmed.contains('}') {
                let closing = trimmed.matches('}').count();
                if brace_depth <= closing {
                    in_class = false;
                    brace_depth = 0;
                    if let Some(cls) = current_class.take() {
                        let key = cls.clean_name.to_ascii_lowercase();
                        let exact_key = cls.name.to_ascii_lowercase();
                        index.classes.insert(key, cls.clone());
                        index.classes.insert(exact_key, cls);
                    }
                    continue;
                } else {
                    brace_depth -= closing;
                }
            }

            // Inside class: match functions e.g. "void ReportCrimeIdsDelegate__DelegateSignature(...);"
            if let Some(ref mut cls) = current_class {
                if trimmed.contains('(') && trimmed.contains(')') && !trimmed.starts_with('#') {
                    if let Some(paren_pos) = trimmed.find('(') {
                        let before_paren = trimmed[..paren_pos].trim();
                        let tokens: Vec<&str> = before_paren.split_whitespace().collect();
                        if let Some(&func_candidate) = tokens.last() {
                            let clean_func = func_candidate
                                .trim_start_matches('*')
                                .trim_start_matches('&')
                                .trim();
                            if !clean_func.is_empty()
                                && !clean_func.contains("::")
                                && !clean_func.eq_ignore_ascii_case("void")
                                && !clean_func.eq_ignore_ascii_case("bool")
                                && !clean_func.eq_ignore_ascii_case("static")
                                && !clean_func.eq_ignore_ascii_case("const")
                            {
                                if cls.functions.insert(clean_func.to_string()) {
                                    *total_funcs += 1;
                                }
                            }
                        }
                    }
                } else if trimmed.ends_with(';') && !trimmed.starts_with('#') {
                    // Property / Delegate declaration e.g. "FPalPlayerStateOnReportCrimeIdsDelegate OnReportCrimeIdsDelegate;"
                    let before_semi = trimmed.trim_end_matches(';').trim();
                    if let Some(space_pos) = before_semi.rfind(' ') {
                        let prop_candidate = before_semi[space_pos + 1..].trim();
                        let clean_prop = prop_candidate
                            .trim_start_matches('*')
                            .trim_start_matches('&')
                            .trim();
                        if !clean_prop.is_empty() && !clean_prop.contains("::") {
                            cls.properties.insert(clean_prop.to_string());
                        }
                    }
                }
            }
        }
    }

    if let Some(cls) = current_class {
        let key = cls.clean_name.to_ascii_lowercase();
        let exact_key = cls.name.to_ascii_lowercase();
        index.classes.insert(key, cls.clone());
        index.classes.insert(exact_key, cls);
    }
}
