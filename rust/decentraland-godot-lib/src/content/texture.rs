use godot::{
    bind::GodotClass,
    builtin::{meta::ToGodot, GString},
    engine::{global::Error, DirAccess, Image, ImageTexture},
    obj::Gd,
};
use tokio::io::AsyncReadExt;

use crate::godot_classes::promise::Promise;

use super::{
    bytes::fast_create_packed_byte_array_from_vec,
    content_provider::ContentProviderContext,
    download::fetch_resource_or_wait,
    thread_safety::{reject_promise, resolve_promise, GodotSingleThreadSafety},
};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct TextureEntry {
    #[var]
    pub image: Gd<Image>,
    #[var]
    pub texture: Gd<ImageTexture>,
}

pub async fn load_png_texture(
    url: String,
    file_hash: String,
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    ctx: ContentProviderContext,
) {
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    match fetch_resource_or_wait(&url, &file_hash, &absolute_file_path, ctx.clone()).await {
        Ok(_) => {}
        Err(err) => {
            reject_promise(
                get_promise,
                format!("Error downloading png texture {file_hash}: {:?}", err),
            );
            return;
        }
    }

    let mut file = match tokio::fs::File::open(&absolute_file_path).await {
        Ok(file) => file,
        Err(err) => {
            reject_promise(
                get_promise,
                format!(
                    "Error opening texture file {}: {:?}",
                    absolute_file_path, err
                ),
            );
            return;
        }
    };

    let mut bytes_vec = Vec::new();
    if let Err(err) = file.read_to_end(&mut bytes_vec).await {
        reject_promise(
            get_promise,
            format!(
                "Error reading texture file {}: {:?}",
                absolute_file_path, err
            ),
        );
        return;
    }

    let Some(_thread_safe_check) = GodotSingleThreadSafety::acquire_owned(&ctx).await else {
        reject_promise(
            get_promise,
            "Error loading gltf when acquiring thread safety".to_string(),
        );
        return;
    };

    let bytes = fast_create_packed_byte_array_from_vec(&bytes_vec);

    let mut image = Image::new();
    let err = image.load_png_from_buffer(bytes);
    if err != Error::OK {
        DirAccess::remove_absolute(GString::from(&absolute_file_path));
        let err = err.to_variant().to::<i32>();
        reject_promise(
            get_promise,
            format!("Error loading texture {absolute_file_path}: {}", err),
        );
        return;
    }

    let Some(mut texture) = ImageTexture::create_from_image(image.clone()) else {
        reject_promise(
            get_promise,
            format!("Error creating texture from image {}", absolute_file_path),
        );
        return;
    };

    texture.set_name(GString::from(&url));

    let texture_entry = Gd::from_init_fn(|_base| TextureEntry { texture, image });
    resolve_promise(get_promise, Some(texture_entry.to_variant()));
}
