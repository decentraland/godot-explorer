use godot::{
    builtin::{meta::ToGodot, PackedByteArray, Variant},
    engine::{AudioStream, AudioStreamMp3, AudioStreamOggVorbis, AudioStreamWav},
    obj::{Gd, NewGd},
};

use super::{
    content_mapping::ContentMappingAndUrlRef, content_provider::ContentProviderContext,
    file_string::get_extension, packed_array::PackedByteArrayFromVec,
    thread_safety::GodotSingleThreadSafety,
};

pub async fn load_audio(
    file_path: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let extension = get_extension(&file_path);
    if ["wav", "ogg", "mp3"].contains(&extension.as_str()) {
        return Err(anyhow::Error::msg(format!(
            "Audio {} unrecognized format",
            file_path
        )));
    }

    let file_hash = content_mapping
        .get_hash(file_path.as_str())
        .ok_or(anyhow::Error::msg("File not found in the content mappings"))?;

    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);

    let bytes_vec = ctx
        .resource_provider
        .fetch_resource_with_data(&url, file_hash, &absolute_file_path)
        .await
        .map_err(anyhow::Error::msg)?;

    let _thread_safe_check = GodotSingleThreadSafety::acquire_owned(&ctx)
        .await
        .ok_or(anyhow::Error::msg("Failed while trying to "))?;

    let bytes = PackedByteArray::from_vec(&bytes_vec);
    let audio_stream: Option<Gd<AudioStream>> = match extension.as_str() {
        ".wav" => {
            let mut audio_stream = AudioStreamWav::new_gd();
            audio_stream.set_data(bytes);
            Some(audio_stream.upcast())
        }
        ".ogg" => AudioStreamOggVorbis::load_from_buffer(bytes).map(|value| value.upcast()),
        ".mp3" => {
            let mut audio_stream = AudioStreamMp3::new_gd();
            audio_stream.set_data(bytes);
            Some(audio_stream.upcast())
        }
        _ => None,
    };

    let audio_stream = audio_stream.ok_or(anyhow::Error::msg("Error creating audio stream"))?;
    Ok(Some(audio_stream.to_variant()))
}
