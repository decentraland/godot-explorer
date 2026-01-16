//! Emote GLTF loading (for ContentProvider emote loading).

use godot::{
    builtin::{Quaternion, VarArray},
    classes::{animation::TrackType, Animation, AnimationLibrary, AnimationPlayer, Node, Node3D},
    meta::ToGodot,
    obj::{Gd, NewAlloc},
    prelude::*,
};

use super::super::{
    content_mapping::ContentMappingAndUrlRef,
    content_provider::SceneGltfContext,
    scene_saver::{get_emote_path_for_hash, save_node_as_scene},
};
use super::common::{clear_owner_recursive, count_nodes, load_gltf_pipeline, set_owner_recursive};

/// Strip Blender duplicate suffixes from bone names.
///
/// Blender adds `_001`, `_002`, etc. suffixes when objects have duplicate names.
/// These need to be stripped so animation tracks can target the actual skeleton bones.
///
/// Pattern: `_0XX` where XX are digits (e.g., `_001`, `_012`, `_099`)
/// This does NOT strip valid bone suffixes like `Index1`, `Thumb2` (no underscore before digit)
///
/// Examples:
/// - `Avatar_LeftLeg_001` -> `Avatar_LeftLeg`
/// - `Avatar_RightFoot_012` -> `Avatar_RightFoot`
/// - `Avatar_LeftHandIndex1` -> `Avatar_LeftHandIndex1` (unchanged - valid bone name)
fn strip_blender_suffix(name: &str) -> String {
    // Pattern: ends with underscore + 0 + two more digits
    // e.g., _001, _002, _012, _099
    if name.len() >= 4 {
        let bytes = name.as_bytes();
        let len = bytes.len();

        // Check if ends with _0XX pattern
        if bytes[len - 4] == b'_'
            && bytes[len - 3] == b'0'
            && bytes[len - 2].is_ascii_digit()
            && bytes[len - 1].is_ascii_digit()
        {
            return name[..len - 4].to_string();
        }
    }
    name.to_string()
}

/// Get the last 16 alphanumeric characters from a hash (used for animation naming).
pub fn get_last_16_alphanumeric(input: &str) -> String {
    let alphanumeric: String = input
        .chars()
        .rev()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(16)
        .collect();

    alphanumeric
        .chars()
        .rev()
        .collect::<String>()
        .to_lowercase()
}

