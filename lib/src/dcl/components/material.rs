use crate::content::content_mapping::ContentMappingAndUrlRef;

use super::{
    proto_components::{
        common::{
            texture_union::Tex, Color3, Color4, TextureFilterMode, TextureUnion, TextureWrapMode,
            Vector2,
        },
        sdk::components::{pb_material, MaterialTransparencyMode},
    },
    SceneEntityId,
};

#[derive(Clone)]
pub struct RoundedFloat(pub f32);

#[derive(Clone)]
pub struct RoundedColor3(pub Color3);

#[derive(Clone)]
pub struct RoundedColor4(pub Color4);

impl From<RoundedFloat> for f32 {
    fn from(val: RoundedFloat) -> Self {
        val.0
    }
}

impl From<&f32> for RoundedFloat {
    fn from(value: &f32) -> Self {
        Self(*value)
    }
}

impl std::hash::Hash for RoundedFloat {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let value = (100000.0 * self.0) as i64;
        value.hash(state);
    }
}

impl PartialEq for RoundedFloat {
    fn eq(&self, other: &Self) -> bool {
        let value = (100000.0 * self.0) as i64;
        let other_value = (100000.0 * other.0) as i64;
        value == other_value
    }
}

impl Eq for RoundedFloat {
    fn assert_receiver_is_total_eq(&self) {}
}

impl std::hash::Hash for RoundedColor3 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        RoundedFloat(self.0.r).hash(state);
        RoundedFloat(self.0.g).hash(state);
        RoundedFloat(self.0.b).hash(state);
    }
}

impl PartialEq for RoundedColor3 {
    fn eq(&self, other: &Self) -> bool {
        RoundedFloat(self.0.r) == RoundedFloat(other.0.r)
            && RoundedFloat(self.0.g) == RoundedFloat(other.0.g)
            && RoundedFloat(self.0.b) == RoundedFloat(other.0.b)
    }
}

impl std::hash::Hash for RoundedColor4 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        RoundedFloat(self.0.a).hash(state);
        RoundedFloat(self.0.r).hash(state);
        RoundedFloat(self.0.g).hash(state);
        RoundedFloat(self.0.b).hash(state);
    }
}

impl PartialEq for RoundedColor4 {
    fn eq(&self, other: &Self) -> bool {
        RoundedFloat(self.0.r) == RoundedFloat(other.0.r)
            && RoundedFloat(self.0.g) == RoundedFloat(other.0.g)
            && RoundedFloat(self.0.b) == RoundedFloat(other.0.b)
            && RoundedFloat(self.0.a) == RoundedFloat(other.0.a)
    }
}

impl Eq for RoundedColor4 {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Eq for RoundedColor3 {
    fn assert_receiver_is_total_eq(&self) {}
}

#[derive(Clone)]
pub struct RoundedVector2(pub Vector2);

impl std::hash::Hash for RoundedVector2 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        RoundedFloat(self.0.x).hash(state);
        RoundedFloat(self.0.y).hash(state);
    }
}

impl PartialEq for RoundedVector2 {
    fn eq(&self, other: &Self) -> bool {
        RoundedFloat(self.0.x) == RoundedFloat(other.0.x)
            && RoundedFloat(self.0.y) == RoundedFloat(other.0.y)
    }
}

impl Eq for RoundedVector2 {
    fn assert_receiver_is_total_eq(&self) {}
}

