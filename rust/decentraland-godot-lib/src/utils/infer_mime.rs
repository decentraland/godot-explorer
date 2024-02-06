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
    let valid_image_type = match image_type {
        0 | 1 | 2 | 3 | 9 | 10 | 11 => true,
        _ => false,
    };

    // Check if the pixel depth is one of the common values
    let valid_pixel_depth = match pixel_depth {
        8 | 16 | 24 | 32 => true,
        _ => false,
    };

    valid_image_type && valid_pixel_depth
}

pub fn is_ktx(buffer: &[u8]) -> bool {
    // KTX file signature
    let signature: [u8; 12] = [
        0xAB, 0x4B, 0x54, 0x58, 0x20, 0x31, 0x31, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
    ];

    // Check if the buffer starts with the KTX signature
    buffer.starts_with(&signature)
}
