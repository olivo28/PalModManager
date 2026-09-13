// JSON Delta & Merge Patch Engine for Mod Configurations
// Implements recursive delta calculation and RFC-7396 merge patch application
use serde_json::{Map, Value};

/// Computes a lightweight delta patch containing only the modified, added,
/// or deleted keys between an original JSON and a user-modified JSON.
/// Returns `None` if there are zero differences.
pub fn compute_json_delta(original: &Value, modified: &Value) -> Option<Value> {
    if original == modified {
        return None;
    }

    match (original, modified) {
        (Value::Object(orig_map), Value::Object(mod_map)) => {
            let mut delta = Map::new();

            // 1. Check modified or newly added keys
            for (key, mod_val) in mod_map {
                match orig_map.get(key) {
                    Some(orig_val) => {
                        if orig_val != mod_val {
                            if let (Value::Object(_), Value::Object(_)) = (orig_val, mod_val) {
                                if let Some(sub_delta) = compute_json_delta(orig_val, mod_val) {
                                    delta.insert(key.clone(), sub_delta);
                                }
                            } else {
                                delta.insert(key.clone(), mod_val.clone());
                            }
                        }
                    }
                    None => {
                        delta.insert(key.clone(), mod_val.clone());
                    }
                }
            }

            // 2. Check keys deleted in modified
            for key in orig_map.keys() {
                if !mod_map.contains_key(key) {
                    delta.insert(key.clone(), Value::Null);
                }
            }

            if delta.is_empty() {
                None
            } else {
                Some(Value::Object(delta))
            }
        }
        // If one or both are not objects (e.g. array, primitive), the entire modified value is the delta
        _ => Some(modified.clone()),
    }
}

/// Applies a JSON delta patch (RFC 7396 merge patch) onto a target JSON structure.
pub fn apply_json_delta(target: &mut Value, delta: &Value) -> Result<(), String> {
    match delta {
        Value::Object(delta_map) => {
            if !target.is_object() {
                *target = Value::Object(Map::new());
            }

            if let Value::Object(ref mut target_map) = target {
                for (key, val) in delta_map {
                    if val.is_null() {
                        target_map.remove(key);
                    } else if val.is_object() {
                        let entry = target_map.entry(key.clone()).or_insert(Value::Object(Map::new()));
                        apply_json_delta(entry, val)?;
                    } else {
                        target_map.insert(key.clone(), val.clone());
                    }
                }
            }
            Ok(())
        }
        _ => {
            *target = delta.clone();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_delta_no_changes() {
        let a = json!({ "name": "Palworld", "hp": 100 });
        let b = json!({ "name": "Palworld", "hp": 100 });
        assert_eq!(compute_json_delta(&a, &b), None);
    }

    #[test]
    fn test_delta_single_field_change() {
        let orig = json!({ "name": "Palworld", "hp": 100, "speed": 1.0 });
        let modified = json!({ "name": "Palworld", "hp": 250, "speed": 1.0 });

        let delta = compute_json_delta(&orig, &modified).expect("delta expected");
        assert_eq!(delta, json!({ "hp": 250 }));

        let mut base = orig.clone();
        apply_json_delta(&mut base, &delta).expect("apply delta");
        assert_eq!(base, modified);
    }

    #[test]
    fn test_delta_nested_object() {
        let orig = json!({
            "settings": {
                "difficulty": "normal",
                "rates": { "exp": 1.0, "capture": 1.0 }
            },
            "unrelated": "data"
        });

        let modified = json!({
            "settings": {
                "difficulty": "hard",
                "rates": { "exp": 2.5, "capture": 1.0 }
            },
            "unrelated": "data"
        });

        let delta = compute_json_delta(&orig, &modified).expect("delta expected");
        assert_eq!(delta, json!({
            "settings": {
                "difficulty": "hard",
                "rates": { "exp": 2.5 }
            }
        }));

        let mut base = orig.clone();
        apply_json_delta(&mut base, &delta).expect("apply delta");
        assert_eq!(base, modified);
    }

    #[test]
    fn test_delta_field_addition_and_deletion() {
        let orig = json!({ "a": 1, "b": 2 });
        let modified = json!({ "a": 1, "c": 3 });

        let delta = compute_json_delta(&orig, &modified).expect("delta expected");
        assert_eq!(delta, json!({ "b": null, "c": 3 }));

        let mut base = orig.clone();
        apply_json_delta(&mut base, &delta).expect("apply delta");
        assert_eq!(base, modified);
    }
}
