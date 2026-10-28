//! Which wearables an avatar shows: the category rules, hiding priority and required fallbacks.

use godot::prelude::*;

use crate::avatars::item::DclItemEntityDefinition;
use crate::godot_classes::dcl_global::DclGlobal;

pub mod category {
    pub const EYES: &str = "eyes";
    pub const EYEBROWS: &str = "eyebrows";
    pub const MOUTH: &str = "mouth";
    pub const FACIAL_HAIR: &str = "facial_hair";
    pub const HAIR: &str = "hair";
    pub const HEAD: &str = "head";
    pub const BODY_SHAPE: &str = "body_shape";
    pub const UPPER_BODY: &str = "upper_body";
    pub const LOWER_BODY: &str = "lower_body";
    pub const FEET: &str = "feet";
    pub const EARRING: &str = "earring";
    pub const EYEWEAR: &str = "eyewear";
    pub const HAT: &str = "hat";
    pub const HELMET: &str = "helmet";
    pub const MASK: &str = "mask";
    pub const TIARA: &str = "tiara";
    pub const TOP_HEAD: &str = "top_head";
    pub const SKIN: &str = "skin";
    pub const HANDS: &str = "hands";
    pub const HANDS_WEAR: &str = "hands_wear";

    /// Highest priority first: a wearable earlier in the list wins over the ones it hides.
    pub const HIDING_PRIORITY: [&str; 18] = [
        SKIN,
        UPPER_BODY,
        HANDS_WEAR,
        LOWER_BODY,
        FEET,
        HELMET,
        HAT,
        TOP_HEAD,
        MASK,
        EYEWEAR,
        EARRING,
        TIARA,
        HAIR,
        EYEBROWS,
        EYES,
        MOUTH,
        FACIAL_HAIR,
        BODY_SHAPE,
    ];

    pub const REQUIRED: [&str; 3] = [EYES, MOUTH, EYEBROWS];
}

const MALE: &str = "urn:decentraland:off-chain:base-avatars:BaseMale";
const FEMALE: &str = "urn:decentraland:off-chain:base-avatars:BaseFemale";

/// The wearable that fills a required category the profile left empty.
fn default_wearable(body_shape_id: &str, category: &str) -> Option<&'static str> {
    let male = body_shape_id == MALE;
    if !male && body_shape_id != FEMALE {
        return None;
    }
    Some(match (male, category) {
        (true, category::EYES) => "urn:decentraland:off-chain:base-avatars:eyes_00",
        (true, category::EYEBROWS) => "urn:decentraland:off-chain:base-avatars:eyebrows_00",
        (true, category::MOUTH) => "urn:decentraland:off-chain:base-avatars:mouth_00",
        (false, category::EYES) => "urn:decentraland:off-chain:base-avatars:f_eyes_00",
        (false, category::EYEBROWS) => "urn:decentraland:off-chain:base-avatars:f_eyebrows_00",
        (false, category::MOUTH) => "urn:decentraland:off-chain:base-avatars:f_mouth_00",
        _ => return None,
    })
}

pub fn is_texture(category: &str) -> bool {
    matches!(
        category,
        category::EYES | category::EYEBROWS | category::MOUTH
    )
}

/// Categories the equipped wearables hide, with `force_render` removed.
/// `hides_of` gives the hide list of the wearable equipped in a category, `None` when empty.
/// With `skip_compute_hidden` a hidden wearable still contributes its own hides.
pub fn compose_hidden_categories(
    hides_of: impl Fn(&str) -> Option<Vec<String>>,
    force_render: &[String],
    skip_compute_hidden: bool,
) -> Vec<String> {
    let mut hidden: Vec<String> = Vec::new();
    for priority_category in category::HIDING_PRIORITY {
        if !skip_compute_hidden && hidden.iter().any(|h| h == priority_category) {
            continue;
        }
        let Some(list) = hides_of(priority_category) else {
            continue;
        };
        for to_hide in list {
            if !hidden.contains(&to_hide) {
                hidden.push(to_hide);
            }
        }
    }
    hidden.retain(|category| !force_render.contains(category));
    hidden
}

/// Equipped items in insertion order (the order the meshes get added in).
#[derive(Default, Clone)]
pub struct ByCategory(Vec<(String, Gd<DclItemEntityDefinition>)>);

