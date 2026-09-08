use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct LuaParamInfo {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub struct LuaMethodInfo {
    pub class_name: String,
    pub method_name: String,
    pub params: Vec<LuaParamInfo>,
    pub return_type: Option<String>,
}

impl LuaMethodInfo {
    /// Builds a Monaco-compatible snippet string with tab-stops:
    /// e.g. "prefix:MethodName(${1:Param1}, ${2:Param2})"
    pub fn build_snippet(&self, prefix: &str) -> String {
        if self.params.is_empty() {
            return format!("{}:{}()", prefix, self.method_name);
        }

        let args: Vec<String> = self
            .params
            .iter()
            .enumerate()
            .map(|(idx, p)| format!("${{{}:{}}}", idx + 1, p.name))
            .collect();

        format!("{}:{}({})", prefix, self.method_name, args.join(", "))
    }

    /// Builds a clean TypeScript/EmmyLua style method signature:
    /// e.g. "(Param1: Type, Param2: Type) -> ReturnType"
    pub fn build_detail(&self) -> String {
        let args: Vec<String> = self
            .params
            .iter()
            .map(|p| {
                if p.type_name.is_empty() {
                    p.name.clone()
                } else {
                    format!("{}: {}", p.name, p.type_name)
                }
            })
            .collect();

        let ret = self
            .return_type
            .as_deref()
            .unwrap_or("void");

        format!("({}) -> {}", args.join(", "), ret)
    }
}

type SignatureMap = HashMap<String, HashMap<String, LuaMethodInfo>>;

static SIGNATURE_CACHE: Mutex<Option<Arc<SignatureMap>>> = Mutex::new(None);

/// Resolves the path to Pal.lua containing primary Palworld EmmyLua definitions.
fn resolve_pal_lua_path(game_path: &str, program_path: &str) -> Option<PathBuf> {
    // 1. Live game installation
    if !game_path.is_empty() {
        let live_path = Path::new(game_path)
            .join("Pal")
            .join("Binaries")
            .join("Win64")
            .join("ue4ss")
            .join("Mods")
            .join("shared")
            .join("types")
            .join("Pal.lua");
        if live_path.is_file() {
            return Some(live_path);
        }
    }

    // 2. Resources unpacked directory
    if !program_path.is_empty() {
        let res_path = Path::new(program_path)
            .join("resources")
            .join("lua_types")
            .join("Pal.lua");
        if res_path.is_file() {
            return Some(res_path);
        }
    }

    // 3. Current working directory fallback
    let local_path = PathBuf::from("resources").join("lua_types").join("Pal.lua");
    if local_path.is_file() {
        return Some(local_path);
    }

    None
}

/// Parses Pal.lua into an indexed map of Class -> Method -> LuaMethodInfo.
pub fn get_or_load_lua_signatures(game_path: &str, program_path: &str) -> Option<Arc<SignatureMap>> {
    let mut cache = SIGNATURE_CACHE.lock().ok()?;
    if let Some(ref map) = *cache {
        return Some(Arc::clone(map));
    }

    let lua_path = resolve_pal_lua_path(game_path, program_path)?;
    let content = fs::read_to_string(&lua_path).ok()?;

    let mut map: SignatureMap = HashMap::new();
    let mut pending_params: Vec<LuaParamInfo> = Vec::new();
    let mut pending_return: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("---@param ") {
            let rest = trimmed.trim_start_matches("---@param ").trim();
            if let Some((name, type_name)) = rest.split_once(' ') {
                pending_params.push(LuaParamInfo {
                    name: name.trim().to_string(),
                    type_name: type_name.trim().to_string(),
                });
            } else if !rest.is_empty() {
                pending_params.push(LuaParamInfo {
                    name: rest.to_string(),
                    type_name: String::new(),
                });
            }
            continue;
        }

        if trimmed.starts_with("---@return ") {
            let ret = trimmed.trim_start_matches("---@return ").trim();
            pending_return = Some(ret.to_string());
            continue;
        }

        // Detect function ClassName:MethodName(args) end
        if trimmed.starts_with("function ") && trimmed.contains(':') {
            let rest = trimmed.trim_start_matches("function ").trim();
            if let Some((class_name, method_part)) = rest.split_once(':') {
                let method_name = method_part.split('(').next().unwrap_or("").trim();
                if !method_name.is_empty() {
                    let class_lower = class_name.trim().to_ascii_lowercase();
                    let method_lower = method_name.to_ascii_lowercase();

                    let info = LuaMethodInfo {
                        class_name: class_name.trim().to_string(),
                        method_name: method_name.to_string(),
                        params: std::mem::take(&mut pending_params),
                        return_type: pending_return.take(),
                    };

                    map.entry(class_lower)
                        .or_default()
                        .insert(method_lower, info);
                }
            }
            pending_params.clear();
            pending_return = None;
            continue;
        }

        if !trimmed.starts_with("---") {
            pending_params.clear();
            pending_return = None;
        }
    }

    let arc_map = Arc::new(map);
    *cache = Some(Arc::clone(&arc_map));
    Some(arc_map)
}

/// Finds a method signature for a given class name and method name.
pub fn find_lua_method_signature(
    map: &SignatureMap,
    class_name: &str,
    method_name: &str,
) -> Option<LuaMethodInfo> {
    let clean_cls = class_name
        .trim_start_matches('/')
        .split('.')
        .last()
        .unwrap_or(class_name)
        .to_ascii_lowercase();

    let method_lower = method_name.to_ascii_lowercase();

    map.get(&clean_cls)
        .and_then(|methods| methods.get(&method_lower))
        .cloned()
}
