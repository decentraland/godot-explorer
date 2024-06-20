use super::{content_mapping::ContentMappingAndUrlRef, content_provider::ContentProviderContext};
use godot::builtin::Variant;

pub async fn download_video(
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    ctx.resource_provider
        .fetch_resource(&url, &file_hash, &absolute_file_path)
        .await
        .map_err(anyhow::Error::msg)?;
    Ok(None)
}