impl ByCategory {
    pub fn get(&self, category: &str) -> Option<&Gd<DclItemEntityDefinition>> {
        self.0.iter().find(|(c, _)| c == category).map(|(_, w)| w)
    }

    pub fn contains(&self, category: &str) -> bool {
        self.get(category).is_some()
    }

    pub fn insert(&mut self, category: &str, item: Gd<DclItemEntityDefinition>) {
        match self.0.iter_mut().find(|(c, _)| c == category) {
            Some(slot) => slot.1 = item,
            None => self.0.push((category.to_string(), item)),
        }
    }

    pub fn remove(&mut self, category: &str) {
        self.0.retain(|(c, _)| c != category);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, Gd<DclItemEntityDefinition>)> {
        self.0.iter()
    }
}

#[derive(Default)]
pub struct CuratedWearableList {
    pub by_category: ByCategory,
    pub hidden_categories: Vec<String>,
    pub need_to_fetch: Vec<String>,
}

fn wearable(id: &str) -> Option<Gd<DclItemEntityDefinition>> {
    let mut provider = DclGlobal::singleton().bind().get_content_provider();
    let wearable = provider.bind_mut().get_wearable(id.into());
    wearable
}

fn is_valid(item: &Option<Gd<DclItemEntityDefinition>>, body_shape_id: &str) -> bool {
    DclItemEntityDefinition::is_valid_wearable(item.clone(), body_shape_id.to_string(), false)
}

fn hides_in(by_category: &ByCategory, body_shape_id: &str, category: &str) -> Option<Vec<String>> {
    let list: Vec<String> = by_category
        .get(category)?
        .bind()
        .get_hides_list(body_shape_id.to_string())
        .as_slice()
        .iter()
        .map(|s| s.to_string())
        .collect();
    (!list.is_empty()).then_some(list)
}

/// The wearables to show, their hidden categories, and the fallbacks still to download.
pub fn get_curated_wearable_list(
    body_shape_id: &str,
    wearables: &[String],
    force_render: &[String],
    show_only_wearables: bool,
) -> CuratedWearableList {
    let mut ret = CuratedWearableList::default();

    let body_shape = wearable(body_shape_id);
    if !is_valid(&body_shape, body_shape_id) {
        return ret;
    }
    if let Some(body_shape) = body_shape {
        ret.by_category.insert(category::BODY_SHAPE, body_shape);
    }

    for wearable_id in wearables {
        let item = wearable(wearable_id);
        if is_valid(&item, body_shape_id) {
            if let Some(item) = item {
                let category = item.bind().get_category().to_string();
                if !ret.by_category.contains(&category) {
                    ret.by_category.insert(&category, item);
                }
            }
        } else {
            godot_error!("invalid wearable {wearable_id} for body_shape {body_shape_id}");
        }
    }

    let unused = compose_hidden_categories(
        |c| hides_in(&ret.by_category, body_shape_id, c),
        force_render,
        false,
    );
    for hide_category in &unused {
        ret.by_category.remove(hide_category);
    }

    ret.hidden_categories = compose_hidden_categories(
        |c| hides_in(&ret.by_category, body_shape_id, c),
        force_render,
        true,
    );

    if show_only_wearables {
        // Explicit wearables render; the body parts and facial features fallbacks do not.
        let explicit: Vec<String> = wearables
            .iter()
            .filter_map(|id| wearable(id))
            .map(|item| item.bind().get_category().to_string())
            .collect();
        for category in [
            category::UPPER_BODY,
            category::LOWER_BODY,
            category::FEET,
            category::HANDS,
            category::HANDS_WEAR,
            category::HEAD,
            category::HAIR,
            category::FACIAL_HAIR,
            category::EYES,
            category::EYEBROWS,
            category::MOUTH,
            category::SKIN,
        ] {
            let name = category.to_string();
            if !explicit.contains(&name) && !ret.hidden_categories.contains(&name) {
                ret.hidden_categories.push(name);
            }
        }
    }

    ret.need_to_fetch = set_fallback_for_missing_needed_categories(
        body_shape_id,
        &mut ret.by_category,
        &ret.hidden_categories,
    );
    ret
}

