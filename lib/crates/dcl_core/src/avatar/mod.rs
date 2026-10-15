//! The avatar rules (D-2): what equipping, unequipping, changing the body shape, colouring,
//! the emote slots and randomizing produce. The engine renders the result; it decides nothing.

mod catalog;
mod rules;
mod taxonomy;

#[cfg(test)]
mod tests;

pub use catalog::{parse_wearables, Catalog, CatalogItem};
pub use rules::{
    equip, randomize, set_body_shape, set_color, set_emote_slot, unequip, AvatarError,
};
pub use taxonomy::{
    base_emotes, is_base_wearable, is_removable, palette, BaseEmote, BASE_AVATARS, BASE_EMOTES,
    EMOTE_SLOTS, FEMALE, MALE,
};
