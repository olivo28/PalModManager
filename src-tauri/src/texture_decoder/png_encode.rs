use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

/// Computes CRC32 checksum for PNG chunks
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn write_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    let length = data.len() as u32;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(chunk_type);
    output.extend_from_slice(data);

    let mut crc_payload = Vec::with_capacity(4 + data.len());
    crc_payload.extend_from_slice(chunk_type);
    crc_payload.extend_from_slice(data);
    let checksum = crc32(&crc_payload);
    output.extend_from_slice(&checksum.to_be_bytes());
}

/// Encodes an RGBA8 buffer (width * height * 4 bytes) into a valid PNG byte vector
pub fn encode_rgba8_to_png(width: u32, height: u32, rgba_pixels: &[u8]) -> Result<Vec<u8>, String> {
    if (width * height * 4) as usize != rgba_pixels.len() {
        return Err(format!(
            "Pixel buffer size mismatch: expected {} bytes ({}x{}x4), got {} bytes",
            width * height * 4,
            width,
            height,
            rgba_pixels.len()
        ));
    }

    let mut png = Vec::with_capacity((width * height * 2) as usize + 1024);

    // PNG Signature
    png.extend_from_slice(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR Chunk
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // Bit depth: 8 bits per channel
    ihdr.push(6); // Color type: 6 = RGBA
    ihdr.push(0); // Compression method: 0 (Deflate)
    ihdr.push(0); // Filter method: 0 (Standard)
    ihdr.push(0); // Interlace method: 0 (None)
    write_chunk(&mut png, b"IHDR", &ihdr);

    // IDAT Chunk: Scanlines filtered with Filter 0 (None)
    let stride = (width * 4) as usize;
    let mut raw_scanlines = Vec::with_capacity((stride + 1) * height as usize);

    for y in 0..height as usize {
        raw_scanlines.push(0x00); // Filter byte 0 (None)
        let row_start = y * stride;
        let row_end = row_start + stride;
        raw_scanlines.extend_from_slice(&rgba_pixels[row_start..row_end]);
    }

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(&raw_scanlines).map_err(|e| format!("Zlib compression failed: {}", e))?;
    let compressed_idat = encoder.finish().map_err(|e| format!("Zlib finish failed: {}", e))?;

    write_chunk(&mut png, b"IDAT", &compressed_idat);

    // IEND Chunk
    write_chunk(&mut png, b"IEND", &[]);

    Ok(png)
}
