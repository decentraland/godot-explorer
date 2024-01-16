use godot::obj::Gd;

use crate::godot_classes::promise::Promise;

use super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::ContentProviderContext,
    download::fetch_resource_or_wait,
    thread_safety::{reject_promise, resolve_promise},
};

pub async fn download_video(
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    ctx: ContentProviderContext,
) {
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    match fetch_resource_or_wait(&url, &file_hash, &absolute_file_path, ctx.clone()).await {
        Ok(_) => {}
        Err(err) => {
            reject_promise(
                get_promise,
                format!("Error downloading video {file_hash}: {:?}", err),
            );
            return;
        }
    }

    resolve_promise(get_promise, None);
}
