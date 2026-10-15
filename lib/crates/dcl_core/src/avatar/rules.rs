//! What each edit produces. Every function returns the next avatar and leaves the input alone.

use rand::Rng;

use super::{
    catalog::{Catalog, CatalogItem},
    taxonomy::{
        default_for, is_removable, palette, EMOTE_SLOTS, FEMALE, MALE, REQUIRED_WITH_DEFAULT,
    },
};
use crate::app::{AvatarState, ColorTarget};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvatarError {
    /// The wearables were not loaded, so nothing can be resolved or drawn from them.
    CatalogNotLoaded,
    /// The engine has not reported the avatar yet, so there is nothing to edit.
    NoAvatar,
    /// The catalog does not know the item, so its category is not known either.
    UnknownItem(String),
    /// The item has no representation for the avatar's body shape.
    WrongBodyShape(String),
    InvalidBodyShape(String),
    InvalidColor(String),
    SlotOutOfRange(u32),
}

impl std::fmt::Display for AvatarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CatalogNotLoaded => write!(f, "the wearable catalog is not loaded"),
            Self::NoAvatar => write!(f, "no avatar to edit yet"),
            Self::UnknownItem(urn) => write!(f, "unknown item {urn}"),
            Self::WrongBodyShape(urn) => write!(f, "{urn} does not fit this body shape"),
            Self::InvalidBodyShape(urn) => write!(f, "{urn} is not a body shape"),
            Self::InvalidColor(color) => write!(f, "{color} is not a #rrggbb colour"),
            Self::SlotOutOfRange(slot) => write!(f, "emote slot {slot} is out of range"),
        }
    }
}

impl std::error::Error for AvatarError {}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn category_of<'a>(catalog: &'a Catalog, urn: &str) -> Option<&'a CatalogItem> {
    catalog.find(urn)
}

/// Puts the item on, replacing whatever the avatar wears in the same category.
pub fn equip(
    avatar: &AvatarState,
    catalog: &Catalog,
    urn: &str,
) -> Result<AvatarState, AvatarError> {
    let item = category_of(catalog, urn).ok_or_else(|| AvatarError::UnknownItem(urn.into()))?;
    if !item.fits(&avatar.body_shape) {
        return Err(AvatarError::WrongBodyShape(urn.into()));
    }
    let mut next = avatar.clone();
    next.wearables.retain(|worn| {
        !same(worn, urn)
            && category_of(catalog, worn).is_none_or(|other| other.category != item.category)
    });
    next.wearables.push(urn.to_string());
    Ok(next)
}

/// Takes the item off; the categories the avatar cannot be without stay on.
pub fn unequip(avatar: &AvatarState, catalog: &Catalog, urn: &str) -> AvatarState {
    let mut next = avatar.clone();
    if category_of(catalog, urn).is_some_and(|item| !is_removable(&item.category)) {
        return next;
    }
    next.wearables.retain(|worn| !same(worn, urn));
    next
}

/// Switches the body shape, drops what does not fit it and refills the required categories.
pub fn set_body_shape(
    avatar: &AvatarState,
    catalog: &Catalog,
    urn: &str,
) -> Result<AvatarState, AvatarError> {
    let shape = if same(urn, MALE) {
        MALE
    } else if same(urn, FEMALE) {
        FEMALE
    } else {
        return Err(AvatarError::InvalidBodyShape(urn.into()));
    };
    let mut next = avatar.clone();
    next.body_shape = shape.to_string();
    next.wearables
        .retain(|worn| category_of(catalog, worn).is_none_or(|item| item.fits(shape)));
    for category in REQUIRED_WITH_DEFAULT {
        let present = next
            .wearables
            .iter()
            .any(|worn| category_of(catalog, worn).is_some_and(|item| item.category == category));
        if !present {
            if let Some(default) = default_for(shape, category) {
                next.wearables.push(default.to_string());
            }
        }
    }
    Ok(next)
}

fn is_hex_color(color: &str) -> bool {
    color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn set_color(
    avatar: &AvatarState,
    target: ColorTarget,
    color: &str,
) -> Result<AvatarState, AvatarError> {
    if !is_hex_color(color) {
        return Err(AvatarError::InvalidColor(color.into()));
    }
    let mut next = avatar.clone();
    let color = color.to_ascii_lowercase();
    match target {
        ColorTarget::Skin => next.skin = color,
        ColorTarget::Hair => next.hair = color,
        ColorTarget::Eyes => next.eyes = color,
    }
    Ok(next)
}

pub fn set_emote_slot(
    avatar: &AvatarState,
    slot: u32,
    urn: &str,
) -> Result<AvatarState, AvatarError> {
    let index = slot as usize;
    if index >= EMOTE_SLOTS {
        return Err(AvatarError::SlotOutOfRange(slot));
    }
    let mut next = avatar.clone();
    next.emotes.resize(EMOTE_SLOTS, String::new());
    next.emotes[index] = urn.to_string();
    Ok(next)
}

/// Optional categories appear with this chance; required ones always.
const OPTIONAL_CHANCE: f64 = 0.6;

fn pick<'a, T>(items: &'a [T], rng: &mut impl Rng) -> &'a T {
    &items[rng.gen_range(0..items.len())]
}

fn random_colour(target: ColorTarget, rng: &mut impl Rng) -> String {
    pick(palette(target), rng).to_string()
}

/// A random avatar out of the free base wearables; the emotes stay as they are.
pub fn randomize(
    avatar: &AvatarState,
    catalog: &Catalog,
    rng: &mut impl Rng,
) -> Result<AvatarState, AvatarError> {
    let body_shape = if rng.gen_bool(0.5) { MALE } else { FEMALE };
    let fitting: Vec<&CatalogItem> = catalog
        .items()
        .iter()
        .filter(|item| {
            item.rarity == "base" && item.category != "body_shape" && item.fits(body_shape)
        })
        .collect();
    if fitting.is_empty() {
        return Err(AvatarError::CatalogNotLoaded);
    }
    let mut categories: Vec<&str> = Vec::new();
    for item in &fitting {
        if !categories.contains(&item.category.as_str()) {
            categories.push(&item.category);
        }
    }
    let mut wearables = Vec::new();
    for category in categories {
        let choices: Vec<&CatalogItem> = fitting
            .iter()
            .copied()
            .filter(|item| item.category == category)
            .collect();
        if !is_removable(category) || rng.gen_bool(OPTIONAL_CHANCE) {
            wearables.push(pick(&choices, rng).urn.clone());
        }
    }
    Ok(AvatarState {
        body_shape: body_shape.to_string(),
        wearables,
        emotes: avatar.emotes.clone(),
        skin: random_colour(ColorTarget::Skin, rng),
        hair: random_colour(ColorTarget::Hair, rng),
        eyes: random_colour(ColorTarget::Eyes, rng),
    })
}
