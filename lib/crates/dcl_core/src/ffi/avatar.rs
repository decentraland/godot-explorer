//! FFI for the avatar rules: the wearable catalog the edits resolve against and the fixed data the
//! editor shows. The edits themselves are `Control`s the core computes (`app`).

use super::{core, CoreError};
use crate::{
    app::ColorTarget,
    avatar::{self, BaseEmote, Catalog, CatalogItem},
    profile::{Endpoints, ReqwestTransport},
};

/// Loads the base wearables once, makes them the catalog the avatar edits use and returns them.
#[uniffi::export]
pub async fn load_avatar_catalog() -> Result<Vec<CatalogItem>, CoreError> {
    let core = core()?;
    let loaded = core.with_app(|app| app.catalog().items().to_vec());
    if !loaded.is_empty() {
        return Ok(loaded);
    }
    let items = core
        .handle()
        .spawn(
            async move { avatar::base_catalog(&ReqwestTransport::new(), &Endpoints::live()).await },
        )
        .await
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?
        .map_err(|e| CoreError::InvalidCall {
            detail: e.to_string(),
        })?;
    let catalog = Catalog::new(items.clone());
    core.with_app(|app| app.set_catalog(catalog));
    Ok(items)
}

/// The colour swatches the editor offers.
#[uniffi::export]
pub fn avatar_palette(target: ColorTarget) -> Vec<String> {
    avatar::palette(target)
        .iter()
        .map(|c| c.to_string())
        .collect()
}

/// The emotes every account has.
#[uniffi::export]
pub fn base_emotes() -> Vec<BaseEmote> {
    avatar::base_emotes()
}

/// Whether the avatar may go without an item of this category.
#[uniffi::export]
pub fn is_removable_category(category: String) -> bool {
    avatar::is_removable(&category)
}
