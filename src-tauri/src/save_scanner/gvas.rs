use std::io::{Cursor, Read, Write};

/// Decompresses Palworld PLZ / ZLIB / GVAS encoded bytes into raw GVAS stream
pub fn decompress_palworld_save(raw_bytes: &[u8]) -> Result<Vec<u8>, String> {
    if raw_bytes.len() < 4 {
        return Err("File too small to be a valid save".to_string());
    }

    // 1. Check if already uncompressed GVAS (starts exactly with "GVAS")
    if raw_bytes.starts_with(b"GVAS") {
        return Ok(raw_bytes.to_vec());
    }

    // 2a. Palworld PlM / LZ4 format (used for Level.sav since Palworld ~0.4+)
    //
    // Outer header layout: [u32 uncomp_total][u32 comp_total][3 bytes "PlM"][u8 version]
    //   = 12 bytes total. The version byte (e.g. 0x31 = '1') is always present.
    if raw_bytes.len() > 12 && raw_bytes.get(8..11) == Some(b"PlM") {
        #[inline(always)]
        fn read_u32(b: &[u8], off: usize) -> usize {
            if off + 4 > b.len() { return 0; }
            u32::from_le_bytes([b[off], b[off+1], b[off+2], b[off+3]]) as usize
        }

        let total_uncomp = read_u32(raw_bytes, 0);
        let total_comp   = read_u32(raw_bytes, 4);
        const HDR: usize = 12;

        // --- Path A1: Oodle (Mermaid/Kraken) decompression via oozextract ---
        if total_comp > 0 && HDR + total_comp <= raw_bytes.len() {
            let mut extractor = oozextract::Extractor::new();
            let mut out = vec![0u8; total_uncomp];
            let comp_slice = &raw_bytes[HDR..HDR + total_comp];
            crate::logger::log(&format!(
                "[PMM-DBG] PlM oozextract: decompressing {} compressed bytes -> {} expected uncompressed bytes",
                comp_slice.len(), total_uncomp
            ));
            match extractor.read_from_slice(comp_slice, &mut out) {
                Ok(decomp_len) => {
                    crate::logger::log(&format!("[PMM-DBG] PlM oozextract SUCCESS: {} bytes", decomp_len));
                    out.truncate(decomp_len);
                    return Ok(out);
                }
                Err(e) => {
                    crate::logger::log(&format!("[PMM-DBG] PlM oozextract FAIL: {:?}", e));
                }
            }
        }

        // --- Path A2: LZ4 fallback ---
        if total_comp > 0 && HDR + total_comp <= raw_bytes.len() {
            crate::logger::log(&format!(
                "[PMM-DBG] PlM Path A (LZ4): decompress bytes[{}..{}] ({} bytes) → expected {} bytes",
                HDR, HDR + total_comp, total_comp, total_uncomp
            ));
            match lz4_flex::decompress(&raw_bytes[HDR..HDR + total_comp], total_uncomp) {
                Ok(dec) if !dec.is_empty() => {
                    crate::logger::log(&format!("[PMM-DBG] PlM Path A OK: {} bytes", dec.len()));
                    return Ok(dec);
                }
                Ok(_) => crate::logger::log("[PMM-DBG] PlM Path A returned empty"),
                Err(e) => crate::logger::log(&format!("[PMM-DBG] PlM Path A FAIL: {}", e)),
            }
        }

        // --- Path B: multi-chunk (each chunk has its own 12-byte header) ---
        let mut out: Vec<u8> = Vec::with_capacity(total_uncomp.min(256 * 1024 * 1024));
        let mut pos = HDR;
        let mut ok = true;
        while pos + HDR < raw_bytes.len() {
            if raw_bytes.get(pos + 8..pos + 11) != Some(b"PlM") {
                break;
            }
            let chunk_uncomp = read_u32(raw_bytes, pos);
            let chunk_comp   = read_u32(raw_bytes, pos + 4);
            pos += HDR;
            if chunk_comp == 0 || pos + chunk_comp > raw_bytes.len() { ok = false; break; }
            let comp_chunk = &raw_bytes[pos..pos + chunk_comp];
            
            let mut chunk_out = vec![0u8; chunk_uncomp];
            let mut extractor = oozextract::Extractor::new();
            if let Ok(d_len) = extractor.read_from_slice(comp_chunk, &mut chunk_out) {
                chunk_out.truncate(d_len);
                out.extend_from_slice(&chunk_out);
            } else if let Ok(dec) = lz4_flex::decompress(comp_chunk, chunk_uncomp) {
                out.extend_from_slice(&dec);
            } else {
                ok = false;
                break;
            }
            pos += chunk_comp;
        }
        if ok && !out.is_empty() {
            return Ok(out);
        }
    }

    // Fallback if GVAS magic is at a small offset
    if let Some(pos) = raw_bytes.windows(4).position(|w| w == b"GVAS") {
        if pos <= 32 {
            return Ok(raw_bytes[pos..].to_vec());
        }
    }

    // 2b. Candidate slice offsets for Palworld Pocketpair PLZ/ZLIB formats
    let mut candidate_slices: Vec<&[u8]> = Vec::new();

    if raw_bytes.len() > 12 { candidate_slices.push(&raw_bytes[12..]); }
    if raw_bytes.len() > 11 { candidate_slices.push(&raw_bytes[11..]); }
    if raw_bytes.len() > 8 { candidate_slices.push(&raw_bytes[8..]); }
    candidate_slices.push(raw_bytes);

    for (i, w) in raw_bytes.windows(2).take(64).enumerate() {
        if w[0] == 0x78 && (w[1] == 0x9c || w[1] == 0x01 || w[1] == 0xda || w[1] == 0x5e || w[1] == 0xbb) {
            candidate_slices.push(&raw_bytes[i..]);
        }
    }

    for slice in candidate_slices {
        let mut zlib_dec = flate2::read::ZlibDecoder::new(Cursor::new(slice));
        let mut out = Vec::new();
        if zlib_dec.read_to_end(&mut out).is_ok() && out.len() >= 4 {
            if out.starts_with(b"GVAS") || out.windows(4).take(32).any(|w| w == b"GVAS") || out.len() > 1024 {
                return Ok(out);
            }
        }

        let mut def_dec = flate2::read::DeflateDecoder::new(Cursor::new(slice));
        let mut def_out = Vec::new();
        if def_dec.read_to_end(&mut def_out).is_ok() && def_out.len() >= 4 {
            if def_out.starts_with(b"GVAS") || def_out.windows(4).take(32).any(|w| w == b"GVAS") || def_out.len() > 1024 {
                return Ok(def_out);
            }
        }
    }

    Err("Failed to decompress save file with ZLIB/PLZ format".to_string())
}

