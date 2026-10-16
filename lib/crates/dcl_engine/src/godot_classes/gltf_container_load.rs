//! The behaviors of a GltfContainer: requesting its load from the shared coordinator, realizing
//! the instance, fixing materials, building colliders and following the entity's motion.

use godot::classes::base_material_3d::{Flags, TextureParam};
use godot::classes::node::ProcessMode;
use godot::classes::physics_server_3d::BodyMode;
use godot::classes::{
    BaseMaterial3D, CollisionShape3D, ConcavePolygonShape3D, Mesh, MeshInstance3D, PackedScene,
    PhysicsServer3D, StaticBody3D,
};
use godot::prelude::*;

use super::dcl_gltf_container::{DclGltfContainer, GltfContainerLoadingState};
use crate::fetcher::globals;
use crate::scene_runner::gltf_loading;

/// The collision mask of a collider: invisible meshes (marked while processing the glTF) and
/// visible ones have their own.
pub fn collider_mask(invisible_mesh: bool, visible_cmask: i32, invisible_cmask: i32) -> i32 {
    if invisible_mesh {
        invisible_cmask
    } else {
        visible_cmask
    }
}

/// Whether a spawned collider is tracked for a later switch to KINEMATIC: only active ones, and
/// only while the entity is not already known to move.
pub fn tracks_static_mode(mask: i32, kinematic_requested: bool) -> bool {
    mask != 0 && !kinematic_requested
}

fn set_kinematic(body: &Gd<StaticBody3D>) {
    let rid = body.get_rid();
    if rid.is_valid() {
        PhysicsServer3D::singleton().body_set_mode(rid, BodyMode::KINEMATIC);
    }
}

impl DclGltfContainer {
    /// Requests the load at the end of the frame, once the entity is in the tree.
    pub(crate) fn defer_load(&mut self) {
        self.to_gd()
            .run_deferred(|container| container.async_load_gltf());
    }

    /// Resolves the content hash, decides optimized or runtime once, and hands over to the
    /// coordinator, which shares the download and the load across every container of the hash.
    pub fn async_load_gltf(&mut self) {
        // `dcl_gltf_src` stays as the scene sent it: `change_gltf` compares it case-sensitively
        // with the src Rust hands back, so lowercasing it makes every later mutation (say a
        // collision-mask change during AvatarAttach) read as a src change and reload. The
        // content mapping lowercases for its lookups itself.
        let mapping = globals::scene_runner()
            .bind()
            .get_scene_content_mapping(self.dcl_scene_id);
        let hash = mapping.bind().get_hash(self.dcl_gltf_src.clone());
        self.dcl_gltf_hash = hash.clone();

        if hash.is_empty() {
            self.dcl_gltf_loading_state = GltfContainerLoadingState::NotFound;
            return;
        }
        self.dcl_gltf_loading_state = GltfContainerLoadingState::Loading;
        gltf_loading::schedule_timeout(&self.to_gd(), gltf_loading::LOAD_TIMEOUT_MS);

        let has_optimized = globals::content_provider()
            .bind()
            .optimized_asset_exists(hash.clone());
        let (only_no_optimized, only_optimized) = {
            let cli = globals::cli();
            let cli = cli.bind();
            (cli.only_no_optimized, cli.only_optimized)
        };
        let use_optimized = if only_no_optimized {
            // Always runtime-process, ignoring optimized assets.
            false
        } else if only_optimized {
            // Skip entirely when no optimized asset exists.
            if !has_optimized {
                self.dcl_gltf_loading_state = GltfContainerLoadingState::NotFound;
                gltf_loading::cancel_timeout(&self.to_gd());
                return;
            }
            true
        } else {
            // Prefer the pre-baked optimized asset when present.
            has_optimized
        };

        self.optimized = use_optimized;
        self.requested_hash = hash.to_string();
        let fresh = std::mem::take(&mut self.force_fresh_load);
        gltf_loading::request(
            &self.to_gd(),
            self.dcl_scene_id,
            &hash.to_string(),
            &self.dcl_gltf_src.to_string(),
            use_optimized,
            mapping,
            fresh,
        );
    }

    /// Whether the instance still has to be built: a load in flight and nothing added yet. Guards
    /// a group from being drained twice for late waiters.
    pub fn needs_realize(&self) -> bool {
        self.dcl_gltf_loading_state == GltfContainerLoadingState::Loading
            && self.base().get_child_count() == 0
    }

