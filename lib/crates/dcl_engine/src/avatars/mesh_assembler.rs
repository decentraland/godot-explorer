//! Assembles wearable meshes onto an avatar's base skeleton: merges the extra (spring) bones,
//! recycles their slots across outfit changes, rebinds mesh skins by name and swaps the glTF
//! materials for the shared toon shaders.

use std::cell::RefCell;
use std::collections::HashMap;

use godot::classes::base_material_3d::{CullMode, Feature, TextureParam, Transparency};
use godot::classes::{BaseMaterial3D, MeshInstance3D, Shader, ShaderMaterial, Skeleton3D, Skin};
use godot::prelude::*;

const TOON: &str = "res://assets/avatar/dcl_toon.gdshader";
const TOON_ALPHA_CLIP: &str = "res://assets/avatar/dcl_toon_alpha_clip.gdshader";
const TOON_ALPHA_BLEND: &str = "res://assets/avatar/dcl_toon_alpha_blend.gdshader";
const TOON_DOUBLE: &str = "res://assets/avatar/dcl_toon_double.gdshader";
const TOON_ALPHA_CLIP_DOUBLE: &str = "res://assets/avatar/dcl_toon_alpha_clip_double.gdshader";
const TOON_ALPHA_BLEND_DOUBLE: &str = "res://assets/avatar/dcl_toon_alpha_blend_double.gdshader";

thread_local! {
    /// Toon materials keyed by the source material's instance id, so avatars wearing the same
    /// wearable share one material. Skin and hair surfaces clone on write when they are tinted.
    static TOON_CACHE: RefCell<HashMap<i64, Gd<ShaderMaterial>>> = RefCell::new(HashMap::new());
}

/// `Avatar_Hips_2` -> `Avatar_Hips`: a Blender-style duplicate-import suffix (issue #1945).
fn strip_duplicate_suffix(bone_name: &str) -> Option<&str> {
    let (stem, tail) = bone_name.rsplit_once('_')?;
    (!tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit())).then_some(stem)
}

pub struct MeshAssembler {
    skeleton: Gd<Skeleton3D>,
    /// Bones this assembler added to the base skeleton and the active wearables use.
    active_extra_bones: Vec<i32>,
    /// Recycled slots for the next merge: `Skeleton3D` has no `remove_bone`, so reusing them is
    /// the only way to keep the bone count bounded across outfit and body-shape changes.
    free_bones: Vec<i32>,
    stale_counter: u32,
}

impl MeshAssembler {
    pub fn new(skeleton: Gd<Skeleton3D>) -> Self {
        Self {
            skeleton,
            active_extra_bones: Vec::new(),
            free_bones: Vec::new(),
            stale_counter: 0,
        }
    }

    /// Disables the bones merged by the previous outfit and returns their slots to the pool.
    pub fn recycle_extra_bones(&mut self) {
        let count = self.skeleton.get_bone_count();
        for bone in std::mem::take(&mut self.active_extra_bones) {
            if bone < 0 || bone >= count {
                continue;
            }
            let name = format!("__stale_bone_{}", self.stale_counter);
            self.stale_counter += 1;
            self.skeleton.set_bone_name(bone, name.as_str());
            self.skeleton
                .set_bone_enabled_ex(bone)
                .enabled(false)
                .done();
            self.skeleton.set_bone_parent(bone, -1);
            self.skeleton.set_bone_rest(bone, Transform3D::IDENTITY);
            self.skeleton.reset_bone_pose(bone);
            self.free_bones.push(bone);
        }
    }

    /// The base-skeleton name of a wearable bone: the de-suffixed name when that one exists,
    /// otherwise the original so genuine extra bones are still merged as new bones.
    pub fn resolve_to_base_bone_name(&self, bone_name: &str) -> String {
        if self.skeleton.find_bone(bone_name) != -1 {
            return bone_name.to_string();
        }
        match strip_duplicate_suffix(bone_name) {
            Some(stem) if self.skeleton.find_bone(stem) != -1 => stem.to_string(),
            _ => bone_name.to_string(),
        }
    }

    /// Copies the bones the wearable has and the base lacks (hair, earrings, capes), parents
    /// before children so the parent lookup by name always succeeds.
    pub fn merge_extra_bones(&mut self, wearable: &Gd<Skeleton3D>) {
        let count = wearable.get_bone_count();
        if count == 0 {
            return;
        }
        let mut missing: Vec<(i32, i32, String)> = Vec::new();
        for i in 0..count {
            let name = wearable.get_bone_name(i).to_string();
            if self.resolve_to_base_bone_name(&name) != name || self.skeleton.find_bone(&name) != -1
            {
                continue;
            }
            let mut depth = 0;
            let mut cursor = wearable.get_bone_parent(i);
            while cursor != -1 {
                depth += 1;
                cursor = wearable.get_bone_parent(cursor);
            }
            missing.push((depth, i, name));
        }
        missing.sort_by_key(|(depth, _, _)| *depth);

        for (_, wearable_idx, name) in missing {
            let new_idx = match self.free_bones.pop() {
                Some(slot) => {
                    self.skeleton.set_bone_name(slot, name.as_str());
                    self.skeleton.set_bone_enabled_ex(slot).enabled(true).done();
                    slot
                }
                None => {
                    self.skeleton.add_bone(name.as_str());
                    self.skeleton.get_bone_count() - 1
                }
            };
            self.skeleton
                .set_bone_rest(new_idx, wearable.get_bone_rest(wearable_idx));
            self.skeleton.reset_bone_pose(new_idx);
            self.active_extra_bones.push(new_idx);

            // A recycled slot may still point at its previous parent.
            let parent = wearable.get_bone_parent(wearable_idx);
            let mut parent_base = -1;
            if parent != -1 {
                let parent_name = wearable.get_bone_name(parent).to_string();
                parent_base = self
                    .skeleton
                    .find_bone(self.resolve_to_base_bone_name(&parent_name).as_str());
                if parent_base == -1 {
                    godot_warn!(
                        "[AVATAR] Extra bone '{name}' parent '{parent_name}' not found in base skeleton; leaving as root"
                    );
                }
            }
            self.skeleton.set_bone_parent(new_idx, parent_base);
        }
    }

