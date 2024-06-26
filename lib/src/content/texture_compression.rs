use anyhow::Error;
use godot::engine::Image;
use godot::obj::{EngineEnum, Gd};
use std::fs::File;
use std::io::{Write, BufWriter};

pub struct ResourceImporterTexture;

impl ResourceImporterTexture {
    pub fn save_ctex(
        image: Gd<Image>,
        to_path: String,
        streamable: bool,
    ) -> Result<(), Error> {
        if !image.is_compressed() {
            return Err(anyhow::anyhow!("Image should be compressed"));
        }

        let width = image.get_width();
        let height = image.get_height();
        let mipmap_count = image.get_mipmap_count();
        let format = image.get_format().ord();
        let data = image.get_data();
        let data_vec = data.as_slice();

        // Perform the synchronous file operations
        let file = File::create(&to_path).map_err(|_| {
            anyhow::anyhow!("Failed to open file")
        })?;
        let mut writer = BufWriter::new(file);

        writer.write_all(b"GST2").map_err(|_| {
            anyhow::anyhow!("Failed to write header")
        })?;

        let format_version: u32 = 1; // Replace with actual CompressedTexture2D::FORMAT_VERSION
        let mut flags: u32 = 0;

        if streamable {
            flags |= 1; // Replace with actual CompressedTexture2D::FORMAT_BIT_STREAM
        }

        let reserved: [u32; 4] = [0; 4];

        writer.write_all(&format_version.to_le_bytes())?;
        writer.write_all(&(width as u32).to_le_bytes())?;
        writer.write_all(&(height as u32).to_le_bytes())?;
        writer.write_all(&flags.to_le_bytes())?;
        for r in reserved.iter() {
            writer.write_all(&r.to_le_bytes())?;
        }

        writer.write_all(&0u32.to_le_bytes())?; // Replace with actual CompressedTexture2D::DATA_FORMAT_IMAGE
        writer.write_all(&(width as u16).to_le_bytes())?;
        writer.write_all(&(height as u16).to_le_bytes())?;
        writer.write_all(&(mipmap_count as u32).to_le_bytes())?;
        writer.write_all(&(format as u32).to_le_bytes())?;

        writer.write_all(data_vec)?;

        Ok(())
    }

    pub fn is_ctex(data: &[u8]) -> bool {
        const CTEX_MAGIC: &[u8; 4] = b"GST2";
        
        // Check if the data has at least 4 bytes
        if data.len() < 4 {
            return false;
        }
        
        // Check the first 4 bytes
        &data[0..4] == CTEX_MAGIC
    }
}