    /// Instantiates a fresh copy of the shared PackedScene and adds it. Called for every waiter
    /// of one source within a single frame: instantiating clones the node tree but references
    /// the already loaded, already GPU-resident meshes and textures, so it is CPU-only and cheap.
    pub fn instantiate_and_add(&mut self, scene: &Gd<PackedScene>) {
        // The scene may have been unloaded between the shared load and this realize pass.
        if !self.base().is_inside_tree() {
            self.finish_with_error("scene unloaded during load");
            return;
        }
        let Some(gltf) = scene
            .instantiate()
            .and_then(|n| n.try_cast::<Node3D>().ok())
        else {
            self.finish_with_error("the loaded scene has no 3D root");
            return;
        };
        self.apply_fixes(&gltf);
        // Colliders are created with mask 0 initially.
        let (visible, invisible, scene_id, entity_id) = (
            self.dcl_visible_cmask,
            self.dcl_invisible_cmask,
            self.dcl_scene_id,
            self.dcl_entity_id,
        );
        self.set_mask_colliders(
            &gltf.clone().upcast(),
            visible,
            invisible,
            scene_id,
            entity_id,
        );
        // Added to the tree so every MeshInstance3D inside has a valid global transform: the
        // manager bakes per-mesh local poses against the container's current world.
        self.base_mut().add_child(&gltf);
    }

    /// Marks FINISHED after the batch's render frame.
    pub fn complete_shared_load(&mut self) {
        if self.dcl_gltf_loading_state != GltfContainerLoadingState::Loading {
            return;
        }
        self.dcl_gltf_loading_state = GltfContainerLoadingState::Finished;
        gltf_loading::cancel_timeout(&self.to_gd());
        self.check_animations();
    }

    /// The shared load failed for this hash.
    pub fn on_shared_load_error(&mut self, reason: &str) {
        self.finish_with_error(reason);
    }

    pub fn on_load_timeout(&mut self) {
        self.finish_with_error("timeout");
    }

    fn finish_with_error(&mut self, reason: &str) {
        godot_error!("GLTF load error for {}: {reason}", self.dcl_gltf_src);
        if !self.dcl_gltf_hash.is_empty() {
            globals::content_provider()
                .bind()
                .report_resource_failed(self.dcl_gltf_hash.clone(), reason.into());
        }
        self.dcl_gltf_loading_state = GltfContainerLoadingState::FinishedWithError;
        gltf_loading::cancel_timeout(&self.to_gd());
    }

    /// Whether the container's scene is the one the player stands in.
    pub fn is_current_scene(&self) -> bool {
        self.dcl_scene_id == globals::scene_runner().bind().get_current_parcel_scene_id()
    }

    // ---- materials ----

    fn apply_fixes(&self, gltf: &Gd<Node3D>) {
        let mut pending: Vec<Gd<Node>> = gltf.get_children().iter_shared().collect();
        while let Some(node) = pending.pop() {
            pending.extend(node.get_children().iter_shared());
            let Ok(mesh_instance) = node.try_cast::<MeshInstance3D>() else {
                continue;
            };
            let Some(mesh) = mesh_instance.get_mesh() else {
                continue;
            };
            for surface in 0..mesh.get_surface_count() {
                if let Some(material) = mesh
                    .surface_get_material(surface)
                    .and_then(|m| m.try_cast::<BaseMaterial3D>().ok())
                {
                    fix_material(material);
                }
            }
        }
    }

    // ---- colliders ----

    fn static_body_of(mesh_instance: &Gd<MeshInstance3D>) -> Option<Gd<StaticBody3D>> {
        mesh_instance
            .get_children()
            .iter_shared()
            .find_map(|child| child.try_cast::<StaticBody3D>().ok())
    }

