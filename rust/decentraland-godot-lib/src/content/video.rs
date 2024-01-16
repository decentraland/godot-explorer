use super::{
    content_mapping::ContentMappingAndUrlRef, content_provider::ContentProviderContext,
    download::fetch_resource_or_wait,
};
use godot::builtin::Variant;

pub async fn download_video(
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: ContentProviderContext,
) -> Result<Option<Variant>, anyhow::Error> {
    let url = format!("{}{}", content_mapping.base_url, file_hash);
    let absolute_file_path = format!("{}{}", ctx.content_folder, file_hash);
    fetch_resource_or_wait(&url, &file_hash, &absolute_file_path, ctx.clone())
        .await
        .map_err(anyhow::Error::msg)?;
    Ok(None)
}
