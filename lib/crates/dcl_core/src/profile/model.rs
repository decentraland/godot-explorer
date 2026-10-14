//! The profile as the shell sees it, and the document it is read from and written back to.
//!
//! The catalyst's profile JSON carries more than the shell edits (tutorial step, blocked and
//! muted lists, the whole avatar). `ProfileDocument` keeps that JSON whole and changes only the
//! fields an update names, so a deploy never drops what this domain does not model.

use serde_json::{json, Map, Value};

const DEFAULT_AVATAR_BODY: &str = "urn:decentraland:off-chain:base-avatars:BaseFemale";
const DEFAULT_WEARABLES: [&str; 7] = [
    "f_sweater",
    "f_jeans",
    "bun_shoes",
    "standard_hair",
    "f_eyes_01",
    "f_eyebrows_00",
    "f_mouth_00",
];

#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileLink {
    pub title: String,
    pub url: String,
}

/// A profile, public or own. Absent text fields read as empty.
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub address: String,
    pub version: u32,
    pub has_claimed_name: bool,
    pub name: String,
    pub description: String,
    pub country: String,
    pub pronouns: String,
    pub gender: String,
    pub sexual_orientation: String,
    pub relationship_status: String,
    pub employment_status: String,
    pub profession: String,
    pub real_name: String,
    pub hobbies: String,
    pub links: Vec<ProfileLink>,
    /// Face snapshot of profiles that still carry one; empty for ADR-290 profiles, whose image
    /// the profile-images service renders.
    pub profile_picture_url: String,
}

/// The fields of a profile the shell edits; `None` leaves the field as it is. An empty string
/// clears an optional text field, and an empty `links` clears the links.
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub country: Option<String>,
    pub pronouns: Option<String>,
    pub gender: Option<String>,
    pub sexual_orientation: Option<String>,
    pub relationship_status: Option<String>,
    pub employment_status: Option<String>,
    pub profession: Option<String>,
    pub real_name: Option<String>,
    pub hobbies: Option<String>,
    pub has_claimed_name: Option<bool>,
    pub links: Option<Vec<ProfileLink>>,
}

/// The avatar part of a profile (body, wearables, emotes, colors) exactly as the catalyst serves
/// it. Phase 4 replaces it with the avatar domain's typed model; until then the engine owns its
/// meaning and this domain only carries it through a deploy.
#[derive(Clone, Debug, PartialEq)]
pub struct AvatarPayload(pub Value);

/// Text fields: (profile JSON key, whether an empty value is stored as `null`).
/// `name` and `description` are always strings; the rest are `null` when empty.
const TEXT_FIELDS: [(&str, bool); 11] = [
    ("name", false),
    ("description", false),
    ("country", true),
    ("pronouns", true),
    ("gender", true),
    ("sexualOrientation", true),
    ("relationshipStatus", true),
    ("employmentStatus", true),
    ("profession", true),
    ("realName", true),
    ("hobbies", true),
];

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileDocument {
    content: Map<String, Value>,
}

impl ProfileDocument {
    pub fn from_content(content: Map<String, Value>) -> Self {
        Self { content }
    }

    /// The profile of a new account: the default avatar and no text.
    pub fn default_for(address: &str) -> Self {
        let wearables: Vec<String> = DEFAULT_WEARABLES
            .iter()
            .map(|id| format!("urn:decentraland:off-chain:base-avatars:{id}"))
            .collect();
        let color = |r: f64, g: f64, b: f64| json!({ "color": { "r": r, "g": g, "b": b } });
        let content = json!({
            "userId": address,
            "ethAddress": address,
            "name": "",
            "description": "",
            "version": 1,
            "tutorialStep": 0,
            "hasClaimedName": false,
            "hasConnectedWeb3": false,
            "avatar": {
                "bodyShape": DEFAULT_AVATAR_BODY,
                "wearables": wearables,
                "emotes": [],
                "forceRender": [],
                "showOnlyWearables": false,
                "snapshots": null,
                "eyes": color(0.3, 0.22, 0.99),
                "hair": color(0.596, 0.372, 0.215),
                "skin": color(1.0, 0.867, 0.737),
            },
        });
        match content {
            Value::Object(content) => Self { content },
            _ => unreachable!("json! of an object"),
        }
    }

