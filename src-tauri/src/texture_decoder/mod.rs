use std::path::Path;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

pub mod bc_decode;
pub mod png_encode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TexturePreviewInfo {
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub format_name: String,
    pub mip_count: u32,
    pub has_alpha: bool,
    pub source_file: String,
    pub size_bytes: usize,
}

#[derive(Debug, Clone)]
struct UexpTextureMeta {
    pub width: u32,
    pub height: u32,
}

/// Parse real SizeX and SizeY of Texture2D in .uexp,
/// validating that dimensions fit within the available payload buffer.
fn parse_texture_metadata_from_uexp(uexp_data: &[u8], max_payload_size: usize) -> Option<UexpTextureMeta> {
    if uexp_data.len() < 32 {
        return None;
    }

    let len = uexp_data.len();
    let mut best: Option<(u32, u32)> = None;

    for i in 0..len.saturating_sub(12) {
        let mut rdr = Cursor::new(&uexp_data[i..]);
        let sx = rdr.read_u32::<LittleEndian>().unwrap_or(0);
        let sy = rdr.read_u32::<LittleEndian>().unwrap_or(0);
        let sz = rdr.read_u32::<LittleEndian>().unwrap_or(0);

        if sz == 1
            && sx >= 64
            && sx <= 8192
            && sy >= 64
            && sy <= 8192
            && sx.is_power_of_two()
            && sy.is_power_of_two()
        {
            // Ensure minimum required bytes (at 0.5 bpp) fit within the available payload
            let min_bytes_needed = (sx as usize * sy as usize) / 2;
            if min_bytes_needed <= max_payload_size + 8192 {
                let area = sx * sy;
                if best.map_or(true, |(bw, bh)| area > bw * bh) {
                    best = Some((sx, sy));
                }
            }
        }
    }

    best.map(|(width, height)| UexpTextureMeta { width, height })
}

/// Deduce real dimensions based on pure Mip 0 byte footprint
fn deduce_dimensions_from_size(size: usize, bytes_per_pixel: f64) -> (u32, u32, usize) {
    let standard_sizes: &[(u32, u32)] = &[
        (4096, 4096),
        (4096, 2048),
        (2048, 4096),
        (2048, 2048),
        (2048, 1024),
        (1024, 2048),
        (1024, 1024),
        (1024, 512),
        (512, 1024),
        (512, 512),
        (512, 256),
        (256, 512),
        (256, 256),
        (128, 128),
        (64, 64),
    ];

    // In streaming mipmap textures (.ubulk), Mip 0 represents ~75% of total file size
    for &(w, h) in standard_sizes {
        let mip0_bytes = ((w as f64 * h as f64) * bytes_per_pixel).round() as usize;
        let total_with_mips = ((mip0_bytes as f64) * 1.333333).round() as usize;

        if size == mip0_bytes || (size >= mip0_bytes && size <= total_with_mips + 4096) {
            return (w, h, mip0_bytes);
        }
    }

    for &(w, h) in standard_sizes {
        let mip0_bytes = ((w as f64 * h as f64) * bytes_per_pixel).round() as usize;
        if size >= mip0_bytes {
            return (w, h, mip0_bytes);
        }
    }

    (2048, 2048, size)
}

/// Extract exact EPixelFormat from .uasset Name Table or NameMap tokens with heuristic fallback.
fn detect_pixel_format(
    uasset_bytes: Option<&[u8]>,
    names_sample: &[String],
    lower_path: &str,
    size_bytes: usize,
) -> (String, f64) {
    let mut candidate_tokens: Vec<String> = names_sample.to_vec();

    // Extract PF_* strings directly from raw .uasset binary stream if available
    if let Some(bytes) = uasset_bytes {
        let text = String::from_utf8_lossy(bytes);
        for token in text.split(|c: char| c == '\0' || c < ' ' || c > '~') {
            let t = token.trim();
            if t.starts_with("PF_") && !candidate_tokens.iter().any(|c| c == t) {
                candidate_tokens.push(t.to_string());
            }
        }
    }

    // Match official Unreal Engine EPixelFormat tokens
    for token in &candidate_tokens {
        match token.as_str() {
            "PF_BC7" => return ("PF_BC7".to_string(), 1.0f64),
            "PF_BC5" => return ("PF_BC5".to_string(), 1.0f64),
            "PF_BC4" => return ("PF_BC4".to_string(), 0.5f64),
            "PF_DXT5" | "PF_BC3" => return ("PF_DXT5".to_string(), 1.0f64),
            "PF_DXT1" | "PF_BC1" => return ("PF_DXT1".to_string(), 0.5f64),
            "PF_B8G8R8A8" => return ("PF_B8G8R8A8".to_string(), 4.0f64),
            "PF_R8G8B8A8" => return ("PF_R8G8B8A8".to_string(), 4.0f64),
            "PF_G8" | "PF_R8" => return ("PF_G8".to_string(), 1.0f64),
            _ => {}
        }
    }

    // Fallback based on Palworld asset naming conventions
    let is_normal = lower_path.contains("_n.") || lower_path.ends_with("_n") || lower_path.contains("_norm") || lower_path.contains("_normal");
    let is_mask_candidate = lower_path.contains("_m.") || lower_path.ends_with("_m") || lower_path.contains("_mask") || lower_path.contains("_ao") || lower_path.contains("_r.") || lower_path.ends_with("_r");

    if is_normal {
        ("PF_BC5".to_string(), 1.0f64)
    } else if is_mask_candidate {
        if size_bytes <= 3_500_000 && size_bytes >= 1_000_000 {
            ("PF_BC4".to_string(), 0.5f64)
        } else {
            ("PF_BC7".to_string(), 1.0f64)
        }
    } else {
        ("PF_BC7".to_string(), 1.0f64)
    }
}