    /// Visible meshes are baked without a collider (see `scene.rs` `bake_lazy_collider_faces`):
    /// the faces live in the shared mesh's metadata, and the body and its concave shape are
    /// built here the first time an entity asks for a non-zero `visibleMeshesCollisionMask`. The
    /// shape is cached on the mesh, so every instance of the same glTF shares one BVH.
    fn ensure_lazy_collider(mesh_instance: &mut Gd<MeshInstance3D>) -> Option<Gd<StaticBody3D>> {
        let mut mesh: Gd<Mesh> = mesh_instance.get_mesh()?;
        // The first instance to need the shape builds it and consumes `dcl_faces`; every later
        // instance of the same mesh finds it under `dcl_shape`.
        let existing: Variant = mesh
            .get_meta_ex("dcl_shape")
            .default(&Variant::nil())
            .done();
        let shape = match existing.try_to::<Gd<ConcavePolygonShape3D>>() {
            Ok(shape) => shape,
            Err(_) => {
                if !mesh.has_meta("dcl_faces") {
                    return None;
                }
                let mut shape = ConcavePolygonShape3D::new_gd();
                if let Ok(faces) = mesh.get_meta("dcl_faces").try_to::<PackedVector3Array>() {
                    shape.set_faces(&faces);
                }
                let backface = mesh
                    .get_meta_ex("dcl_backface")
                    .default(&true.to_variant())
                    .done()
                    .booleanize();
                shape.set_backface_collision_enabled(backface);
                mesh.set_meta("dcl_shape", &shape.to_variant());
                // The shape owns its own copy of the faces (and the BVH): ours go.
                mesh.remove_meta("dcl_faces");
                shape
            }
        };
        let mut body = StaticBody3D::new_alloc();
        body.set_name(&format!("{}_colgen", mesh_instance.get_name()));
        body.set_meta("invisible_mesh", &false.to_variant());
        body.set_collision_layer(0);
        body.set_collision_mask(0);
        let mut collision_shape = CollisionShape3D::new_alloc();
        collision_shape.set_shape(&shape);
        body.add_child(&collision_shape);
        mesh_instance.add_child(&body);
        Some(body)
    }

    /// Sets collision masks and metadata on every collider after instantiating. A StaticBody3D
    /// is STATIC by default and switches to KINEMATIC if the entity moves. True when any
    /// collider has an active mask (needs kinematic tracking).
    pub fn set_mask_colliders(
        &self,
        root: &Gd<Node>,
        visible_cmask: i32,
        invisible_cmask: i32,
        scene_id: i32,
        entity_id: i32,
    ) -> bool {
        let mut has_active = false;
        for node in root.get_children().iter_shared() {
            if let Ok(mut mesh_instance) = node.clone().try_cast::<MeshInstance3D>() {
                let mut body = Self::static_body_of(&mesh_instance);
                if body.is_none() && visible_cmask != 0 {
                    body = Self::ensure_lazy_collider(&mut mesh_instance);
                }
                if let Some(mut body) = body {
                    let invisible = body.has_meta("invisible_mesh")
                        && body.get_meta("invisible_mesh").booleanize();
                    let mask = collider_mask(invisible, visible_cmask, invisible_cmask);
                    body.set_meta("dcl_col", &mask.to_variant());
                    body.set_meta("dcl_scene_id", &scene_id.to_variant());
                    body.set_meta("dcl_entity_id", &entity_id.to_variant());
                    body.set_collision_layer(mask as u32);
                    body.set_collision_mask(0);
                    if mask == 0 {
                        body.set_process_mode(ProcessMode::DISABLED);
                    } else {
                        body.set_process_mode(ProcessMode::INHERIT);
                        if self.kinematic_requested {
                            // The entity is known to move: KINEMATIC at once.
                            set_kinematic(&body);
                        } else {
                            // Tracked: switches to KINEMATIC if the entity moves.
                            body.set_meta("dcl_static_mode", &true.to_variant());
                        }
                        has_active = true;
                    }
                }
            }
            if self.set_mask_colliders(&node, visible_cmask, invisible_cmask, scene_id, entity_id) {
                has_active = true;
            }
        }
        has_active
    }

    fn update_mask_colliders(&self, root: &Gd<Node>) {
        for node in root.get_children().iter_shared() {
            if let Ok(mut mesh_instance) = node.clone().try_cast::<MeshInstance3D>() {
                let mut body = Self::static_body_of(&mesh_instance);
                if body.is_none() && self.dcl_visible_cmask != 0 {
                    body = Self::ensure_lazy_collider(&mut mesh_instance);
                }
                if let Some(mut body) = body {
                    let invisible = body.has_meta("invisible_mesh")
                        && body.get_meta("invisible_mesh").booleanize();
                    let mask =
                        collider_mask(invisible, self.dcl_visible_cmask, self.dcl_invisible_cmask);
                    body.set_collision_layer(mask as u32);
                    body.set_collision_mask(0);
                    body.set_meta("dcl_col", &mask.to_variant());
                    body.set_process_mode(if mask == 0 {
                        ProcessMode::DISABLED
                    } else {
                        ProcessMode::INHERIT
                    });
                }
            }
            self.update_mask_colliders(&node);
        }
    }

