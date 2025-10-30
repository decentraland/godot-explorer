use godot::{
    builtin::{Array, Dictionary, GString},
    obj::Gd,
    prelude::*,
};

use crate::comms::profile::{ProfileLink, UserProfile};

use super::avatar_type::DclAvatarWireFormat;

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct DclUserProfile {
    pub inner: UserProfile,
}

impl DclUserProfile {
    pub fn from_gd(inner: UserProfile) -> Gd<Self> {
        Gd::from_init_fn(|_base| Self { inner })
    }
}

#[godot_api]
impl DclUserProfile {
    #[func]
    fn equal(&self, other: Gd<DclUserProfile>) -> bool {
        self.inner == other.bind().inner
    }

    #[func]
    fn duplicated(&self) -> Gd<DclUserProfile> {
        Self::from_gd(self.inner.clone())
    }

    /// Returns a copy of the inner avatar. Use `set_avatar` to modify the avatar.
    #[func]
    fn get_avatar(&self) -> Gd<DclAvatarWireFormat> {
        DclAvatarWireFormat::from_gd(self.inner.content.avatar.clone())
    }

    #[func]
    fn get_base_url(&self) -> GString {
        GString::from(self.inner.base_url.clone())
    }

    #[func]
    fn has_connected_web3(&self) -> bool {
        self.inner.content.has_connected_web3.unwrap_or_default()
    }

    #[func]
    fn get_name(&self) -> GString {
        GString::from(self.inner.content.name.clone())
    }

    #[func]
    fn has_claimed_name(&self) -> bool {
        self.inner.content.has_claimed_name.unwrap_or(false)
    }

    #[func]
    fn get_description(&self) -> GString {
        GString::from(self.inner.content.description.clone())
    }

    #[func]
    fn get_ethereum_address(&self) -> GString {
        GString::from(self.inner.content.eth_address.clone())
    }

    #[func]
    fn set_description(&mut self, description: GString) {
        self.inner.content.description = description.to_string();
    }

    #[func]
    fn set_name(&mut self, name: GString) {
        self.inner.content.name = name.to_string();
    }

    #[func]
    fn set_has_connected_web3(&mut self, has_connected_web3: bool) {
        self.inner.content.has_connected_web3 = Some(has_connected_web3);
    }

    #[func]
    fn set_has_claimed_name(&mut self, has_claimed_name: bool) {
        self.inner.content.has_claimed_name = Some(has_claimed_name);
    }

    #[func]
    fn set_avatar(&mut self, avatar: Gd<DclAvatarWireFormat>) {
        self.inner.content.avatar = avatar.bind().inner.clone();
    }

    #[func]
    pub fn increment_profile_version(&mut self) {
        self.inner.content.version += 1;
        self.inner.version = self.inner.content.version as u32;
    }

    #[func]
    fn get_profile_version(&self) -> u32 {
        self.inner.content.version as u32
    }

    #[func]
    fn set_profile_version(&mut self, version: u32) {
        self.inner.content.version = version as i64;
        self.inner.version = version;
    }

    #[func]
    pub fn from_godot_dictionary(dictionary: Dictionary) -> Gd<DclUserProfile> {
        let value = godot::classes::Json::stringify(dictionary.to_variant());
        DclUserProfile::from_gd(json5::from_str(value.to_string().as_str()).unwrap_or_default())
    }

    #[func]
    pub fn to_godot_dictionary(&self) -> Dictionary {
        let value = serde_json::to_string(&self.inner).unwrap_or_default();
        let value = godot::classes::Json::parse_string(value.into());
        value.to::<Dictionary>()
    }

    #[func]
    pub fn get_blocked(&self) -> Array<GString> {
        let mut arr = Array::new();
        if let Some(blocked) = &self.inner.content.blocked {
            for addr in blocked {
                arr.push(GString::from(addr.as_str()));
            }
        }
        arr
    }

    #[func]
    pub fn get_muted(&self) -> Array<GString> {
        let mut arr = Array::new();
        if let Some(muted) = &self.inner.content.muted {
            for addr in muted {
                arr.push(GString::from(addr.as_str()));
            }
        }
        arr
    }

    #[func]
    pub fn set_blocked(&mut self, blocked_list: Array<GString>) {
        let blocked_set: std::collections::HashSet<String> =
            blocked_list.iter_shared().map(|s| s.to_string()).collect();
        self.inner.content.blocked = if blocked_set.is_empty() {
            None
        } else {
            Some(blocked_set)
        };
    }

    #[func]
    pub fn set_muted(&mut self, muted_list: Array<GString>) {
        let muted_set: std::collections::HashSet<String> =
            muted_list.iter_shared().map(|s| s.to_string()).collect();
        self.inner.content.muted = if muted_set.is_empty() {
            None
        } else {
            Some(muted_set)
        };
    }

