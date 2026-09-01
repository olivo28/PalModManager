//! GPU Block Compression (BC) Decoders
//! Correctly decodes BC1, BC3, BC4, BC5 (Normal Maps with Z reconstruction), BC7, and BGRA/RGBA.

#[inline(always)]
fn decode_bc4_block_internal(block: &[u8], out: &mut [u8; 16]) {
    let a0 = block[0];
    let a1 = block[1];

    let mut lut = [0u8; 8];
    lut[0] = a0;
    lut[1] = a1;

    if a0 > a1 {
        lut[2] = (((6 * a0 as u16 + 1 * a1 as u16) / 7)) as u8;
        lut[3] = (((5 * a0 as u16 + 2 * a1 as u16) / 7)) as u8;
        lut[4] = (((4 * a0 as u16 + 3 * a1 as u16) / 7)) as u8;
        lut[5] = (((3 * a0 as u16 + 4 * a1 as u16) / 7)) as u8;
        lut[6] = (((2 * a0 as u16 + 5 * a1 as u16) / 7)) as u8;
        lut[7] = (((1 * a0 as u16 + 6 * a1 as u16) / 7)) as u8;
    } else {
        lut[2] = (((4 * a0 as u16 + 1 * a1 as u16) / 5)) as u8;
        lut[3] = (((3 * a0 as u16 + 2 * a1 as u16) / 5)) as u8;
        lut[4] = (((2 * a0 as u16 + 3 * a1 as u16) / 5)) as u8;
        lut[5] = (((1 * a0 as u16 + 4 * a1 as u16) / 5)) as u8;
        lut[6] = 0;
        lut[7] = 255;
    }

    let mut indices = 0u64;
    for i in 0..6 {
        indices |= (block[2 + i] as u64) << (i * 8);
    }

    for i in 0..16 {
        let code = ((indices >> (i * 3)) & 0x07) as usize;
        out[i] = lut[code];
    }
}