    /// Switches every tracked collider from STATIC to KINEMATIC; searches all descendants, not
    /// only the direct children of a MeshInstance3D.
    fn switch_colliders_to_kinematic(root: &Gd<Node>) {
        if !root.is_instance_valid() {
            return;
        }
        for node in root.get_children().iter_shared() {
            if !node.is_instance_valid() {
                continue;
            }
            if let Ok(mut body) = node.clone().try_cast::<StaticBody3D>() {
                if body.has_meta("dcl_static_mode") {
                    set_kinematic(&body);
                    body.remove_meta("dcl_static_mode");
                }
            }
            Self::switch_colliders_to_kinematic(&node);
        }
    }

    /// The entity moved enough times (the second transform change) to need kinematic colliders.
    pub fn switch_to_kinematic_mode(&mut self) {
        if !self.base().is_inside_tree() || self.base().is_queued_for_deletion() {
            return;
        }
        // Remembered, so future colliders spawn KINEMATIC.
        self.kinematic_requested = true;
        if let Some(gltf) = self.get_gltf_resource() {
            Self::switch_colliders_to_kinematic(&gltf.upcast());
        }
    }

    // ---- changes ----

    /// Applies a changed GltfContainer: a new src reloads everything, a new mask only the
    /// colliders.
    pub fn change_gltf(&mut self, new_src: &str, visible_cmask: i32, invisible_cmask: i32) {
        let gltf = self.get_gltf_resource();
        let src_changed = self.dcl_gltf_src != new_src;
        let masks_changed =
            visible_cmask != self.dcl_visible_cmask || invisible_cmask != self.dcl_invisible_cmask;
        self.dcl_visible_cmask = visible_cmask;
        self.dcl_invisible_cmask = invisible_cmask;

        if src_changed {
            self.dcl_gltf_src = new_src.into();
            self.optimized = false;
            self.tear_down_for_reload(gltf);
        } else if masks_changed {
            if let Some(gltf) = gltf {
                self.update_mask_colliders(&gltf.upcast());
            }
        }
    }

    /// Re-runs the load for the current src even though the string did not change: the preview
    /// hot-reload case, where the file changed under a src and hash that both stayed. The load
    /// re-resolves against the (already updated) content mapping, including the missing-entry
    /// case, which lands in NOT_FOUND, as a deleted file should.
    pub fn force_reload_gltf(&mut self) {
        let gltf = self.get_gltf_resource();
        self.optimized = false;
        self.force_fresh_load = true;
        self.tear_down_for_reload(gltf);
    }

    /// Detaches from the old group, frees the old instance, and marks LOADING at once: Rust's
    /// `sync_gltf_loading_state` runs later this same frame and would read a stale FINISHED,
    /// evict the entity from `scene.gltf_loading`, and the real finish would never re-apply
    /// GltfNodeModifiers (skins lost on a src change or a reparent-triggered reload).
    fn tear_down_for_reload(&mut self, gltf: Option<Gd<Node3D>>) {
        if !self.requested_hash.is_empty() {
            let hash = std::mem::take(&mut self.requested_hash);
            gltf_loading::unregister(&self.to_gd(), &hash);
        }
        if let Some(mut gltf) = gltf {
            self.base_mut().remove_child(&gltf);
            gltf.queue_free();
        }
        if let Some(mut pending) = self.dcl_pending_node.take() {
            pending.queue_free();
        }
        self.dcl_gltf_loading_state = GltfContainerLoadingState::Loading;
        self.defer_load();
    }
}

/// Applied once per material object: materials are shared by every instance of a PackedScene,
/// and re-running this per instantiate halved `metallic` again for each copy of the same glTF.
fn fix_material(mut material: Gd<BaseMaterial3D>) {
    if material.has_meta("dcl_fixed") {
        return;
    }
    material.set_meta("dcl_fixed", &true.to_variant());
    // A metallic texture means the metallic value is halved (the metallic-specular-roughness
    // rule).
    if material.get_texture(TextureParam::METALLIC).is_some() {
        let metallic = material.get_metallic();
        material.set_metallic(metallic * 0.5);
    }
    // To replicate Foundation.
    material.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, false);
}

#[godot_api(secondary)]
impl DclGltfContainer {
    /// Rust signals that the entity moved enough to need kinematic colliders.
    #[func]
    fn on_switch_to_kinematic(&mut self) {
        self.switch_to_kinematic_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invisible_meshes_use_their_own_mask() {
        assert_eq!(collider_mask(true, 1, 3), 3);
        assert_eq!(collider_mask(false, 1, 3), 1);
        assert_eq!(collider_mask(false, 0, 3), 0);
    }

    #[test]
    fn only_active_static_colliders_are_tracked() {
        assert!(tracks_static_mode(1, false));
        assert!(!tracks_static_mode(0, false));
        assert!(!tracks_static_mode(1, true));
    }
}
