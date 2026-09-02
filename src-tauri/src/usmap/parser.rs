use std::path::Path;
use std::fs;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsmapProperty {
    pub index: u16,
    pub name: String,
    pub type_name: String,
    pub struct_type: Option<String>,
    pub enum_type: Option<String>,
    pub inner_type: Option<String>,
    pub array_dim: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsmapStruct {
    pub name: String,
    pub super_type: Option<String>,
    pub properties: Vec<UsmapProperty>,
    pub property_map: HashMap<u16, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsmapSchema {
    pub game_version: String,
    pub names: Vec<String>,
    pub enums: HashMap<String, Vec<String>>,
    pub structs: HashMap<String, UsmapStruct>,
    pub total_structs: usize,
    pub total_enums: usize,
    pub total_names: usize,
}

impl UsmapSchema {
    pub fn get_struct(&self, name: &str) -> Option<&UsmapStruct> {
        self.structs.get(name)
    }

    pub fn find_struct(&self, query: &str) -> Option<&UsmapStruct> {
        if let Some(s) = self.structs.get(query) {
            return Some(s);
        }
        let clean = query.trim_start_matches('F').trim_start_matches('U').trim_start_matches('A');
        if let Some(s) = self.structs.get(clean) {
            return Some(s);
        }
        for (k, v) in &self.structs {
            if k.eq_ignore_ascii_case(query) || k.eq_ignore_ascii_case(clean) {
                return Some(v);
            }
        }
        None
    }

    pub fn get_enum(&self, name: &str) -> Option<&Vec<String>> {
        self.enums.get(name)
    }

    pub fn resolve_property_name(&self, struct_name: &str, prop_index: u16) -> Option<String> {
        let mut current_name = Some(struct_name.to_string());
        while let Some(name) = current_name {
            if let Some(s) = self.structs.get(&name) {
                if let Some(prop_name) = s.property_map.get(&prop_index) {
                    return Some(prop_name.clone());
                }
                current_name = s.super_type.clone();
            } else {
                break;
            }
        }
        None
    }
}

pub fn parse_usmap_file(path: &Path) -> Result<UsmapSchema, String> {
    if !path.exists() {
        return Err(format!("USMAP file not found at {:?}", path));
    }

    let buffer = fs::read(path).map_err(|e| format!("Failed to read USMAP file: {}", e))?;
    if buffer.len() < 12 {
        return Err("USMAP file is too small to contain a valid header.".to_string());
    }

    let mut cursor = 0;

    // 1. Read Magic
    let magic = read_u16(&buffer, &mut cursor)?;
    if magic != 0x30C4 && magic != 0x30C0 && magic != 0x30C1 && magic != 0x30C2 && magic != 0x30C3 {
        return Err(format!("Invalid USMAP magic: 0x{:04X}", magic));
    }

    // 2. Read Version & Compression
    let version = read_u8(&buffer, &mut cursor)?;
    let compression = read_u8(&buffer, &mut cursor)?;
    let _compressed_size = read_u32(&buffer, &mut cursor)? as usize;
    let _decompressed_size = read_u32(&buffer, &mut cursor)? as usize;

    let payload: Vec<u8> = if compression == 0 {
        buffer[cursor..].to_vec()
    } else {
        buffer[cursor..].to_vec()
    };

    let mut p_cursor = 0;

    // 3. Read String Table (Names)
    // In UnrealMappingsDumper / UAssetAPI, payload starts with names_data_len then name_count
    let _names_data_len = read_u32(&payload, &mut p_cursor)? as usize;
    let name_count = read_u32(&payload, &mut p_cursor)? as usize;
    let mut names = Vec::with_capacity(name_count);

    for _ in 0..name_count {
        let len = if version >= 3 {
            read_u16(&payload, &mut p_cursor)? as usize
        } else {
            read_u8(&payload, &mut p_cursor)? as usize
        };
        if p_cursor + len > payload.len() {
            return Err("Unexpected end of buffer while reading names table.".to_string());
        }
        let str_bytes = &payload[p_cursor..p_cursor + len];
        p_cursor += len;
        let s = String::from_utf8_lossy(str_bytes).to_string();
        names.push(s);
    }

    // 4. Read Enums Table
    let enum_count = read_u32(&payload, &mut p_cursor).unwrap_or(0) as usize;
    let mut enums = HashMap::with_capacity(enum_count.min(10000));

    for _ in 0..enum_count {
        if p_cursor >= payload.len() { break; }
        let enum_name_idx = match read_u32(&payload, &mut p_cursor) {
            Ok(idx) => idx as usize,
            Err(_) => break,
        };
        let enum_name = names.get(enum_name_idx).cloned().unwrap_or_else(|| format!("Enum_{}", enum_name_idx));

        let val_count = if version >= 4 {
            read_u16(&payload, &mut p_cursor).unwrap_or(0) as usize
        } else {
            read_u8(&payload, &mut p_cursor).unwrap_or(0) as usize
        };

        let mut values = Vec::with_capacity(val_count.min(1000));
        for _ in 0..val_count {
            if p_cursor >= payload.len() { break; }
            let val_idx = match read_u32(&payload, &mut p_cursor) {
                Ok(idx) => idx as usize,
                Err(_) => break,
            };
            let val_name = names.get(val_idx).cloned().unwrap_or_else(|| format!("Val_{}", val_idx));
            values.push(val_name);
        }
        enums.insert(enum_name, values);
    }

    // 5. Read Structs / Classes Table
    let struct_count = read_u32(&payload, &mut p_cursor).unwrap_or(0) as usize;
    let mut structs = HashMap::with_capacity(struct_count.min(20000));

    for _ in 0..struct_count {
        if p_cursor >= payload.len() { break; }
        let struct_name_idx = match read_u32(&payload, &mut p_cursor) {
            Ok(idx) => idx as usize,
            Err(_) => break,
        };
        let struct_name = names.get(struct_name_idx).cloned().unwrap_or_else(|| format!("Struct_{}", struct_name_idx));

        let super_idx = match read_u32(&payload, &mut p_cursor) {
            Ok(idx) => idx as usize,
            Err(_) => break,
        };
        let super_type = if super_idx != 0xFFFFFFFF && super_idx < names.len() {
            Some(names[super_idx].clone())
        } else {
            None
        };

        let prop_count = match read_u16(&payload, &mut p_cursor) {
            Ok(c) => c as usize,
            Err(_) => break,
        };
        let _serializable_prop_count = read_u16(&payload, &mut p_cursor).unwrap_or(0);

        let mut properties = Vec::with_capacity(prop_count.min(2000));
        let mut property_map = HashMap::with_capacity(prop_count.min(2000));

        for _ in 0..prop_count {
            if p_cursor >= payload.len() { break; }
            let prop_index = match read_u16(&payload, &mut p_cursor) {
                Ok(idx) => idx,
                Err(_) => break,
            };
            let array_dim = match read_u8(&payload, &mut p_cursor) {
                Ok(d) => d,
                Err(_) => break,
            };
            let prop_name_idx = match read_u32(&payload, &mut p_cursor) {
                Ok(idx) => idx as usize,
                Err(_) => break,
            };
            let prop_name = names.get(prop_name_idx).cloned().unwrap_or_else(|| format!("Prop_{}", prop_name_idx));

            let (type_name, struct_type, enum_type, inner_type) = match parse_property_type(&payload, &mut p_cursor, &names) {
                Ok(t) => t,
                Err(_) => break,
            };

            property_map.insert(prop_index, prop_name.clone());
            properties.push(UsmapProperty {
                index: prop_index,
                name: prop_name,
                type_name,
                struct_type,
                enum_type,
                inner_type,
                array_dim,
            });
        }

        structs.insert(struct_name.clone(), UsmapStruct {
            name: struct_name,
            super_type,
            properties,
            property_map,
        });
    }

    let total_structs = structs.len();
    let total_enums = enums.len();
    let total_names = names.len();
    names.sort();

    Ok(UsmapSchema {
        game_version: "v1.0.3".to_string(),
        names,
        enums,
        structs,
        total_structs,
        total_enums,
        total_names,
    })
}

fn parse_property_type(payload: &[u8], cursor: &mut usize, names: &[String]) -> Result<(String, Option<String>, Option<String>, Option<String>), String> {
    let type_tag = read_u8(payload, cursor)?;
    match type_tag {
        0 => {
            // ByteProperty in USMAP has an optional enum_name_idx (u32) or 0xFFFFFFFF
            let e_idx = read_u32(payload, cursor)? as usize;
            let enum_name = if e_idx != 0xFFFFFFFF && e_idx < names.len() {
                names.get(e_idx).cloned()
            } else {
                None
            };
            Ok(("ByteProperty".to_string(), None, enum_name, None))
        }
        1 => Ok(("BoolProperty".to_string(), None, None, None)),
        2 => Ok(("Int8Property".to_string(), None, None, None)),
        3 => Ok(("Int16Property".to_string(), None, None, None)),
        4 => Ok(("IntProperty".to_string(), None, None, None)),
        5 => Ok(("Int64Property".to_string(), None, None, None)),
        6 => Ok(("UInt16Property".to_string(), None, None, None)),
        7 => Ok(("UInt32Property".to_string(), None, None, None)),
        8 => Ok(("UInt64Property".to_string(), None, None, None)),
        9 => Ok(("FloatProperty".to_string(), None, None, None)),
        10 => Ok(("DoubleProperty".to_string(), None, None, None)),
        11 => Ok(("StrProperty".to_string(), None, None, None)),
        12 => Ok(("NameProperty".to_string(), None, None, None)),
        13 => Ok(("TextProperty".to_string(), None, None, None)),
        14 => {
            // EnumProperty: inner property type + enum_name_idx
            let (inner_name, _, _, _) = parse_property_type(payload, cursor, names)?;
            let enum_idx = read_u32(payload, cursor)? as usize;
            let enum_name = if enum_idx != 0xFFFFFFFF && enum_idx < names.len() {
                names.get(enum_idx).cloned()
            } else {
                None
            };
            Ok(("EnumProperty".to_string(), None, enum_name, Some(inner_name)))
        }
        15 => {
            // StructProperty: struct_name_idx
            let s_idx = read_u32(payload, cursor)? as usize;
            let s_name = if s_idx != 0xFFFFFFFF && s_idx < names.len() {
                names.get(s_idx).cloned()
            } else {
                None
            };
            Ok(("StructProperty".to_string(), s_name, None, None))
        }
        16 => {
            // ArrayProperty: inner property type
            let (inner_name, s_name, e_name, _) = parse_property_type(payload, cursor, names)?;
            Ok(("ArrayProperty".to_string(), s_name, e_name, Some(inner_name)))
        }
        17 => {
            // SetProperty: inner property type
            let (inner_name, s_name, e_name, _) = parse_property_type(payload, cursor, names)?;
            Ok(("SetProperty".to_string(), s_name, e_name, Some(inner_name)))
        }
        18 => {
            // MapProperty: key type + value type
            let (k_name, _, _, _) = parse_property_type(payload, cursor, names)?;
            let (v_name, s_name, e_name, _) = parse_property_type(payload, cursor, names)?;
            Ok(("MapProperty".to_string(), s_name, e_name, Some(format!("{} -> {}", k_name, v_name))))
        }
        19 => Ok(("ObjectProperty".to_string(), None, None, None)),
        20 => Ok(("ClassProperty".to_string(), None, None, None)),
        21 => Ok(("SoftObjectProperty".to_string(), None, None, None)),
        22 => Ok(("SoftClassProperty".to_string(), None, None, None)),
        23 => Ok(("DelegateProperty".to_string(), None, None, None)),
        24 => Ok(("MulticastDelegateProperty".to_string(), None, None, None)),
        25 => Ok(("MulticastInlineDelegateProperty".to_string(), None, None, None)),
        26 => Ok(("FieldPathProperty".to_string(), None, None, None)),
        27 => {
            // OptionalProperty: inner property type
            let (inner_name, s_name, e_name, _) = parse_property_type(payload, cursor, names)?;
            Ok(("OptionalProperty".to_string(), s_name, e_name, Some(inner_name)))
        }
        other => Ok((format!("UnknownProperty_{}", other), None, None, None)),
    }
}

fn read_u8(buf: &[u8], cursor: &mut usize) -> Result<u8, String> {
    if *cursor >= buf.len() { return Err("Buffer underflow reading u8".to_string()); }
    let val = buf[*cursor];
    *cursor += 1;
    Ok(val)
}

fn read_u16(buf: &[u8], cursor: &mut usize) -> Result<u16, String> {
    if *cursor + 2 > buf.len() { return Err("Buffer underflow reading u16".to_string()); }
    let val = u16::from_le_bytes([buf[*cursor], buf[*cursor + 1]]);
    *cursor += 2;
    Ok(val)
}

fn read_u32(buf: &[u8], cursor: &mut usize) -> Result<u32, String> {
    if *cursor + 4 > buf.len() { return Err("Buffer underflow reading u32".to_string()); }
    let val = u32::from_le_bytes([buf[*cursor], buf[*cursor + 1], buf[*cursor + 2], buf[*cursor + 3]]);
    *cursor += 4;
    Ok(val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_bundled_usmap() {
        let usmap_path = PathBuf::from("../resources/mappings/Palworld.usmap");
        if usmap_path.exists() {
            let buffer = fs::read(&usmap_path).unwrap();
            eprintln!("USMAP file len = {}", buffer.len());
            let schema = parse_usmap_file(&usmap_path).expect("Should parse Palworld.usmap cleanly");
            eprintln!("PARSED USMAP RESULT: structs={}, enums={}, names={}", schema.total_structs, schema.total_enums, schema.total_names);
        }
    }
}
