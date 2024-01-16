use godot::{
    builtin::meta::ToGodot,
    engine::{AudioStream, AudioStreamMp3, AudioStreamWav},
    obj::Gd,
};
use tokio::io::AsyncReadExt;

use crate::godot_classes::promise::Promise;

use super::{
    bytes::fast_create_packed_byte_array_from_vec,
    content_mapping::ContentMappingAndUrlRef,
    content_provider::ContentProviderContext,
    download::fetch_resource_or_wait,
    file_string::get_extension,
    thread_safety::{reject_promise, resolve_promise, GodotSingleThreadSafety},
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

    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    match fetch_resource_or_wait(&url, file_hash, &absolute_file_path, ctx.clone()).await {
        Ok(_) => {}
        Err(err) => {
            reject_promise(
                get_promise,
                format!("Error downloading audio {file_hash}: {:?}", err),
            );
            return;
        }
    }

    let mut file = match tokio::fs::File::open(&absolute_file_path).await {
        Ok(file) => file,
        Err(err) => {
            reject_promise(
                get_promise,
                format!("Error opening audio file {}: {:?}", file_path, err),
            );
            return;
        }
    };

    let mut bytes_vec = Vec::new();
    if let Err(err) = file.read_to_end(&mut bytes_vec).await {
        reject_promise(
            get_promise,
            format!("Error reading audio file {}: {:?}", file_path, err),
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