/// Process emote animations and return components for embedding or DclEmoteGltf creation.
///
/// Returns (armature_prop, default_animation, prop_animation)
/// This is used by load_and_save_emote_gltf to extract and embed animations in the background thread.
#[allow(clippy::type_complexity)]
pub fn process_emote_animations(
    file_hash: &str,
    gltf_node: &Gd<Node3D>,
) -> Option<(
    Option<Gd<Node3D>>,
    Option<Gd<Animation>>,
    Option<Gd<Animation>>,
)> {
    let anim_sufix_from_hash = get_last_16_alphanumeric(file_hash);
    let armature_prop_node = gltf_node.find_child("Armature_Prop");

    let anim_player = gltf_node.try_get_node_as::<AnimationPlayer>("AnimationPlayer")?;

    let armature_prefix = format!("Armature_Prop_{}/Skeleton3D:", anim_sufix_from_hash);

    let armature_prop = armature_prop_node
        .and_then(|v| v.clone().try_cast::<Node3D>().ok())
        .map(|mut node| {
            node.set_name(&format!("Armature_Prop_{}", anim_sufix_from_hash));
            node.rotate_y(std::f32::consts::PI);
            node
        });

    let is_single_animation = anim_player.get_animation_list().len() == 1;

    let anim_list: Vec<String> = anim_player
        .get_animation_list()
        .as_slice()
        .iter()
        .map(|v| v.to_string())
        .collect();

    let mut default_animation: Option<Gd<Animation>> = None;
    let mut prop_animation: Option<Gd<Animation>> = None;
    let mut default_anim_key = None;
    let mut prop_anim_key = None;

    tracing::debug!(
        "Emote '{}': Found {} animations: {:?}, is_single_animation={}",
        file_hash,
        anim_list.len(),
        anim_list,
        is_single_animation
    );

    for animation_key in anim_list.iter() {
        // Strip Blender suffixes before checking for _avatar or _prop endings
        let key_lower = animation_key.to_lowercase();
        let key_stripped = strip_blender_suffix(&key_lower);

        if is_single_animation || key_stripped.ends_with("_avatar") {
            default_anim_key = Some(animation_key.clone());
        } else if key_stripped.ends_with("_prop")
            || key_stripped.ends_with("action")
            || key_lower.contains("prop")
        {
            // Match prop animations: ending with _prop, or "action" (common Blender naming),
            // or containing "prop" anywhere in the name
            prop_anim_key = Some(animation_key.clone());
        }
    }

    // Corner case, the glb doesn't follow the docs instructions
    if !is_single_animation {
        for animation_key in anim_list.iter() {
            if default_anim_key.is_none() {
                default_anim_key = Some(animation_key.clone());
            } else if prop_anim_key.is_none() {
                prop_anim_key = Some(animation_key.clone());
            }
        }
    }

    tracing::debug!(
        "Emote '{}': Animation assignments - default_anim_key={:?}, prop_anim_key={:?}",
        file_hash,
        default_anim_key,
        prop_anim_key
    );

    let mut play_emote_audio_args = VarArray::new();
    play_emote_audio_args.push(&file_hash.to_variant());
    let play_emote_audio_call = VarDictionary::from_iter([
        ("method", "_play_emote_audio".to_variant()),
        ("args", play_emote_audio_args.to_variant()),
    ]);

    let mut audio_added = false;

    for animation_key in anim_list.iter() {
        let Some(mut anim) = anim_player.get_animation(animation_key) else {
            continue;
        };

        tracing::debug!(
            "Processing emote animation: hash='{}', key='{}', track_count={}, is_default={}, is_prop={}",
            file_hash,
            animation_key,
            anim.get_track_count(),
            default_anim_key.as_ref() == Some(animation_key),
            prop_anim_key.as_ref() == Some(animation_key)
        );

        // Log all tracks before processing
        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();
            tracing::debug!("  [BEFORE] Track[{}]: '{}'", track_idx, track_path);
        }

        if default_anim_key.as_ref() == Some(animation_key) {
            default_animation = Some(anim.clone());
            anim.set_name(&anim_sufix_from_hash.to_godot())
        } else if prop_anim_key.as_ref() == Some(animation_key) {
            prop_animation = Some(anim.clone());
            anim.set_name(&format!("{anim_sufix_from_hash}_prop"))
        }

        // First pass: identify tracks that reference orphan Blender duplicate bones
        // These tracks have animation data authored for a wrong bone hierarchy
        // and will cause incorrect animations if applied to the real skeleton.
        //
        // We identify two types of problematic tracks:
        // 1. Direct orphan tracks: "Armature/Avatar_LeftLeg_001" (direct child with suffix)
        // 2. Descendants of orphans: "Armature/Avatar_LeftLeg_001/Avatar_LeftFoot"
        //    (these have animation data in the wrong coordinate space)
        let mut orphan_tracks: Vec<i32> = Vec::new();

        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();

            // Skip tracks that already have Skeleton3D or are prop tracks
            if track_path.contains("Skeleton3D") || track_path.contains("Armature_Prop") {
                continue;
            }

            let parts: Vec<&str> = track_path.split('/').collect();

            // Check if ANY part of the path (except the last bone) has a Blender suffix
            // This means the track is either an orphan or a child of an orphan
            let mut has_orphan_ancestor = false;
            for (i, part) in parts.iter().enumerate() {
                // Skip "Armature" prefix and the last part (which is the target bone)
                if i == 0 || i == parts.len() - 1 {
                    continue;
                }
                // Check if this intermediate path part has a Blender suffix
                let stripped = strip_blender_suffix(part);
                if stripped != *part {
                    has_orphan_ancestor = true;
                    break;
                }
            }

            // Also check for direct orphan: "Armature/BoneName_0XX" (2 parts)
            if parts.len() == 2 && parts[0] == "Armature" {
                let bone_name = parts[1];
                let stripped = strip_blender_suffix(bone_name);
                if stripped != bone_name {
                    has_orphan_ancestor = true;
                }
            }

            if has_orphan_ancestor {
                tracing::debug!(
                    "Removing orphan hierarchy track [{}]: '{}' (has Blender duplicate ancestor)",
                    track_idx,
                    track_path
                );
                orphan_tracks.push(track_idx);
            }
        }

        // Remove orphan tracks (in reverse order to maintain indices)
        orphan_tracks.sort();
        orphan_tracks.reverse();
        for track_idx in &orphan_tracks {
            tracing::debug!("Removing orphan track {}", track_idx);
            anim.remove_track(*track_idx);
        }

        // Second pass: remap remaining tracks
        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();

            // Handle root motion "Armature" tracks specially
            // These animate the entire Armature node and need coordinate system transformation
            // because the avatar's Skeleton3D has a 180° Y rotation built into its transform
            if track_path == "Armature" {
                let track_type = anim.track_get_type(track_idx);
                match track_type {
                    TrackType::POSITION_3D => {
                        // Don't transform position - let it pass through as-is
                        // The sway motion should work without position changes
                        tracing::debug!(
                            "  Keeping position track 'Armature' unchanged ({} keys)",
                            anim.track_get_key_count(track_idx)
                        );
                    }
                    TrackType::ROTATION_3D => {
                        // Transform rotation values: invert X only
                        // This compensates for the 180° Y rotation in the skeleton
                        let key_count = anim.track_get_key_count(track_idx);
                        for key_idx in 0..key_count {
                            let value = anim.track_get_key_value(track_idx, key_idx);
                            if let Ok(rot) = value.try_to::<Quaternion>() {
                                // Invert X and Z (fixes direction), then apply +90° X rotation (fixes forward tilt)
                                // The avatar's Skeleton3D has a 180° Y rotation, requiring this compensation
                                let sin_45 = std::f32::consts::FRAC_PI_4.sin(); // sin(45°) = cos(45°) ≈ 0.7071
                                let x_pos90 = Quaternion::new(sin_45, 0.0, 0.0, sin_45);
                                let inverted = Quaternion::new(-rot.x, rot.y, -rot.z, rot.w);
                                let transformed = x_pos90 * inverted;
                                anim.track_set_key_value(
                                    track_idx,
                                    key_idx,
                                    &transformed.to_variant(),
                                );
                            }
                        }
                        tracing::debug!(
                            "  Transformed {} rotation keys for root motion track 'Armature' (inverted X)",
                            key_count
                        );
                    }
                    _ => {
                        tracing::debug!(
                            "  Keeping root motion track 'Armature' unchanged (type: {:?})",
                            track_type
                        );
                    }
                }
                continue;
            }

            if !track_path.contains("Skeleton3D") {
                let last_track_name = track_path.split('/').next_back().unwrap_or_default();

                // Strip Blender duplicate suffixes from the BONE NAME only
                let bone_name = strip_blender_suffix(last_track_name);

                // Check if this is a prop track (Armature_Prop/...) or avatar track (Armature/...)
                if track_path.starts_with("Armature_Prop") {
                    // Check if it's a root prop track (just "Armature_Prop")
                    if track_path == "Armature_Prop" {
                        // Rename to Armature_Prop_{hash} for root motion
                        let new_track_path = format!("Armature_Prop_{}", anim_sufix_from_hash);
                        tracing::debug!(
                            "  Remapping prop root motion '{}' -> '{}'",
                            track_path,
                            new_track_path
                        );
                        anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
                    } else {
                        // Prop bone track: remap to Armature_Prop_{hash}/Skeleton3D:{bone}
                        let new_track_path = format!("{}{}", armature_prefix, bone_name);
                        tracing::debug!(
                            "  Remapping prop track '{}' -> '{}'",
                            track_path,
                            new_track_path
                        );
                        anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
                    }
                } else {
                    // Avatar track: remap to Armature/Skeleton3D:{bone}
                    let new_track_path = format!("Armature/Skeleton3D:{}", bone_name);
                    anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
                }
            } else if track_path.contains("Armature_Prop/Skeleton3D")
                || track_path.contains("Armature_Prop:")
            {
                // Already has Skeleton3D, just rename the Armature_Prop to include hash
                let track_subname = track_path.split(':').next_back().unwrap_or_default();
                let new_track_path = format!("{}{}", armature_prefix, track_subname);
                tracing::debug!(
                    "  Remapping Skeleton3D prop track '{}' -> '{}'",
                    track_path,
                    new_track_path
                );
                anim.track_set_path(track_idx, &NodePath::from(&new_track_path));
            }
        }

        if armature_prop.is_some() {
            let new_track_prop = anim.add_track(TrackType::VALUE);
            anim.track_set_path(
                new_track_prop,
                &format!("Armature_Prop_{}:visible", anim_sufix_from_hash),
            );
            anim.track_insert_key(new_track_prop, 0.0, &true.to_variant());
        }

        if !audio_added {
            audio_added = true;
            let new_track_audio = anim.add_track(TrackType::METHOD);
            anim.track_set_path(new_track_audio, ".");
            anim.track_insert_key(
                new_track_audio,
                0.0,
                &play_emote_audio_call.clone().to_variant(),
            );
        }

        // Log all tracks after processing
        tracing::debug!(
            "  [AFTER] Animation '{}' now has {} tracks",
            animation_key,
            anim.get_track_count()
        );
        for track_idx in 0..anim.get_track_count() {
            let track_path = anim.track_get_path(track_idx).to_string();
            tracing::debug!("  [AFTER] Track[{}]: '{}'", track_idx, track_path);
        }
    }

    Some((armature_prop, default_animation, prop_animation))
}

