use godot::{
    builtin::{meta::ToGodot, GString},
    engine::{file_access::ModeFlags, AudioStream, AudioStreamMp3, AudioStreamWav, FileAccess},
    obj::Gd,
};

use crate::{
    godot_classes::promise::Promise,
    http_request::request_response::{RequestOption, ResponseType},
};

use super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::ContentProviderContext,
    file_string::get_extension,
    thread_safety::{reject_promise, resolve_promise},
};

pub async fn load_audio(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    get_promise: impl Fn() -> Option<Gd<Promise>>,
    ctx: ContentProviderContext,
) {
    let extension = get_extension(&file_path);
    if ["wav", "ogg", "mp3"].contains(&extension.as_str()) {
        reject_promise(
            get_promise,
            format!("Audio {} unrecognized format", file_path),
        );
        return;
    }

    let Some(file_hash) = content_mapping.content.get(&file_path) else {
        reject_promise(
            get_promise,
            "File not found in the content mappings".to_string(),
        );
        return;
    };

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
                    format!(
                        "Error downloading audio {file_hash} ({file_path}): {:?}",
                        err
                    ),
                );
                return;
            }
        }
    }

    let Some(file) = FileAccess::open(GString::from(&absolute_file_path), ModeFlags::READ) else {
        reject_promise(
            get_promise,
            format!("Error opening audio file {}", absolute_file_path),
        );
        return;
    };

    let bytes = file.get_buffer(file.get_length() as i64);
    let audio_stream: Option<Gd<AudioStream>> = match extension.as_str() {
        ".wav" => {
            let mut audio_stream = AudioStreamWav::new();
            audio_stream.set_data(bytes);
            Some(audio_stream.upcast())
        }
        // ".ogg" => {
        //     let audio_stream = AudioStreamOggVorbis::new();
        //     // audio_stream.set_(bytes);
        //     audio_stream.upcast()
        // }
        ".mp3" => {
            let mut audio_stream = AudioStreamMp3::new();
            audio_stream.set_data(bytes);
            Some(audio_stream.upcast())
        }
        _ => None,
    };

    let Some(audio_stream) = audio_stream else {
        reject_promise(
            get_promise,
            format!("Error creating audio stream for {}", absolute_file_path),
        );
        return;
    };

    resolve_promise(get_promise, Some(audio_stream.to_variant()));
}