    pub fn content(&self) -> &Map<String, Value> {
        &self.content
    }

    pub fn avatar(&self) -> AvatarPayload {
        AvatarPayload(self.content.get("avatar").cloned().unwrap_or(Value::Null))
    }

    pub fn version(&self) -> u32 {
        self.content
            .get("version")
            .and_then(Value::as_i64)
            .map_or(0, |v| v.max(0) as u32)
    }

    fn get(&self, key: &str) -> &Value {
        self.content.get(key).unwrap_or(&Value::Null)
    }

    fn text(&self, key: &str) -> String {
        self.content
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    pub fn profile(&self) -> Profile {
        let links = self
            .content
            .get("links")
            .and_then(Value::as_array)
            .map(|links| {
                links
                    .iter()
                    .map(|link| ProfileLink {
                        title: link["title"].as_str().unwrap_or_default().to_owned(),
                        url: link["url"].as_str().unwrap_or_default().to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Profile {
            address: self.text("ethAddress"),
            version: self.version(),
            has_claimed_name: self.get("hasClaimedName").as_bool().unwrap_or(false),
            name: self.text("name"),
            description: self.text("description"),
            country: self.text("country"),
            pronouns: self.text("pronouns"),
            gender: self.text("gender"),
            sexual_orientation: self.text("sexualOrientation"),
            relationship_status: self.text("relationshipStatus"),
            employment_status: self.text("employmentStatus"),
            profession: self.text("profession"),
            real_name: self.text("realName"),
            hobbies: self.text("hobbies"),
            links,
            profile_picture_url: self.get("avatar")["snapshots"]["face256"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        }
    }

    /// A copy with the named fields changed.
    pub fn updated(&self, update: &ProfileUpdate) -> Self {
        let values = [
            &update.name,
            &update.description,
            &update.country,
            &update.pronouns,
            &update.gender,
            &update.sexual_orientation,
            &update.relationship_status,
            &update.employment_status,
            &update.profession,
            &update.real_name,
            &update.hobbies,
        ];
        let mut content = self.content.clone();
        for ((key, nullable), value) in TEXT_FIELDS.iter().zip(values) {
            if let Some(value) = value {
                let stored = if *nullable && value.is_empty() {
                    Value::Null
                } else {
                    Value::String(value.clone())
                };
                content.insert((*key).to_owned(), stored);
            }
        }
        if let Some(claimed) = update.has_claimed_name {
            content.insert("hasClaimedName".into(), Value::Bool(claimed));
        }
        if let Some(links) = &update.links {
            let stored = if links.is_empty() {
                Value::Null
            } else {
                Value::Array(
                    links
                        .iter()
                        .map(|l| json!({ "title": l.title, "url": l.url }))
                        .collect(),
                )
            };
            content.insert("links".into(), stored);
        }
        Self { content }
    }

    pub fn with_version(&self, version: u32) -> Self {
        let mut content = self.content.clone();
        content.insert("version".into(), json!(version));
        Self { content }
    }

    pub fn with_address(&self, address: &str) -> Self {
        let mut content = self.content.clone();
        content.insert("userId".into(), json!(address));
        content.insert("ethAddress".into(), json!(address));
        Self { content }
    }

    /// The metadata a deployment carries (ADR-290): no snapshots, and no emote slot left empty,
    /// which the catalyst rejects as an invalid pointer.
    pub fn deployable(&self) -> Map<String, Value> {
        let mut content = self.content.clone();
        if let Some(Value::Object(avatar)) = content.get_mut("avatar") {
            avatar.insert("snapshots".into(), Value::Null);
            if let Some(Value::Array(emotes)) = avatar.get_mut("emotes") {
                emotes.retain(|emote| emote["urn"].as_str().is_some_and(|urn| !urn.is_empty()));
            }
        }
        content
    }
}
