use crate::utils::infer_mime;

use super::{
    bytes::fast_create_packed_byte_array_from_vec, content_provider::ContentProviderContext,
    download::fetch_resource_or_wait, thread_safety::GodotSingleThreadSafety,
};
use godot::{
    bind::GodotClass,
    builtin::{meta::ToGodot, GString, Variant, Vector2i},
    engine::{global::Error, DirAccess, Image, ImageTexture},
    obj::Gd,
};
use tokio::io::AsyncReadExt;

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct TextureEntry {
    #[var]
    pub image: Gd<Image>,
    #[var]
    pub texture: Gd<ImageTexture>,
    #[var]
    pub original_size: Vector2i,
}

pub async fn load_image_texture(
    url: String,
    file_hash: String,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    fetch_resource_or_wait(&url, &file_hash, &absolute_file_path, ctx.clone())
        .await
        .map_err(anyhow::Error::msg)?;

    let mut file = tokio::fs::File::open(&absolute_file_path).await?;
    let mut bytes_vec = Vec::new();
    file.read_to_end(&mut bytes_vec).await?;

    let _thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed trying to get thread-safe check"))?;

    let bytes = fast_create_packed_byte_array_from_vec(&bytes_vec);

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
    resize_image(&mut image, max_size);

    let mut texture = ImageTexture::create_from_image(image.clone()).ok_or(anyhow::Error::msg(
        format!("Error creating texture from image {}", absolute_file_path),
    ))?;

    texture.set_name(GString::from(&url));

    let texture_entry = Gd::from_init_fn(|_base| TextureEntry {
        texture,
        image,
        original_size,
    });

    Ok(Some(texture_entry.to_variant()))
}

pub fn resize_image(image: &mut Gd<Image>, max_size: i32) -> bool {
    let image_width = image.get_width();
    let image_height = image.get_height();
    if image_width > image_height {
        if image_width > max_size {
            image.resize(max_size, (image_height * max_size) / image_width);
            tracing::debug!(
                "Resize! {}x{} to {}x{}",
                image_width,
                image_height,
                image.get_width(),
                image.get_height()
            );
            return true;
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
        return true;
    }
    false
}