impl Default for RoundedVector2 {
    fn default() -> Self {
        Self(Vector2 { x: 0.0, y: 0.0 })
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum DclSourceTex {
    Texture(String),
    AvatarTexture(String),
    VideoTexture(SceneEntityId),
}

impl Default for DclSourceTex {
    fn default() -> DclSourceTex {
        DclSourceTex::Texture("".to_string())
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct DclTexture {
    pub wrap_mode: TextureWrapMode,
    pub filter_mode: TextureFilterMode,
    pub source: DclSourceTex,
    /// UV offset, default = (0, 0). Only applies to main texture in PbrMaterial/UnlitMaterial.
    pub offset: RoundedVector2,
    /// UV tiling/scale, default = (1, 1). Only applies to main texture in PbrMaterial/UnlitMaterial.
    pub tiling: RoundedVector2,
}

impl Default for DclTexture {
    fn default() -> Self {
        Self {
            wrap_mode: TextureWrapMode::TwmClamp,
            filter_mode: TextureFilterMode::TfmBilinear,
            source: DclSourceTex::default(),
            offset: RoundedVector2(Vector2 { x: 0.0, y: 0.0 }),
            tiling: RoundedVector2(Vector2 { x: 1.0, y: 1.0 }),
        }
    }
}

impl From<&TextureUnion> for Option<DclTexture> {
    fn from(value: &TextureUnion) -> Option<DclTexture> {
        let texture = value.tex.as_ref()?;
        let wrap_mode = match texture {
            Tex::Texture(texture) => *texture
                .wrap_mode
                .as_ref()
                .unwrap_or(&TextureWrapMode::TwmClamp.into()),
            Tex::AvatarTexture(avatar_texture) => *avatar_texture
                .wrap_mode
                .as_ref()
                .unwrap_or(&TextureWrapMode::TwmClamp.into()),
            Tex::VideoTexture(video_texture) => *video_texture
                .wrap_mode
                .as_ref()
                .unwrap_or(&TextureWrapMode::TwmClamp.into()),
            Tex::UiTexture(_) => todo!("UI Texture not implemented"),
        };
        let filter_mode = match texture {
            Tex::Texture(texture) => *texture
                .filter_mode
                .as_ref()
                .unwrap_or(&TextureFilterMode::TfmBilinear.into()),
            Tex::AvatarTexture(avatar_texture) => *avatar_texture
                .filter_mode
                .as_ref()
                .unwrap_or(&TextureFilterMode::TfmBilinear.into()),
            Tex::VideoTexture(video_texture) => *video_texture
                .filter_mode
                .as_ref()
                .unwrap_or(&TextureFilterMode::TfmBilinear.into()),
            Tex::UiTexture(_) => todo!("UI Texture not implemented"),
        };

        let wrap_mode = TextureWrapMode::from_i32(wrap_mode).unwrap_or(TextureWrapMode::TwmClamp);
        let filter_mode =
            TextureFilterMode::from_i32(filter_mode).unwrap_or(TextureFilterMode::TfmBilinear);

        match value.tex.as_ref()? {
            Tex::Texture(texture) => {
                // Only Texture type has offset and tiling fields
                let offset = texture
                    .offset
                    .as_ref()
                    .map(|v| RoundedVector2(v.clone()))
                    .unwrap_or_else(|| RoundedVector2(Vector2 { x: 0.0, y: 0.0 }));
                let tiling = texture
                    .tiling
                    .as_ref()
                    .map(|v| RoundedVector2(v.clone()))
                    .unwrap_or_else(|| RoundedVector2(Vector2 { x: 1.0, y: 1.0 }));

                Some(DclTexture {
                    wrap_mode,
                    filter_mode,
                    source: DclSourceTex::Texture(texture.src.clone()),
                    offset,
                    tiling,
                })
            }
            Tex::AvatarTexture(avatar_texture) => Some(DclTexture {
                wrap_mode,
                filter_mode,
                source: DclSourceTex::AvatarTexture(avatar_texture.user_id.clone()),
                offset: RoundedVector2(Vector2 { x: 0.0, y: 0.0 }),
                tiling: RoundedVector2(Vector2 { x: 1.0, y: 1.0 }),
            }),
            Tex::VideoTexture(video_texture) => Some(DclTexture {
                wrap_mode,
                filter_mode,
                source: DclSourceTex::VideoTexture(SceneEntityId::from_i32(
                    video_texture.video_player_entity as i32,
                )),
                offset: RoundedVector2(Vector2 { x: 0.0, y: 0.0 }),
                tiling: RoundedVector2(Vector2 { x: 1.0, y: 1.0 }),
            }),
            Tex::UiTexture(_) => todo!("UI Texture not implemented"),
        }
    }
}

impl DclTexture {
    fn with_hash(&mut self, content_mapping_files: &ContentMappingAndUrlRef) {
        if let DclSourceTex::Texture(file_path) = &mut self.source {
            let content_hash = content_mapping_files.get_hash(file_path.as_str());

            if content_hash.is_none() {
                return;
            }
            if let Some(content_hash) = content_hash {
                *file_path = content_hash.to_string();
            }
        }
    }

    pub fn from_proto_with_hash(
        texture: &Option<TextureUnion>,
        content_mapping_files: &ContentMappingAndUrlRef,
    ) -> Option<Self> {
        let value: Option<DclTexture> = texture.as_ref()?.into();

        if let Some(mut value) = value {
            value.with_hash(content_mapping_files);
            Some(value)
        } else {
            None
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct DclUnlitMaterial {
    pub texture: Option<DclTexture>,
    pub alpha_test: RoundedFloat,
    pub cast_shadows: bool,
    pub diffuse_color: RoundedColor4,
    pub alpha_texture: Option<DclTexture>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct DclPbrMaterial {
    pub texture: Option<DclTexture>,
    pub alpha_test: RoundedFloat,
    pub cast_shadows: bool,
    pub alpha_texture: Option<DclTexture>,
    pub emissive_texture: Option<DclTexture>,
    pub bump_texture: Option<DclTexture>,
    pub albedo_color: RoundedColor4,
    pub emissive_color: RoundedColor3,
    pub reflectivity_color: RoundedColor3,
    pub transparency_mode: MaterialTransparencyMode,
    pub metallic: RoundedFloat,
    pub roughness: RoundedFloat,
    pub specular_intensity: RoundedFloat,
    pub emissive_intensity: RoundedFloat,
    pub direct_intensity: RoundedFloat,
}

#[derive(Clone, Hash, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum DclMaterial {
    Unlit(DclUnlitMaterial),
    Pbr(DclPbrMaterial),
}

impl DclMaterial {
    pub fn get_textures(&self) -> Vec<&Option<DclTexture>> {
        match self {
            DclMaterial::Unlit(unlit_material) => {
                vec![&unlit_material.texture, &unlit_material.alpha_texture]
            }
            DclMaterial::Pbr(pbr) => {
                vec![
                    &pbr.texture,
                    &pbr.alpha_texture,
                    &pbr.emissive_texture,
                    &pbr.bump_texture,
                ]
            }
        }
    }
}

// Default from .proto comments
impl Default for DclPbrMaterial {
    fn default() -> Self {
        Self {
            texture: None,
            alpha_test: RoundedFloat(0.5),
            cast_shadows: true,
            alpha_texture: None,
            emissive_texture: None,
            bump_texture: None,
            albedo_color: RoundedColor4(Color4::white()),
            emissive_color: RoundedColor3(Color3::black()),
            reflectivity_color: RoundedColor3(Color3::white()),
            transparency_mode: MaterialTransparencyMode::MtmAuto,
            metallic: RoundedFloat(0.5),
            roughness: RoundedFloat(0.5),
            specular_intensity: RoundedFloat(1.0),
            emissive_intensity: RoundedFloat(2.0),
            direct_intensity: RoundedFloat(1.0),
        }
    }
}

// Default from .proto comments
impl Default for DclUnlitMaterial {
    fn default() -> Self {
        Self {
            texture: None,
            alpha_test: RoundedFloat(0.5),
            cast_shadows: true,
            diffuse_color: RoundedColor4(Color4::white()),
            alpha_texture: None,
        }
    }
}

impl DclMaterial {
    pub fn from_proto(
        source: &pb_material::Material,
        content_mapping_files: &ContentMappingAndUrlRef,
    ) -> Self {
        match source {
            pb_material::Material::Pbr(pbr) => {
                let mut value: DclPbrMaterial = DclPbrMaterial {
                    // Fill defined values from pbr
                    texture: DclTexture::from_proto_with_hash(&pbr.texture, content_mapping_files),
                    alpha_texture: DclTexture::from_proto_with_hash(
                        &pbr.alpha_texture,
                        content_mapping_files,
                    ),
                    bump_texture: DclTexture::from_proto_with_hash(
                        &pbr.bump_texture,
                        content_mapping_files,
                    ),
                    emissive_texture: DclTexture::from_proto_with_hash(
                        &pbr.emissive_texture,
                        content_mapping_files,
                    ),
                    ..Default::default()
                };

                if pbr.alpha_test.is_some() {
                    value.alpha_test = pbr.alpha_test.as_ref().unwrap().into();
                }
                if pbr.cast_shadows.is_some() {
                    value.cast_shadows = *pbr.cast_shadows.as_ref().unwrap();
                }
                if pbr.albedo_color.is_some() {
                    value.albedo_color = RoundedColor4(pbr.albedo_color.as_ref().unwrap().clone());
                }
                if pbr.emissive_color.is_some() {
                    value.emissive_color =
                        RoundedColor3(pbr.emissive_color.as_ref().unwrap().clone());
                }
                if pbr.reflectivity_color.is_some() {
                    value.reflectivity_color =
                        RoundedColor3(pbr.reflectivity_color.as_ref().unwrap().clone());
                }
                if pbr.transparency_mode.is_some() {
                    value.transparency_mode = MaterialTransparencyMode::from_i32(
                        *pbr.transparency_mode.as_ref().unwrap(),
                    )
                    .unwrap_or_default();
                }
                if pbr.metallic.is_some() {
                    value.metallic = pbr.metallic.as_ref().unwrap().into();
                }
                if pbr.roughness.is_some() {
                    value.roughness = pbr.roughness.as_ref().unwrap().into();
                }
                if pbr.specular_intensity.is_some() {
                    value.specular_intensity = pbr.specular_intensity.as_ref().unwrap().into();
                }
                if pbr.emissive_intensity.is_some() {
                    value.emissive_intensity = pbr.emissive_intensity.as_ref().unwrap().into();
                }
                if pbr.direct_intensity.is_some() {
                    value.direct_intensity = pbr.direct_intensity.as_ref().unwrap().into();
                }

                DclMaterial::Pbr(value)
            }
            pb_material::Material::Unlit(unlit) => {
                let mut value = DclUnlitMaterial {
                    texture: DclTexture::from_proto_with_hash(
                        &unlit.texture,
                        content_mapping_files,
                    ),
                    alpha_texture: DclTexture::from_proto_with_hash(
                        &unlit.alpha_texture,
                        content_mapping_files,
                    ),
                    ..Default::default()
                };

                if unlit.alpha_test.is_some() {
                    value.alpha_test = unlit.alpha_test.as_ref().unwrap().into();
                }
                if unlit.cast_shadows.is_some() {
                    value.cast_shadows = *unlit.cast_shadows.as_ref().unwrap();
                }
                if unlit.diffuse_color.is_some() {
                    value.diffuse_color =
                        RoundedColor4(unlit.diffuse_color.as_ref().unwrap().clone());
                }

                DclMaterial::Unlit(value)
            }
        }
    }
}
