//! Local profile snapshots (ADR-290): rendered here for immediate display and stored in the local
//! content folder, never uploaded. The profile-images service makes them for everyone else.

use godot::classes::{INode, Image, PackedScene};
use godot::prelude::*;

use crate::avatars::avatar::build::update_from_profile;
use crate::avatars::avatar_type::DclAvatarWireFormat;
use crate::avatars::dcl_user_profile::DclUserProfile;
use crate::avatars::preview::AvatarPreview;
use crate::engine::local_task::{await_promise, spawn_local};
use crate::fetcher::globals;
use crate::godot_classes::dcl_hashing::hash_v1;
use crate::godot_classes::promise::Promise;

const AVATAR_PREVIEW_SCENE: &str = "res://src/ui/pages/backpack/avatar_preview.tscn";
const FACE_SIZE: Vector2i = Vector2i::new(256, 256);
const FACE_ORTHO_SIZE: f32 = 0.55;

/// A Node (child of the world root), not a RefCounted: the off-screen preview it renders is its own
/// child, so the preview cannot outlive it and the engine drops this task with the node.
#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct Snapshot {
    base: Base<Node>,
}

#[godot_api]
impl INode for Snapshot {}

#[godot_api]
impl Snapshot {
    #[signal]
    fn snapshot_generated(face_image: Gd<Image>);

    /// Renders the face of `profile`, stores it and records its hash on `avatar_wire_format`.
    /// The promise resolves when that is done.
    #[func]
    fn async_generate_for_avatar(
        &mut self,
        avatar_wire_format: Gd<DclAvatarWireFormat>,
        profile: Gd<DclUserProfile>,
    ) -> Gd<Promise> {
        let promise = Promise::new_alloc();
        let handle = promise.clone();
        let this = self.to_gd();
        spawn_local(async move {
            generate(this, avatar_wire_format, profile).await;
            if handle.is_instance_valid() {
                handle.clone().bind_mut().resolve();
            }
        });
        promise
    }
}

async fn generate(
    mut this: Gd<Snapshot>,
    mut wire: Gd<DclAvatarWireFormat>,
    profile: Gd<DclUserProfile>,
) {
    let Some(mut preview) = godot::tools::load::<PackedScene>(AVATAR_PREVIEW_SCENE)
        .instantiate()
        .and_then(|n| n.try_cast::<AvatarPreview>().ok())
    else {
        godot_error!("Snapshot: {AVATAR_PREVIEW_SCENE} is not an AvatarPreview");
        return;
    };
    {
        let mut p = preview.bind_mut();
        p.set_show_platform(false);
        p.set_hide_name(true);
        p.set_can_move(false);
    }
    // Parented here and placed off-screen.
    this.add_child(&preview);
    if let Some(viewport) = this.get_viewport() {
        preview.set_position(viewport.get_visible_rect().size);
    }
    let Some(avatar) = preview.bind().get_avatar() else {
        preview.queue_free();
        return;
    };
    update_from_profile(avatar, profile).await;
    if !this.is_instance_valid() {
        return;
    }
    let capture = preview
        .bind_mut()
        .async_get_viewport_image(true, FACE_SIZE, FACE_ORTHO_SIZE, 1);
    let result = await_promise(capture).await;
    let Ok(face) = result.try_to::<Gd<Image>>() else {
        godot_error!("Snapshot: the face capture failed");
        preview.queue_free();
        return;
    };
    let bytes = face.save_png_to_buffer();
    let hash = hash_v1(bytes.as_slice());
    let stored = globals::content_provider()
        .bind_mut()
        .store_file(hash.as_str().into(), bytes);
    await_promise(stored).await;
    if !this.is_instance_valid() {
        return;
    }
    wire.bind_mut()
        .set_snapshots(hash.as_str().into(), "".into());
    preview.queue_free();
    this.emit_signal("snapshot_generated", &[face.to_variant()]);
}