    /// Makes every bind of the mesh's skin name its bone, so it resolves against the base
    /// skeleton (extended with the extra bones) once the mesh is reparented there.
    pub fn rebind_skin_by_name(&self, mesh: &mut Gd<MeshInstance3D>, wearable: &Gd<Skeleton3D>) {
        let Some(skin) = mesh.get_skin() else {
            return;
        };
        let mut skin: Gd<Skin> = skin.duplicate_resource();
        let count = wearable.get_bone_count();
        for i in 0..skin.get_bind_count() {
            let bone = skin.get_bind_bone(i);
            if bone >= 0 && bone < count {
                let name = wearable.get_bone_name(bone).to_string();
                skin.set_bind_name(i, self.resolve_to_base_bone_name(&name).as_str());
            }
        }
        mesh.set_skin(&skin);
    }
}

/// Swaps the mesh's `BaseMaterial3D` surfaces for the shared toon material of the same kind.
pub fn apply_toon_material(node: &Gd<Node>) {
    let Ok(mut mesh_instance) = node.clone().try_cast::<MeshInstance3D>() else {
        return;
    };
    let Some(mesh) = mesh_instance.get_mesh() else {
        return;
    };
    for surface in 0..mesh.get_surface_count() {
        let Some(material) = mesh.surface_get_material(surface) else {
            continue;
        };
        let Ok(base) = material.try_cast::<BaseMaterial3D>() else {
            continue;
        };
        let key = base.instance_id().to_i64();
        let cached = TOON_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(found) = cache.get(&key).filter(|m| m.is_instance_valid()) {
                return found.clone();
            }
            let toon = convert_to_toon(&base);
            cache.insert(key, toon.clone());
            toon
        });
        mesh_instance.set_surface_override_material(surface, &cached);
    }
}

pub fn apply_toon_material_recursive(node: &Gd<Node>) {
    apply_toon_material(node);
    for child in node.get_children().iter_shared() {
        apply_toon_material_recursive(&child);
    }
}

fn convert_to_toon(base: &Gd<BaseMaterial3D>) -> Gd<ShaderMaterial> {
    let transparency = base.get_transparency();
    let scissor = transparency == Transparency::ALPHA_SCISSOR;
    let blend = matches!(
        transparency,
        Transparency::ALPHA | Transparency::ALPHA_HASH | Transparency::ALPHA_DEPTH_PRE_PASS
    );
    let double_sided = base.get_cull_mode() == CullMode::DISABLED;
    let path = match (scissor, blend, double_sided) {
        (true, _, true) => TOON_ALPHA_CLIP_DOUBLE,
        (true, _, false) => TOON_ALPHA_CLIP,
        (false, true, true) => TOON_ALPHA_BLEND_DOUBLE,
        (false, true, false) => TOON_ALPHA_BLEND,
        (false, false, true) => TOON_DOUBLE,
        (false, false, false) => TOON,
    };
    let mut toon = ShaderMaterial::new_gd();
    toon.set_shader(&load::<Shader>(path));
    toon.set_shader_parameter("albedo_color", &base.get_albedo().to_variant());
    if let Some(texture) = base.get_texture(TextureParam::ALBEDO) {
        toon.set_shader_parameter("albedo_texture", &texture.to_variant());
    }
    if base.get_feature(Feature::EMISSION) {
        toon.set_shader_parameter("emission_color", &base.get_emission().to_variant());
        if let Some(texture) = base.get_texture(TextureParam::EMISSION) {
            toon.set_shader_parameter("emission_texture", &texture.to_variant());
        }
    }
    if scissor {
        toon.set_shader_parameter(
            "alpha_scissor_threshold",
            &base.get_alpha_scissor_threshold().to_variant(),
        );
    }
    toon
}

#[cfg(test)]
mod tests {
    use super::strip_duplicate_suffix;

    #[test]
    fn strips_only_a_trailing_numeric_suffix() {
        assert_eq!(strip_duplicate_suffix("Avatar_Hips_2"), Some("Avatar_Hips"));
        assert_eq!(
            strip_duplicate_suffix("Avatar_Hips_001"),
            Some("Avatar_Hips")
        );
        assert_eq!(strip_duplicate_suffix("Avatar_Hips"), None);
        assert_eq!(strip_duplicate_suffix("Avatar_Hips_L"), None);
        assert_eq!(strip_duplicate_suffix("Spring_1_2"), Some("Spring_1"));
        assert_eq!(strip_duplicate_suffix("hair_"), None);
    }
}