pub fn decode_compressed_image(
    format_name: &str,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    let fmt = format_name.to_uppercase();
    let num_pixels = (width * height) as usize;
    let total_bytes = num_pixels * 4;

    // 1. BGRA
    if fmt.contains("B8G8R8A8") {
        if data.len() < total_bytes {
            return Err("Data smaller than dimensions".to_string());
        }
        let mut out = vec![0u8; total_bytes];
        for (i, chunk) in data[..total_bytes].chunks_exact(4).enumerate() {
            let b = i * 4;
            out[b] = chunk[2];
            out[b + 1] = chunk[1];
            out[b + 2] = chunk[0];
            out[b + 3] = chunk[3];
        }
        return Ok(out);
    }

    // 2. RGBA
    if fmt.contains("R8G8B8A8") {
        if data.len() < total_bytes {
            return Err("Data smaller than dimensions".to_string());
        }
        return Ok(data[..total_bytes].to_vec());
    }

    // 2b. G8 / R8 (8-bit grayscale uncompressed)
    if (fmt.contains("G8") || fmt.contains("PF_R8")) && !fmt.contains("B8G8R8A8") && !fmt.contains("R8G8B8A8") {
        if data.len() < num_pixels {
            return Err("Data smaller than dimensions".to_string());
        }
        let mut out = vec![255u8; total_bytes];
        for (i, &v) in data[..num_pixels].iter().enumerate() {
            let b = i * 4;
            out[b] = v;
            out[b + 1] = v;
            out[b + 2] = v;
            out[b + 3] = 255;
        }
        return Ok(out);
    }

    // 3. BC5 (Unreal Engine Normal Maps -> R + G + reconstruct Z + A=255)
    if fmt.contains("BC5") || fmt.contains("PF_BC5") {
        let num_blocks_x = ((width + 3) / 4) as usize;
        let num_blocks_y = ((height + 3) / 4) as usize;
        let mut out = vec![255u8; total_bytes];

        let mut r_chan = [0u8; 16];
        let mut g_chan = [0u8; 16];

        for by in 0..num_blocks_y {
            for bx in 0..num_blocks_x {
                let block_idx = by * num_blocks_x + bx;
                let offset = block_idx * 16;
                if offset + 16 > data.len() {
                    break;
                }

                decode_bc4_block_internal(&data[offset..offset + 8], &mut r_chan);
                decode_bc4_block_internal(&data[offset + 8..offset + 16], &mut g_chan);

                for py in 0..4 {
                    let y = by * 4 + py;
                    if y >= height as usize { continue; }
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        if x >= width as usize { continue; }

                        let src_i = py * 4 + px;
                        let r = r_chan[src_i];
                        let g = g_chan[src_i];

                        // Reconstruct tangent normal Z component: Z = sqrt(1 - X^2 - Y^2)
                        let x_f = (r as f32 / 255.0) * 2.0 - 1.0;
                        let y_f = (g as f32 / 255.0) * 2.0 - 1.0;
                        let z_sq = (1.0 - (x_f * x_f + y_f * y_f)).max(0.0);
                        let b = ((z_sq.sqrt() * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;

                        let dst_i = (y * width as usize + x) * 4;
                        out[dst_i] = r;
                        out[dst_i + 1] = g;
                        out[dst_i + 2] = b;
                        out[dst_i + 3] = 255;
                    }
                }
            }
        }
        return Ok(out);
    }

    // 4. BC4 Grayscale / Single-Channel Mask
    if fmt.contains("BC4") || fmt.contains("PF_BC4") {
        let num_blocks_x = ((width + 3) / 4) as usize;
        let num_blocks_y = ((height + 3) / 4) as usize;
        let mut out = vec![255u8; total_bytes];
        let mut g_chan = [0u8; 16];

        for by in 0..num_blocks_y {
            for bx in 0..num_blocks_x {
                let block_idx = by * num_blocks_x + bx;
                let offset = block_idx * 8;
                if offset + 8 > data.len() {
                    break;
                }

                decode_bc4_block_internal(&data[offset..offset + 8], &mut g_chan);

                for py in 0..4 {
                    let y = by * 4 + py;
                    if y >= height as usize { continue; }
                    for px in 0..4 {
                        let x = bx * 4 + px;
                        if x >= width as usize { continue; }

                        let src_i = py * 4 + px;
                        let v = g_chan[src_i];
                        let dst_i = (y * width as usize + x) * 4;
                        out[dst_i] = v;     // R
                        out[dst_i + 1] = v; // G
                        out[dst_i + 2] = v; // B
                        out[dst_i + 3] = 255; // A
                    }
                }
            }
        }
        return Ok(out);
    }

    // 5. Other formats with texture2ddecoder (BC7, BC3, BC1)
    let mut w = width as usize;
    let mut h = height as usize;

    let is_bc1 = fmt.contains("DXT1") || fmt.contains("BC1") || fmt.contains("PF_DXT1");
    let is_bc3 = fmt.contains("DXT5") || fmt.contains("BC3") || fmt.contains("PF_DXT5");
    let block_bytes = if is_bc1 { 8 } else { 16 };

    // Safety fallback: if buffer is smaller than requested blocks, step down resolution to power-of-two that fits
    while w > 64 && h > 64 && (((w + 3) / 4) * ((h + 3) / 4) * block_bytes) > data.len() {
        w /= 2;
        h /= 2;
    }

    let actual_pixels = w * h;
    let mut u32_buffer = vec![0u32; actual_pixels];
    let needed_bytes = ((w + 3) / 4) * ((h + 3) / 4) * block_bytes;
    let slice = if data.len() >= needed_bytes {
        &data[..needed_bytes]
    } else {
        data
    };

    if is_bc3 {
        texture2ddecoder::decode_bc3(slice, w, h, &mut u32_buffer)
            .map_err(|e| format!("BC3 decode error: {:?}", e))?;
    } else if is_bc1 {
        texture2ddecoder::decode_bc1(slice, w, h, &mut u32_buffer)
            .map_err(|e| format!("BC1 decode error: {:?}", e))?;
    } else {
        // Palworld UE5 Default: BC7 (BPTC)
        texture2ddecoder::decode_bc7(slice, w, h, &mut u32_buffer)
            .map_err(|e| format!("BC7 decode error: {:?}", e))?;
    }

    let mut out_rgba = Vec::with_capacity(actual_pixels * 4);
    for pixel in u32_buffer {
        out_rgba.extend_from_slice(&pixel.to_le_bytes());
    }

    Ok(out_rgba)
}