    // Nuevos campos del perfil
    #[func]
    fn get_country(&self) -> GString {
        GString::from(self.inner.content.country.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_country(&mut self, country: GString) {
        let country_str = country.to_string();
        self.inner.content.country = if country_str.is_empty() {
            None
        } else {
            Some(country_str)
        };
    }

    #[func]
    fn get_gender(&self) -> GString {
        GString::from(self.inner.content.gender.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_gender(&mut self, gender: GString) {
        let gender_str = gender.to_string();
        self.inner.content.gender = if gender_str.is_empty() {
            None
        } else {
            Some(gender_str)
        };
    }

    #[func]
    fn get_pronouns(&self) -> GString {
        GString::from(self.inner.content.pronouns.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_pronouns(&mut self, pronouns: GString) {
        let pronouns_str = pronouns.to_string();
        self.inner.content.pronouns = if pronouns_str.is_empty() {
            None
        } else {
            Some(pronouns_str)
        };
    }

    #[func]
    fn get_relationship_status(&self) -> GString {
        GString::from(
            self.inner
                .content
                .relation_ship_status
                .as_deref()
                .unwrap_or(""),
        )
    }

    #[func]
    fn set_relationship_status(&mut self, relationship_status: GString) {
        let status_str = relationship_status.to_string();
        self.inner.content.relation_ship_status = if status_str.is_empty() {
            None
        } else {
            Some(status_str)
        };
    }

    #[func]
    fn get_sexual_orientation(&self) -> GString {
        GString::from(
            self.inner
                .content
                .sexual_orientation
                .as_deref()
                .unwrap_or(""),
        )
    }

    #[func]
    fn set_sexual_orientation(&mut self, sexual_orientation: GString) {
        let orientation_str = sexual_orientation.to_string();
        self.inner.content.sexual_orientation = if orientation_str.is_empty() {
            None
        } else {
            Some(orientation_str)
        };
    }

    #[func]
    fn get_language(&self) -> GString {
        GString::from(self.inner.content.language.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_language(&mut self, language: GString) {
        let language_str = language.to_string();
        self.inner.content.language = if language_str.is_empty() {
            None
        } else {
            Some(language_str)
        };
    }

    #[func]
    fn get_employment_status(&self) -> GString {
        GString::from(
            self.inner
                .content
                .employment_status
                .as_deref()
                .unwrap_or(""),
        )
    }

    #[func]
    fn set_employment_status(&mut self, employment_status: GString) {
        let status_str = employment_status.to_string();
        self.inner.content.employment_status = if status_str.is_empty() {
            None
        } else {
            Some(status_str)
        };
    }

    #[func]
    fn get_profession(&self) -> GString {
        GString::from(self.inner.content.profession.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_profession(&mut self, profession: GString) {
        let profession_str = profession.to_string();
        self.inner.content.profession = if profession_str.is_empty() {
            None
        } else {
            Some(profession_str)
        };
    }

    #[func]
    fn get_real_name(&self) -> GString {
        GString::from(self.inner.content.real_name.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_real_name(&mut self, real_name: GString) {
        let real_name_str = real_name.to_string();
        self.inner.content.real_name = if real_name_str.is_empty() {
            None
        } else {
            Some(real_name_str)
        };
    }

    #[func]
    fn get_hobbies(&self) -> GString {
        GString::from(self.inner.content.hobbies.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_hobbies(&mut self, hobbies: GString) {
        let hobbies_str = hobbies.to_string();
        self.inner.content.hobbies = if hobbies_str.is_empty() {
            None
        } else {
            Some(hobbies_str)
        };
    }

    #[func]
    fn get_birthdate(&self) -> i64 {
        self.inner.content.birthdate.unwrap_or(0)
    }

    #[func]
    fn set_birthdate(&mut self, birthdate: i64) {
        self.inner.content.birthdate = if birthdate == 0 {
            None
        } else {
            Some(birthdate)
        };
    }

    #[func]
    fn get_interests(&self) -> Array<GString> {
        let mut arr = Array::new();
        if let Some(interests) = &self.inner.content.interests {
            for interest in interests {
                arr.push(GString::from(interest.as_str()));
            }
        }
        arr
    }

    #[func]
    fn set_interests(&mut self, interests_list: Array<GString>) {
        let interests_vec: Vec<String> = interests_list
            .iter_shared()
            .map(|s| s.to_string())
            .collect();
        self.inner.content.interests = if interests_vec.is_empty() {
            None
        } else {
            Some(interests_vec)
        };
    }

    #[func]
    fn get_email(&self) -> GString {
        GString::from(self.inner.content.email.as_deref().unwrap_or(""))
    }

    #[func]
    fn set_email(&mut self, email: GString) {
        let email_str = email.to_string();
        self.inner.content.email = if email_str.is_empty() {
            None
        } else {
            Some(email_str)
        };
    }

    #[func]
    fn get_tutorial_step(&self) -> u32 {
        self.inner.content.tutorial_step
    }

    #[func]
    fn set_tutorial_step(&mut self, tutorial_step: u32) {
        self.inner.content.tutorial_step = tutorial_step;
    }

    #[func]
    fn get_user_id(&self) -> GString {
        GString::from(self.inner.content.get_user_id())
    }

    #[func]
    fn set_user_id(&mut self, user_id: GString) {
        let user_id_str = user_id.to_string();
        self.inner.content.user_id = if user_id_str.is_empty() {
            None
        } else {
            Some(user_id_str)
        };
    }

    #[func]
    fn get_links(&self) -> Array<Dictionary> {
        let mut arr = Array::new();
        if let Some(links) = &self.inner.content.links {
            for link in links {
                let mut dict = Dictionary::new();
                dict.set("title", link.title.clone());
                dict.set("url", link.url.clone());
                arr.push(dict);
            }
        }
        arr
    }

    #[func]
    fn set_links(&mut self, links_array: Array<Dictionary>) {
        let links_vec: Vec<ProfileLink> = links_array
            .iter_shared()
            .filter_map(|dict| {
                let title = dict.get("title")?.to::<GString>().to_string();
                let url = dict.get("url")?.to::<GString>().to_string();
                Some(ProfileLink { title, url })
            })
            .collect();

        self.inner.content.links = if links_vec.is_empty() {
            None
        } else {
            Some(links_vec)
        };
    }
}
