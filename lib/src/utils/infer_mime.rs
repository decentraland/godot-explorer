/// Returns whether a buffer is JPEG image data.
pub fn is_jpeg(buf: &[u8]) -> bool {
    buf.len() > 2 && buf[0] == 0xFF && buf[1] == 0xD8 && buf[2] == 0xFF
}

/// Returns whether a buffer is jpg2 image data.
pub fn is_jpeg2000(buf: &[u8]) -> bool {
    buf.len() > 12
        && buf[0] == 0x0
        && buf[1] == 0x0
        && buf[2] == 0x0
        && buf[3] == 0xC
        && buf[4] == 0x6A
        && buf[5] == 0x50
        && buf[6] == 0x20
        && buf[7] == 0x20
        && buf[8] == 0xD
        && buf[9] == 0xA
        && buf[10] == 0x87
        && buf[11] == 0xA
        && buf[12] == 0x0
}

/// Returns whether a buffer is PNG image data.
pub fn is_png(buf: &[u8]) -> bool {
    buf.len() > 3 && buf[0] == 0x89 && buf[1] == 0x50 && buf[2] == 0x4E && buf[3] == 0x47
}

/// Returns whether a buffer is GIF image data.
pub fn is_gif(buf: &[u8]) -> bool {
    buf.len() > 2 && buf[0] == 0x47 && buf[1] == 0x49 && buf[2] == 0x46
}

/// Returns whether a buffer is WEBP image data.
pub fn is_webp(buf: &[u8]) -> bool {
    buf.len() > 11 && buf[8] == 0x57 && buf[9] == 0x45 && buf[10] == 0x42 && buf[11] == 0x50
}

/// Returns whether a buffer is an ANIMATED WEBP image.
/// Animated WebP can be detected by:
/// 1. VP8X extended format with animation flag set (bit 1 of flags)
/// 2. Presence of ANIM or ANMF chunks
pub fn is_animated_webp(buf: &[u8]) -> bool {
    // Must be a valid WebP first
    if !is_webp(buf) {
        return false;
    }

    // Check for VP8X chunk with animation flag
    if buf.len() >= 21 {
        // Check for VP8X chunk (extended format)
        let is_vp8x = buf[12] == 0x56  // 'V'
            && buf[13] == 0x50         // 'P'
            && buf[14] == 0x38         // '8'
            && buf[15] == 0x58; // 'X'

        if is_vp8x {
            // Check animation flag (bit 1 of flags byte at offset 20)
            if (buf[20] & 0x02) != 0 {
                return true;
            }
        }
    }

    // Also check for ANIM or ANMF chunks in the file
    // Search in first 1KB to avoid scanning entire large files
    let search_limit = buf.len().min(1024);
    for i in 12..search_limit.saturating_sub(3) {
        // Check for "ANIM" chunk (0x41 0x4E 0x49 0x4D)
        if buf[i] == 0x41 && buf[i + 1] == 0x4E && buf[i + 2] == 0x49 && buf[i + 3] == 0x4D {
            return true;
        }
        // Check for "ANMF" chunk (0x41 0x4E 0x4D 0x46)
        if buf[i] == 0x41 && buf[i + 1] == 0x4E && buf[i + 2] == 0x4D && buf[i + 3] == 0x46 {
            return true;
        }
    }

    false
}

/// Returns whether a buffer is BMP image data.
pub fn is_bmp(buf: &[u8]) -> bool {
    buf.len() > 1 && buf[0] == 0x42 && buf[1] == 0x4D
}

pub fn is_svg(buffer: &[u8]) -> bool {
    // Convert the buffer to a string slice for easier processing
    // Note: This simplistic approach assumes the SVG is encoded in UTF-8 and the relevant tags are in the beginning of the file.
    // For real-world applications, consider more robust XML parsing methods to handle different encodings and cases.
    let content = match std::str::from_utf8(buffer) {
        Ok(content) => content,
        Err(_) => return false,
    };

    // Check for the XML declaration (optional) and the <svg> start tag
    content.starts_with("<?xml") && content.contains("<svg") || content.starts_with("<svg")
}

pub fn is_tga(buffer: &[u8]) -> bool {
    if buffer.len() < 18 {
        return false; // Buffer is too small to be a valid TGA file
    }

    let image_type = buffer[2];
    let pixel_depth = buffer[16];

    // Check if the image type is within the typical range for TGA files
    let valid_image_type = matches!(image_type, 0 | 1 | 2 | 3 | 9 | 10 | 11);

    // Check if the pixel depth is one of the common values
    let valid_pixel_depth = matches!(pixel_depth, 8 | 16 | 24 | 32);

    valid_image_type && valid_pixel_depth
}

