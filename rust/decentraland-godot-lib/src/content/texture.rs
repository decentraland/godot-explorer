use crate::utils::infer_mime;

use super::{
    bytes::fast_create_packed_byte_array_from_vec, content_provider::ContentProviderContext,
    download::fetch_resource_or_wait, thread_safety::GodotSingleThreadSafety,
};
use godot::{
    bind::GodotClass,
    builtin::{meta::ToGodot, GString, Variant},
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

    let mut texture = ImageTexture::create_from_image(image.clone()).ok_or(anyhow::Error::msg(
        format!("Error creating texture from image {}", absolute_file_path),
    ))?;
    texture.set_name(GString::from(&url));

    let texture_entry = Gd::from_init_fn(|_base| TextureEntry { texture, image });

    Ok(Some(texture_entry.to_variant()))
}