/// Compress raw GVAS bytes into Palworld PLZ/ZLIB container
pub fn compress_palworld_save(uncompressed: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    let uncompressed_len = uncompressed.len() as u32;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(uncompressed).map_err(|e| e.to_string())?;
    let compressed = encoder.finish().map_err(|e| e.to_string())?;
    let compressed_len = compressed.len() as u32;

    let mut out = Vec::with_capacity(12 + compressed.len());
    out.extend_from_slice(&uncompressed_len.to_le_bytes());
    out.extend_from_slice(&compressed_len.to_le_bytes());
    out.extend_from_slice(b"PlZ\0");
    out.extend_from_slice(&compressed);

    Ok(out)
}

/// Read one Palworld GVAS FString at `pos`. Returns `(string, bytes_consumed)`.
pub fn gvas_fstring(bytes: &[u8], pos: usize) -> Option<(String, usize)> {
    if pos + 4 > bytes.len() { return None; }
    let len = i32::from_le_bytes(bytes[pos..pos + 4].try_into().ok()?);

    if len == 0 {
        return Some((String::new(), 4));
    }

    if len > 0 {
        let byte_len = len as usize;
        if pos + 4 + byte_len > bytes.len() { return None; }
        let s = String::from_utf8_lossy(&bytes[pos + 4..pos + 4 + byte_len.saturating_sub(1)]).into_owned();
        Some((s, 4 + byte_len))
    } else {
        let char_count = (-len) as usize;
        let byte_len = char_count * 2;
        if pos + 4 + byte_len > bytes.len() { return None; }
        let chars: Vec<u16> = bytes[pos + 4..pos + 4 + byte_len]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let s = String::from_utf16_lossy(&chars[..chars.len().saturating_sub(1)]);
        Some((s, 4 + byte_len))
    }
}

/// Scan `bytes` from `search_start` for a top-level GVAS property whose FString name matches `prop_name` exactly.
pub fn gvas_find_property(bytes: &[u8], prop_name: &str, search_start: usize) -> Option<(String, usize)> {
    let name_bytes = prop_name.as_bytes();
    let expected_len = (name_bytes.len() as i32) + 1;
    let len_le = expected_len.to_le_bytes();

    let mut i = search_start;
    let limit = bytes.len().saturating_sub(name_bytes.len() + 4 + 60);

    while i < limit {
        if bytes[i] != len_le[0] { i += 1; continue; }

        if &bytes[i..i + 4] == &len_le
            && bytes.get(i + 4..i + 4 + name_bytes.len()) == Some(name_bytes)
            && bytes.get(i + 4 + name_bytes.len()) == Some(&0u8)
        {
            let mut pos = i + 4 + name_bytes.len() + 1;

            let (type_name, t_sz) = gvas_fstring(bytes, pos)?;
            pos += t_sz;

            let valid_types = [
                "IntProperty", "Int64Property", "FloatProperty", "DoubleProperty",
                "StrProperty", "NameProperty", "TextProperty", "BoolProperty",
                "ByteProperty", "StructProperty", "ArrayProperty", "MapProperty",
                "ObjectProperty", "SoftObjectProperty", "EnumProperty",
            ];
            if !valid_types.contains(&type_name.as_str()) { i += 1; continue; }

            if pos + 8 > bytes.len() { i += 1; continue; }
            let payload_size = u64::from_le_bytes(bytes[pos..pos + 8].try_into().ok()?) as usize;
            pos += 8;

            match type_name.as_str() {
                "StructProperty" => {
                    let (_, st_sz) = gvas_fstring(bytes, pos)?;
                    pos += st_sz;
                    pos += 16;
                }
                "ArrayProperty" => {
                    let (_, at_sz) = gvas_fstring(bytes, pos)?;
                    pos += at_sz;
                }
                "MapProperty" => {
                    let (_, kt_sz) = gvas_fstring(bytes, pos)?;
                    pos += kt_sz;
                    let (_, vt_sz) = gvas_fstring(bytes, pos)?;
                    pos += vt_sz;
                }
                "EnumProperty" => {
                    let (_, et_sz) = gvas_fstring(bytes, pos)?;
                    pos += et_sz;
                }
                "ByteProperty" => {
                    let (_, bt_sz) = gvas_fstring(bytes, pos)?;
                    pos += bt_sz;
                }
                _ => {}
            }

            if type_name == "BoolProperty" {
                return Some((type_name, pos));
            }

            if pos >= bytes.len() { i += 1; continue; }
            let has_guid = bytes[pos];
            pos += 1;
            if has_guid != 0 {
                if pos + 16 > bytes.len() { i += 1; continue; }
                pos += 16;
            }

            if payload_size > 0 && pos + payload_size > bytes.len() { i += 1; continue; }

            return Some((type_name, pos));
        }
        i += 1;
    }
    None
}