/// Build DclEmoteGltf from processed components.
/// Used when loading from cache to create the DclEmoteGltf from stored data.
pub fn build_dcl_emote_gltf(
    armature_prop: Option<Gd<Node3D>>,
    default_animation: Option<Gd<Animation>>,
    prop_animation: Option<Gd<Animation>>,
) -> Gd<DclEmoteGltf> {
    Gd::from_init_fn(|_base| DclEmoteGltf {
        armature_prop,
        default_animation,
        prop_animation,
    })
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct DclEmoteGltf {
    #[var]
    armature_prop: Option<Gd<Node3D>>,
    #[var]
    default_animation: Option<Gd<Animation>>,
    #[var]
    prop_animation: Option<Gd<Animation>>,
}

/// Load and save an emote GLTF to disk.
///
/// This function:
/// 1. Downloads the GLTF and its dependencies
/// 2. Loads it into Godot
/// 3. Processes textures
/// 4. Saves the processed scene to disk as plain Node3D
///    (Animation extraction happens when loading from cache via load_cached_emote)
///
/// Returns the path to the saved scene file on success.
pub async fn load_and_save_emote_gltf(
    file_path: String,
    file_hash: String,
    content_mapping: ContentMappingAndUrlRef,
    ctx: SceneGltfContext,
) -> Result<String, anyhow::Error> {
    let ctx_clone = ctx.clone();

    let (scene_path, file_size) = load_gltf_pipeline(
        file_path,
        file_hash.clone(),
        content_mapping,
        ctx,
        |node, hash, ctx| {
            // Extract animations in background thread
            let (armature_prop, default_animation, prop_animation) =
                process_emote_animations(hash, &node)
                    .ok_or(anyhow::Error::msg("Failed to extract emote animations"))?;

            // Create EmoteRoot node to save with embedded animations
            let mut root = Node3D::new_alloc();
            root.set_name(&StringName::from("EmoteRoot"));

            // Add armature_prop as child if present
            if let Some(mut prop) = armature_prop {
                // Remove from original parent if any
                if let Some(mut parent) = prop.get_parent() {
                    parent.remove_child(&prop.clone().upcast::<Node>());
                }
                // Clear owner to avoid "inconsistent owner" warning
                prop.set_owner(Gd::<Node>::null_arg());
                clear_owner_recursive(&mut prop.clone().upcast::<Node>());
                root.add_child(&prop.clone().upcast::<Node>());
                set_owner_recursive(
                    &mut prop.clone().upcast::<Node>(),
                    &root.clone().upcast::<Node>(),
                );
            }

            // Create AnimationPlayer with processed animations
            let mut anim_player = AnimationPlayer::new_alloc();
            anim_player.set_name(&StringName::from("EmoteAnimations"));

            let mut anim_library = AnimationLibrary::new_gd();
            if let Some(ref anim) = default_animation {
                anim_library.add_animation(&StringName::from(&anim.get_name()), anim);
            }
            if let Some(ref anim) = prop_animation {
                anim_library.add_animation(&StringName::from(&anim.get_name()), anim);
            }
            anim_player.add_animation_library(&StringName::from(""), &anim_library);

            root.add_child(&anim_player.clone().upcast::<Node>());
            anim_player.set_owner(&root.clone().upcast::<Node>());

            // Save the EmoteRoot scene to disk
            let scene_path = get_emote_path_for_hash(&ctx.content_folder, hash);
            save_node_as_scene(root.clone(), &scene_path).map_err(anyhow::Error::msg)?;

            // Get file size synchronously
            let file_size = std::fs::metadata(&scene_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            // Count nodes before freeing
            let node_count = count_nodes(root.clone().upcast());
            tracing::debug!(
                "Emote GLTF processed with embedded animations: {} with {} nodes, saved to {} ({} bytes)",
                hash,
                node_count,
                scene_path,
                file_size
            );

            // Free both nodes since we've saved to disk
            node.free();
            root.free();

            Ok((scene_path, file_size))
        },
    )
    .await?;

    // Register the saved scene in resource_provider for cache management
    ctx_clone
        .resource_provider
        .register_local_file(&scene_path, file_size)
        .await;

    Ok(scene_path)
}