pub fn extract_and_decode_texture(
    pak_path: &Path,
    normalized_uasset_path: &str,
    names_sample: &[String],
) -> Result<TexturePreviewInfo, String> {
    let lower_path = normalized_uasset_path.to_lowercase();
    let base_path = if lower_path.ends_with(".uasset") {
        &normalized_uasset_path[..normalized_uasset_path.len() - 7]
    } else if lower_path.ends_with(".ubulk") || lower_path.ends_with(".uptnl") {
        &normalized_uasset_path[..normalized_uasset_path.len() - 6]
    } else if lower_path.ends_with(".uexp") {
        &normalized_uasset_path[..normalized_uasset_path.len() - 5]
    } else {
        normalized_uasset_path
    };

    let uasset_path = format!("{}.uasset", base_path);
    let ubulk_path = format!("{}.ubulk", base_path);
    let uexp_path = format!("{}.uexp", base_path);

    let uasset_bytes = crate::pak_scanner::extract_pak_entry(pak_path, &uasset_path).ok();
    let uexp_bytes = crate::pak_scanner::extract_pak_entry(pak_path, &uexp_path).ok();

    let (pixel_data, source_file) = if let Ok(ubulk_bytes) = crate::pak_scanner::extract_pak_entry(pak_path, &ubulk_path) {
        if !ubulk_bytes.is_empty() {
            (ubulk_bytes, Path::new(&ubulk_path).file_name().unwrap_or_default().to_string_lossy().to_string())
        } else if let Some(uexp) = uexp_bytes.as_ref() {
            (uexp.clone(), Path::new(&uexp_path).file_name().unwrap_or_default().to_string_lossy().to_string())
        } else {
            return Err("Texture payload is empty".to_string());
        }
    } else if let Some(uexp) = uexp_bytes.as_ref() {
        (uexp.clone(), Path::new(&uexp_path).file_name().unwrap_or_default().to_string_lossy().to_string())
    } else {
        return Err("Texture payload not found".to_string());
    };

    let size_bytes = pixel_data.len();
    if size_bytes < 16 {
        return Err("Texture payload is too small".to_string());
    }

    let uexp_meta = uexp_bytes.as_deref().and_then(|d| parse_texture_metadata_from_uexp(d, size_bytes));

    // Resolve EPixelFormat directly from .uasset NameMap with heuristic fallback
    let (format_name, bytes_per_pixel) = detect_pixel_format(
        uasset_bytes.as_deref(),
        names_sample,
        &lower_path,
        size_bytes,
    );

    let (mut width, mut height, mut mip0_bytes) = if let Some(ref meta) = uexp_meta {
        let expected = ((meta.width as f64 * meta.height as f64) * bytes_per_pixel).round() as usize;
        (meta.width, meta.height, expected)
    } else {
        deduce_dimensions_from_size(size_bytes, bytes_per_pixel)
    };

    // Auto-fit / downscale dimensions iteratively if required bytes exceed available payload
    while mip0_bytes > pixel_data.len() && width > 64 && height > 64 {
        width /= 2;
        height /= 2;
        mip0_bytes = ((width as f64 * height as f64) * bytes_per_pixel).round() as usize;
    }

    let slice_len = mip0_bytes.min(pixel_data.len());
    let mip0_slice = &pixel_data[..slice_len];

    let rgba8_pixels = bc_decode::decode_compressed_image(&format_name, width, height, mip0_slice)?;

    let mut has_alpha = false;
    if !format_name.contains("BC5") && !format_name.contains("BC4") {
        for chunk in rgba8_pixels.chunks_exact(4) {
            if chunk[3] < 250 {
                has_alpha = true;
                break;
            }
        }
    }

    let png_bytes = png_encode::encode_rgba8_to_png(width, height, &rgba8_pixels)?;
    let base64_str = BASE64.encode(&png_bytes);
    let data_url = format!("data:image/png;base64,{}", base64_str);
    let mip_count = ((width.max(height) as f64).log2().floor() as u32) + 1;

    Ok(TexturePreviewInfo {
        data_url,
        width,
        height,
        format_name,
        mip_count,
        has_alpha,
        source_file,
        size_bytes,
    })
}