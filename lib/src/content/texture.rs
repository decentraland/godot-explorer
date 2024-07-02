use crate::{content::texture_compression::ResourceImporterTexture, utils::infer_mime};

use super::{
    content_provider::ContentProviderContext, packed_array::PackedByteArrayFromVec,
    thread_safety::GodotSingleThreadSafety,
};
use godot::{
    bind::GodotClass,
    builtin::{meta::ToGodot, GString, PackedByteArray, Variant, Vector2i},
    engine::{global::Error, image::CompressMode, portable_compressed_texture_2d::CompressionMode, CompressedTexture2D, DirAccess, Image, ImageTexture, PortableCompressedTexture2D, Texture2D},
    obj::Gd,
};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct TextureEntry {
    #[var]
    pub image: Gd<Image>,
    #[var]
    pub texture: Gd<Texture2D>,
    #[var]
    pub original_size: Vector2i,
}

pub async fn load_image_texture(
    url: String,
    file_hash: String,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    let bytes_vec = ctx
        .resource_provider
        .fetch_resource_with_data(&url, &file_hash, &absolute_file_path)
        .await
        .map_err(anyhow::Error::msg)?;

    let _thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed trying to get thread-safe check"))?;

    let bytes = PackedByteArray::from_vec(&bytes_vec);

    let mut image = Image::new();
    let err = if infer_mime::is_png(&bytes_vec) {
        image.load_png_from_buffer(bytes)
    } else if infer_mime::is_jpeg(&bytes_vec) || infer_mime::is_jpeg2000(&bytes_vec) {
        image.load_jpg_from_buffer(bytes)
    } else if infer_mime::is_webp(&bytes_vec) {
        image.load_webp_from_buffer(bytes)
    } else if infer_mime::is_tga(&bytes_vec) {
        image.load_tga_from_buffer(bytes)
    } else if infer_mime::is_ktx(&bytes_vec) {
        image.load_ktx_from_buffer(bytes)
    } else if infer_mime::is_bmp(&bytes_vec) {
        image.load_bmp_from_buffer(bytes)
    } else if infer_mime::is_svg(&bytes_vec) {
        image.load_svg_from_buffer(bytes)
    } else {
        // if we don't know the format... we try to load as png
        image.load_png_from_buffer(bytes)
    };

    if err != Error::OK {
        DirAccess::remove_absolute(GString::from(&absolute_file_path));
        let err = err.to_variant().to::<i32>();
        return Err(anyhow::Error::msg(format!(
            "Error loading texture {absolute_file_path}: {}",
            err
        )));
    }

    let original_size = image.get_size();

    let max_size = ctx.texture_quality.to_max_size();
    let mut texture: Gd<Texture2D> = if std::env::consts::OS == "ios" {
        create_compressed_texture(&mut image, &ctx.content_folder.to_string(), max_size)
    } else {
        resize_image(&mut image, max_size);
        let texture = ImageTexture::create_from_image(image.clone()).ok_or(anyhow::Error::msg(
            format!("Error creating texture from image {}", absolute_file_path),
        ))?;
        texture.upcast()
    };

    texture.set_name(GString::from(&url));

    let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
        image,
        texture,
        original_size
    });

    Ok(Some(texture_entry.to_variant()))
}

pub fn create_compressed_texture(image: &mut Gd<Image>, content_folder: &String, max_size: i32) -> Gd<Texture2D> {
    if std::env::consts::OS == "ios" && max_size != 256 {
        resize_image(image, max_size);

        if !image.is_compressed() {
            image.compress(CompressMode::COMPRESS_ETC2);
        }

        let mut texture = PortableCompressedTexture2D::new();
        texture.create_from_image(image.clone(), CompressionMode::COMPRESSION_MODE_ETC2);
        texture.upcast()

        /*
        let hash = ethers_core::utils::keccak256(&image.get_data().as_slice());
        
        let hash_str = ethers_core::utils::hex::encode(hash);
        
        let absolute_file_path = format!("{}{}.ctex", content_folder, hash_str);
        
        // Check if the file already exists
        if std::path::Path::new(&absolute_file_path).exists() {
            // Load the existing compressed texture
            let mut texture = CompressedTexture2D::new();
            texture.load(absolute_file_path.into_godot());
            return texture.upcast();
        }

        let _ = ResourceImporterTexture::save_ctex(image.clone(), absolute_file_path.clone(), false);
        
        let mut texture = CompressedTexture2D::new();
        texture.load(absolute_file_path.into_godot());
        texture.upcast()
        */
    } else {
        resize_image(image, max_size);
        let texture = ImageTexture::create_from_image(image.clone()).expect("Error creating texture from image");
        texture.upcast()
    }
}

pub fn resize_image(image: &mut Gd<Image>, max_size: i32) -> bool {
    let image_width = image.get_width();
    let image_height = image.get_height();
    let resized = if image_width > image_height {
        if image_width > max_size {
            image.resize(max_size, (image_height * max_size) / image_width);
            tracing::debug!(
                "Resize! {}x{} to {}x{}",
                image_width,
                image_height,
                image.get_width(),
                image.get_height()
            );
            true
        } else {
            false
        }
    } else if image_height > max_size {
        image.resize((image_width * max_size) / image_height, max_size);
        tracing::debug!(
            "Resize! {}x{} to {}x{}",
            image_width,
            image_height,
            image.get_width(),
            image.get_height()
        );
        true
    } else {
        false
    };

    return resized;
}