use godot::{
    bind::{godot_api, GodotClass},
    builtin::{meta::ToGodot, Dictionary, GString},
    obj::Gd,
};

use crate::comms::profile::UserProfile;

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
    pub fn get_avatar(&self) -> Gd<DclAvatarWireFormat> {
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
    fn set_has_claimed_nlame(&mut self, has_claimed_name: bool) {
        self.inner.content.has_claimed_name = Some(has_claimed_name);
    }

    #[func]
    fn set_avatar(&mut self, avatar: Gd<DclAvatarWireFormat>) {
        self.inner.content.avatar = avatar.bind().inner.clone();
    }

    #[func]
    pub fn from_godot_dictionary(dictionary: Dictionary) -> Gd<DclUserProfile> {
        let value = godot::engine::Json::stringify(dictionary.to_variant());
        DclUserProfile::from_gd(
            serde_json::from_str(value.to_string().as_str()).unwrap_or_default(),
        )
    }

    #[func]
    pub fn to_godot_dictionary(&self) -> Dictionary {
        let value = serde_json::to_string(&self.inner).unwrap_or_default();
        let value = godot::engine::Json::parse_string(value.into());
        value.to::<Dictionary>()
    }
}