/// Read an `Int64Property` by exact name from decompressed GVAS bytes.
pub fn gvas_read_int64(bytes: &[u8], prop_name: &str, search_start: usize) -> Option<i64> {
    let (type_name, vpos) = gvas_find_property(bytes, prop_name, search_start)?;
    match type_name.as_str() {
        "Int64Property" => {
            if vpos + 8 > bytes.len() { return None; }
            Some(i64::from_le_bytes(bytes[vpos..vpos + 8].try_into().ok()?))
        }
        "IntProperty" => {
            if vpos + 4 > bytes.len() { return None; }
            Some(i32::from_le_bytes(bytes[vpos..vpos + 4].try_into().ok()?) as i64)
        }
        "ByteProperty" => {
            if vpos < bytes.len() {
                Some(bytes[vpos] as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Read an `IntProperty` / `Int64Property` / `FloatProperty` / `ByteProperty` by exact name
pub fn gvas_read_int(bytes: &[u8], prop_name: &str, search_start: usize) -> Option<i32> {
    let (type_name, vpos) = gvas_find_property(bytes, prop_name, search_start)?;
    match type_name.as_str() {
        "IntProperty" => {
            if vpos + 4 > bytes.len() { return None; }
            Some(i32::from_le_bytes(bytes[vpos..vpos + 4].try_into().ok()?))
        }
        "ByteProperty" => {
            if vpos < bytes.len() {
                Some(bytes[vpos] as i32)
            } else {
                None
            }
        }
        "Int64Property" => {
            if vpos + 8 > bytes.len() { return None; }
            let v = i64::from_le_bytes(bytes[vpos..vpos + 8].try_into().ok()?);
            Some(v as i32)
        }
        "FloatProperty" => {
            if vpos + 4 > bytes.len() { return None; }
            let v = f32::from_le_bytes(bytes[vpos..vpos + 4].try_into().ok()?);
            Some(v as i32)
        }
        _ => None,
    }
}

/// Read a `StrProperty` / `NameProperty` by exact name from decompressed GVAS bytes.
pub fn gvas_read_str(bytes: &[u8], prop_name: &str, search_start: usize) -> Option<String> {
    let (type_name, vpos) = gvas_find_property(bytes, prop_name, search_start)?;
    match type_name.as_str() {
        "StrProperty" | "NameProperty" | "TextProperty" => {
            let (s, _) = gvas_fstring(bytes, vpos)?;
            if !s.is_empty() && s != "None" {
                Some(s)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Read a `FloatProperty` or `DoubleProperty` by exact name from decompressed GVAS bytes.
pub fn gvas_read_float(bytes: &[u8], prop_name: &str, search_start: usize) -> Option<f32> {
    let (type_name, vpos) = gvas_find_property(bytes, prop_name, search_start)?;
    match type_name.as_str() {
        "FloatProperty" => {
            if vpos + 4 > bytes.len() { return None; }
            Some(f32::from_le_bytes(bytes[vpos..vpos + 4].try_into().ok()?))
        }
        "DoubleProperty" => {
            if vpos + 8 > bytes.len() { return None; }
            Some(f64::from_le_bytes(bytes[vpos..vpos + 8].try_into().ok()?) as f32)
        }
        _ => None,
    }
}

/// Read a `BoolProperty` by exact name from decompressed GVAS bytes.
pub fn gvas_read_bool(bytes: &[u8], prop_name: &str, search_start: usize) -> Option<bool> {
    let (type_name, vpos) = gvas_find_property(bytes, prop_name, search_start)?;
    if type_name == "BoolProperty" && vpos < bytes.len() {
        Some(bytes[vpos] != 0)
    } else {
        None
    }
}