pub fn is_ktx(buffer: &[u8]) -> bool {
    // KTX file magic number (first 4 bytes): «KTX (0xAB 0x4B 0x54 0x58)
    // This matches both KTX1 and KTX2 formats
    buffer.len() >= 4
        && buffer[0] == 0xAB
        && buffer[1] == 0x4B
        && buffer[2] == 0x54
        && buffer[3] == 0x58
}

/// Godot's KTX2 loader converts sRGB-tagged Basis data (UASTC/ETC1S) to linear
/// while decoding, but materials sample albedo as sRGB again, so color textures
/// render dark. Re-tagging the DFD transfer function as linear makes the loader
/// keep the encoded bytes, which is what the glTF/SDK texture contract expects.
pub fn ktx2_keep_srgb_bytes(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    const KHR_DF_TRANSFER_LINEAR: u8 = 1;
    const KHR_DF_TRANSFER_SRGB: u8 = 2;
    // vkFormat at 12, dfdByteOffset at 48; transfer is byte 14 of the DFD
    // (total size, vendor/type, version/size, then model, primaries, transfer).
    let vk_format = bytes
        .get(12..16)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes);
    let dfd_offset = bytes
        .get(48..52)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes);
    if let (Some(0), Some(dfd_offset)) = (vk_format, dfd_offset) {
        let transfer_at = dfd_offset as usize + 14;
        if bytes.get(transfer_at) == Some(&KHR_DF_TRANSFER_SRGB) {
            let mut patched = bytes.to_vec();
            patched[transfer_at] = KHR_DF_TRANSFER_LINEAR;
            return std::borrow::Cow::Owned(patched);
        }
    }
    std::borrow::Cow::Borrowed(bytes)
}

/// Returns whether a buffer is AVIF image data.
/// AVIF uses ISOBMFF container format with ftyp box and 'avif' or 'avis' brand.
pub fn is_avif(buffer: &[u8]) -> bool {
    if buffer.len() < 12 {
        return false;
    }

    // Check for ftyp box at offset 4
    let is_ftyp = buffer[4] == 0x66 // 'f'
        && buffer[5] == 0x74        // 't'
        && buffer[6] == 0x79        // 'y'
        && buffer[7] == 0x70; // 'p'

    if !is_ftyp {
        return false;
    }

    // Check for 'avif' or 'avis' brand at offset 8
    let brand = &buffer[8..12];
    brand == b"avif" || brand == b"avis" || brand == b"mif1"
}

/// Returns whether a buffer is HEIC/HEIF image data.
pub fn is_heic(buffer: &[u8]) -> bool {
    if buffer.len() < 12 {
        return false;
    }

    // Check for ftyp box at offset 4
    let is_ftyp = buffer[4] == 0x66 // 'f'
        && buffer[5] == 0x74        // 't'
        && buffer[6] == 0x79        // 'y'
        && buffer[7] == 0x70; // 'p'

    if !is_ftyp {
        return false;
    }

    // Check for HEIC brands at offset 8
    let brand = &buffer[8..12];
    brand == b"heic" || brand == b"heix" || brand == b"hevc" || brand == b"hevx"
}

#[cfg(test)]
mod ktx2_tests {
    use super::ktx2_keep_srgb_bytes;

    fn header(vk_format: u32, transfer: u8) -> Vec<u8> {
        let mut b = vec![0u8; 0x68 + 44];
        b[..12].copy_from_slice(b"\xABKTX 20\xBB\r\n\x1A\n");
        b[12..16].copy_from_slice(&vk_format.to_le_bytes());
        b[48..52].copy_from_slice(&0x68u32.to_le_bytes());
        b[0x68 + 14] = transfer;
        b
    }

    #[test]
    fn srgb_basis_is_retagged_linear() {
        let input = header(0, 2);
        let out = ktx2_keep_srgb_bytes(&input);
        assert_eq!(out[0x68 + 14], 1);
        assert_eq!(input[0x68 + 14], 2);
    }

    #[test]
    fn linear_and_non_basis_are_untouched() {
        assert_eq!(ktx2_keep_srgb_bytes(&header(0, 1))[0x68 + 14], 1);
        assert_eq!(ktx2_keep_srgb_bytes(&header(43, 2))[0x68 + 14], 2);
        assert_eq!(ktx2_keep_srgb_bytes(b"\xABKTX 20").as_ref(), b"\xABKTX 20");
    }
}