fn set_fallback_for_missing_needed_categories(
    body_shape_id: &str,
    by_category: &mut ByCategory,
    hidden_categories: &[String],
) -> Vec<String> {
    let mut need_to_fetch = Vec::new();
    for needed in category::REQUIRED {
        if hidden_categories.iter().any(|h| h == needed) || by_category.contains(needed) {
            continue;
        }
        let Some(fallback_id) = default_wearable(body_shape_id, needed) else {
            continue;
        };
        let fallback = wearable(fallback_id);
        if is_valid(&fallback, body_shape_id) {
            if let Some(fallback) = fallback {
                by_category.insert(needed, fallback);
            }
        } else {
            need_to_fetch.push(fallback_id.to_string());
        }
    }
    need_to_fetch
}

/// Hash of the item's main file for the body shape, empty when it has no representation.
pub fn get_item_main_file_hash(item: &Gd<DclItemEntityDefinition>, body_shape_id: &str) -> String {
    let item = item.bind();
    if !item.has_representation(body_shape_id.to_string()) {
        return String::new();
    }
    let main_file = item.get_representation_main_file(body_shape_id.to_string());
    let mapping = item.get_content_mapping();
    let hash = mapping.bind().get_hash(main_file);
    hash.to_string()
}

/// Texture hashes of an eyes/eyebrows/mouth wearable: the main texture, then its mask.
pub fn get_wearable_facial_hashes(
    item: Option<&Gd<DclItemEntityDefinition>>,
    body_shape_id: &str,
) -> Vec<String> {
    let Some(item) = item else {
        return Vec::new();
    };
    let item = item.bind();
    if !is_texture(&item.get_category().to_string())
        || !item.has_representation(body_shape_id.to_string())
    {
        return Vec::new();
    }
    let main_file = item.get_representation_main_file(body_shape_id.to_string());
    let mapping = item.get_content_mapping();
    let mapping = mapping.bind();
    let files: Vec<String> = mapping
        .get_files()
        .as_slice()
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut main_hash = mapping.get_hash(main_file).to_string();
    if main_hash.is_empty() {
        main_hash = files
            .iter()
            .find(|f| f.ends_with(".png") && !f.ends_with("_mask.png"))
            .map(|f| mapping.get_hash(f.as_str().into()).to_string())
            .unwrap_or_default();
    }
    if main_hash.is_empty() {
        return Vec::new();
    }
    let mask_hash = files
        .iter()
        .find(|f| f.ends_with("_mask.png"))
        .map(|f| mapping.get_hash(f.as_str().into()).to_string())
        .unwrap_or_default();
    if mask_hash.is_empty() {
        vec![main_hash]
    } else {
        vec![main_hash, mask_hash]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn hides(map: &[(&str, &[&str])]) -> impl Fn(&str) -> Option<Vec<String>> {
        let map: HashMap<String, Vec<String>> = map
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect();
        move |category| map.get(category).cloned().filter(|l| !l.is_empty())
    }

    #[test]
    fn priority_wearable_hides_what_it_covers() {
        let hidden = compose_hidden_categories(
            hides(&[("hat", &["hair"]), ("hair", &["eyebrows"])]),
            &[],
            false,
        );
        // The hat hides the hair, so the hair no longer contributes its own hides.
        assert_eq!(hidden, vec!["hair".to_string()]);
    }

    #[test]
    fn skip_compute_hidden_keeps_nested_hides() {
        let hidden = compose_hidden_categories(
            hides(&[("hat", &["hair"]), ("hair", &["eyebrows"])]),
            &[],
            true,
        );
        assert_eq!(hidden, vec!["hair".to_string(), "eyebrows".to_string()]);
    }

    #[test]
    fn force_render_wins() {
        let hidden = compose_hidden_categories(
            hides(&[("hat", &["hair", "eyes"])]),
            &["hair".to_string()],
            false,
        );
        assert_eq!(hidden, vec!["eyes".to_string()]);
    }

    #[test]
    fn fallbacks_follow_the_body_shape() {
        assert_eq!(
            default_wearable(MALE, category::EYES),
            Some("urn:decentraland:off-chain:base-avatars:eyes_00")
        );
        assert_eq!(
            default_wearable(FEMALE, category::MOUTH),
            Some("urn:decentraland:off-chain:base-avatars:f_mouth_00")
        );
        assert_eq!(default_wearable("custom", category::EYES), None);
        assert_eq!(default_wearable(MALE, category::HAT), None);
    }

    #[test]
    fn eyes_eyebrows_mouth_are_textures() {
        assert!(is_texture("eyes") && is_texture("eyebrows") && is_texture("mouth"));
        assert!(!is_texture("hair"));
    }
}
