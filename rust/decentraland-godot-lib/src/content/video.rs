use godot::{
    builtin::GString,
    engine::{file_access::ModeFlags, FileAccess},
    obj::Gd,
};

use crate::{
    godot_classes::promise::Promise,
    http_request::request_response::{RequestOption, ResponseType},
};

use super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::ContentProviderContext,
    thread_safety::{reject_promise, resolve_promise},
};

pub async fn download_video(
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    ctx: ContentProviderContext,
) {
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    if !FileAccess::file_exists(GString::from(&absolute_file_path)) {
        let request = RequestOption::new(
            0,
            format!("{}{}", content_mapping.base_url, file_hash),
            http::Method::GET,
            ResponseType::ToFile(absolute_file_path.clone()),
            None,
            None,
            None,
        );

        match ctx.http_queue_requester.request(request, 0).await {
            Ok(_response) => {}
            Err(err) => {
                reject_promise(
                    get_promise,
                    format!("Error downloading video {file_hash}: {:?}", err),
                );
                return;
            }
        }
    }

    let Some(_file) = FileAccess::open(GString::from(&absolute_file_path), ModeFlags::READ) else {
        reject_promise(
            get_promise,
            format!("Error opening video file {}", absolute_file_path),
        );
        return;
    };

    resolve_promise(get_promise, None);
}
