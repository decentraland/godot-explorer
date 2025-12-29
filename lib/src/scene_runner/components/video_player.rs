use crate::{
    av::backend::BackendType,
    dcl::{
        components::{
            proto_components::sdk::components::{PbVideoEvent, VideoState},
            SceneComponentId,
        },
        crdt::{
            grow_only_set::GenericGrowOnlySetComponentOperation,
            last_write_wins::LastWriteWinsComponentOperation, SceneCrdtState,
            SceneCrdtStateProtoComponents,
        },
        SceneId,
    },
    godot_classes::dcl_video_player::{
        DclVideoPlayer, VIDEO_STATE_BUFFERING, VIDEO_STATE_ERROR, VIDEO_STATE_LOADING,
        VIDEO_STATE_PAUSED, VIDEO_STATE_PLAYING, VIDEO_STATE_READY, VIDEO_STATE_SEEKING,
    },
    scene_runner::{
        godot_dcl_scene::VideoPlayerData,
        scene::{Scene, SceneType},
    },
};

/// Position change threshold in seconds - emit event if position changes by more than this
const POSITION_CHANGE_THRESHOLD: f64 = 0.5;
use godot::{
    classes::{image::Format, Image, ImageTexture, Node},
    prelude::*,
};

/// Determines what kind of update is needed for a video player entity
enum VideoUpdateMode {
    /// Only update playback parameters (volume, playing, looping)
    OnlyChangeValues,
    /// Video source changed - need to reinitialize backend
    ChangeVideo,
    /// First time creating video player for this entity
    FirstSpawnVideo,
}

/// Main update function for video player components.
/// This function handles CRDT component changes and delegates backend management to GDScript.
pub fn update_video_player(
    scene: &mut Scene,
    crdt_state: &mut SceneCrdtState,
    current_parcel_scene_id: &SceneId,
) {
    let godot_dcl_scene = &mut scene.godot_dcl_scene;
    let dirty_lww_components = &scene.current_dirty.lww_components;
    let video_player_component = SceneCrdtStateProtoComponents::get_video_player(crdt_state);

    // Track which entities need livekit registration (done after main loop to avoid borrow issues)
    let mut livekit_registrations = Vec::new();

    if let Some(video_player_dirty) = dirty_lww_components.get(&SceneComponentId::VIDEO_PLAYER) {
        tracing::debug!(
            "Video player component has {} dirty entities in scene {}",
            video_player_dirty.len(),
            scene.scene_id.0
        );

        for entity in video_player_dirty {
            let exist_current_node = godot_dcl_scene.get_godot_entity_node(entity).is_some();

            let next_value = video_player_component
                .get(entity)
                .and_then(|v| v.value.as_ref());

            if let Some(next_value) = next_value {
                let target_src = next_value.src.clone();

                tracing::debug!(
                    "Video player update for entity {}: src={}, volume={:?}, playing={:?}, loop={:?}",
                    entity,
                    target_src,
                    next_value.volume,
                    next_value.playing,
                    next_value.r#loop
                );

                // Determine if this scene should be muted (non-current parcel scenes are muted)
                let muted_by_current_scene = if let SceneType::Parcel = scene.scene_type {
                    scene.scene_id != *current_parcel_scene_id
                } else {
                    true
                };

                let dcl_volume = next_value.volume.unwrap_or(1.0).clamp(0.0, 1.0);
                let playing = next_value.playing.unwrap_or(true);
                let looping = next_value.r#loop.unwrap_or(false);
                let position = next_value.position;
                let playback_rate = next_value.playback_rate.map(|r| r.clamp(0.1, 10.0));

                let (godot_entity_node, mut node_3d) = godot_dcl_scene.ensure_node_3d(entity);

                // Determine update mode based on current state
                let update_mode =
                    if let Some(video_player_data) = godot_entity_node.video_player_data.as_ref() {
                        if target_src != video_player_data.source {
                            VideoUpdateMode::ChangeVideo
                        } else {
                            VideoUpdateMode::OnlyChangeValues
                        }
                    } else {
                        VideoUpdateMode::FirstSpawnVideo
                    };

                match update_mode {
                    VideoUpdateMode::OnlyChangeValues => {
                        // Just update playback parameters on existing player
                        if let Some(mut video_player_node) = get_video_player_node(&node_3d) {
                            if let Some(video_player_data) =
                                godot_entity_node.video_player_data.as_mut()
                            {
                                update_video_player_params(
                                    &mut video_player_node,
                                    video_player_data,
                                    dcl_volume,
                                    muted_by_current_scene,
                                    playing,
                                    looping,
                                    position,
                                    playback_rate,
                                );
                            }
                        }
                    }

                    VideoUpdateMode::ChangeVideo => {
                        // Preserve the timestamp from the old VideoPlayerData for monotonic counter
                        let old_timestamp = godot_entity_node
                            .video_player_data
                            .as_ref()
                            .map(|d| d.timestamp)
                            .unwrap_or(0);

                        tracing::debug!(
                            "Video player changing video for entity {}: {} -> {}",
                            entity,
                            godot_entity_node
                                .video_player_data
                                .as_ref()
                                .map(|d| d.source.as_str())
                                .unwrap_or("none"),
                            target_src
                        );

                        // Dispose existing backend
                        if let Some(mut video_player_node) = get_video_player_node(&node_3d) {
                            video_player_node.bind_mut().backend_dispose();
                        }

                        // Reinitialize with new source
                        let backend_type = BackendType::from_source(&target_src);
                        let video_player_node =
                            get_or_create_video_player_node(&mut node_3d, scene.scene_id.0);

                        initialize_video_player(
                            video_player_node.clone(),
                            backend_type,
                            &target_src,
                            dcl_volume,
                            muted_by_current_scene,
                            playing,
                            looping,
                        );

                        // Update tracking data - preserve timestamp for monotonic counter
                        let mut video_player_data = if backend_type == BackendType::LiveKit {
                            let texture = video_player_node
                                .bind()
                                .get_dcl_texture()
                                .expect("LiveKit video player should have texture");
                            VideoPlayerData::new_with_texture(
                                target_src.clone(),
                                backend_type,
                                texture,
                            )
                        } else {
                            VideoPlayerData::new(target_src.clone(), backend_type)
                        };
                        video_player_data.timestamp = old_timestamp;

                        godot_entity_node.video_player_data = Some(video_player_data);
                        scene.video_players.insert(*entity, video_player_node);

                        if backend_type == BackendType::LiveKit {
                            livekit_registrations.push(*entity);
                        }
                    }

                    VideoUpdateMode::FirstSpawnVideo => {
                        tracing::debug!(
                            "Video player activated (first spawn) for entity {}: {}",
                            entity,
                            target_src
                        );

                        let backend_type = BackendType::from_source(&target_src);
                        let video_player_node =
                            get_or_create_video_player_node(&mut node_3d, scene.scene_id.0);

                        initialize_video_player(
                            video_player_node.clone(),
                            backend_type,
                            &target_src,
                            dcl_volume,
                            muted_by_current_scene,
                            playing,
                            looping,
                        );

                        // Set up tracking data - for LiveKit, store the texture reference
                        let video_player_data = if backend_type == BackendType::LiveKit {
                            let texture = video_player_node
                                .bind()
                                .get_dcl_texture()
                                .expect("LiveKit video player should have texture");
                            VideoPlayerData::new_with_texture(
                                target_src.clone(),
                                backend_type,
                                texture,
                            )
                        } else {
                            VideoPlayerData::new(target_src.clone(), backend_type)
                        };

                        godot_entity_node.video_player_data = Some(video_player_data);
                        scene.video_players.insert(*entity, video_player_node);

                        if backend_type == BackendType::LiveKit {
                            livekit_registrations.push(*entity);
                        }
                    }
                }
            } else if exist_current_node {
                // Component removed - dispose the video player
                let Some(node) = godot_dcl_scene.get_godot_entity_node_mut(entity) else {
                    continue;
                };

                if let Some(video_player_data) = node.video_player_data.as_ref() {
                    tracing::debug!(
                        "Video player deactivated for entity {}: {}",
                        entity,
                        video_player_data.source
                    );
                }

                // Dispose backend through GDScript
                if let Some(base_3d) = &node.base_3d {
                    if let Some(mut video_player_node) = get_video_player_node(base_3d) {
                        video_player_node.bind_mut().backend_dispose();
                    }
                }

                node.video_player_data = None;
                scene.video_players.remove(entity);
            }
        }
    }

    // Process video events from all video players
    poll_video_events(scene, crdt_state);

    // Register livekit video players after the main loop to avoid borrow conflicts
    for entity in livekit_registrations {
        scene.register_livekit_video_player(entity);
    }
}

/// Get an existing VideoPlayer node from a parent node
fn get_video_player_node(parent: &Gd<Node3D>) -> Option<Gd<DclVideoPlayer>> {
    parent
        .get_node_or_null("VideoPlayer")
        .and_then(|n| n.try_cast::<DclVideoPlayer>().ok())
}

/// Get or create a VideoPlayer node
fn get_or_create_video_player_node(parent: &mut Gd<Node3D>, scene_id: i32) -> Gd<DclVideoPlayer> {
    if let Some(existing) = get_video_player_node(parent) {
        return existing;
    }

    // Create new video player node from scene
    let mut video_player_node =
        godot::tools::load::<PackedScene>("res://src/decentraland_components/video_player.tscn")
            .instantiate()
            .expect("Failed to instantiate video_player.tscn")
            .cast::<DclVideoPlayer>();

    video_player_node.bind_mut().set_dcl_scene_id(scene_id);
    video_player_node.set_name("VideoPlayer");

    // Create initial placeholder texture for LiveKit (will be updated by video frames)
    let image = Image::create(8, 8, false, Format::RGBA8).expect("couldn't create video image");
    let texture = ImageTexture::create_from_image(&image).expect("couldn't create video texture");
    video_player_node.bind_mut().set_dcl_texture(Some(texture));

    parent.add_child(&video_player_node.clone().upcast::<Node>());

    video_player_node
}

/// Initialize a video player with the appropriate backend
fn initialize_video_player(
    mut video_player_node: Gd<DclVideoPlayer>,
    backend_type: BackendType,
    source: &str,
    volume: f32,
    muted: bool,
    playing: bool,
    looping: bool,
) {
    // Set volume and mute state (actual volume application handled by GDScript _process)
    video_player_node.bind_mut().set_volume(volume);
    video_player_node.bind_mut().set_muted(muted);

    // Initialize the backend (this calls into GDScript)
    video_player_node.bind_mut().init_backend(
        backend_type.to_gd_int(),
        source.into(),
        playing,
        looping,
    );
}

/// Update playback parameters on an existing video player
#[allow(clippy::too_many_arguments)]
fn update_video_player_params(
    video_player_node: &mut Gd<DclVideoPlayer>,
    video_player_data: &mut VideoPlayerData,
    volume: f32,
    muted: bool,
    playing: bool,
    looping: bool,
    position: Option<f32>,
    playback_rate: Option<f32>,
) {
    // Set volume and mute state (actual volume application handled by GDScript _process)
    video_player_node.bind_mut().set_volume(volume);
    video_player_node.bind_mut().set_muted(muted);

    if playing {
        video_player_node.bind_mut().backend_play();
    } else {
        video_player_node.bind_mut().backend_pause();
    }

    video_player_node.bind_mut().backend_set_looping(looping);

    // Handle seek when position is explicitly set and changed
    // None means "don't change", only Some(value) triggers seek
    if let Some(requested_position) = position {
        // Only seek if position has changed significantly from last requested position
        let position_diff = (requested_position - video_player_data.last_requested_position).abs();
        if position_diff > 0.1 {
            tracing::debug!(
                "Video player seeking to {:.2}s (was {:.2}s)",
                requested_position,
                video_player_data.last_requested_position
            );
            video_player_node
                .bind_mut()
                .backend_seek(requested_position);
            video_player_data.last_requested_position = requested_position;
        }
    }

    // Handle playback rate changes
    // None means "don't change", only Some(value) triggers rate change
    if let Some(rate) = playback_rate {
        let rate_diff = (rate - video_player_data.last_playback_rate).abs();
        if rate_diff > 0.01 {
            tracing::debug!(
                "Video player setting playback rate to {:.2} (was {:.2})",
                rate,
                video_player_data.last_playback_rate
            );
            video_player_node.bind_mut().backend_set_playback_rate(rate);
            video_player_data.last_playback_rate = rate;
        }
    }
}

/// Poll video state from all video players and generate CRDT events on state changes
fn poll_video_events(scene: &mut Scene, crdt_state: &mut SceneCrdtState) {
    let video_event_component = SceneCrdtStateProtoComponents::get_video_event_mut(crdt_state);

    if !scene.video_players.is_empty() {
        tracing::trace!(
            "Polling {} video players in scene {}",
            scene.video_players.len(),
            scene.scene_id.0
        );
    }

    for (entity_id, video_player_node) in scene.video_players.iter() {
        let Some(godot_entity) = scene.godot_dcl_scene.get_godot_entity_node_mut(entity_id) else {
            tracing::warn!("Video player entity {} has no godot entity node", entity_id);
            continue;
        };
        let Some(video_player_data) = godot_entity.video_player_data.as_mut() else {
            tracing::warn!("Video player entity {} has no video_player_data", entity_id);
            continue;
        };

        // Read current state from the video player node
        let player = video_player_node.bind();
        let current_state = player.get_video_state();
        let current_position = player.get_video_position();
        let current_length = player.get_video_length();
        drop(player);

        // Check if any relevant property has changed
        let state_changed = current_state != video_player_data.last_state;
        let length_changed =
            current_length > 0.0 && (current_length - video_player_data.last_length).abs() > 0.001;
        let position_changed =
            (current_position - video_player_data.last_position).abs() > POSITION_CHANGE_THRESHOLD;

        if state_changed || length_changed || position_changed {
            // Map internal state to SDK VideoState
            let sdk_state = match current_state {
                VIDEO_STATE_LOADING => VideoState::VsLoading,
                VIDEO_STATE_READY => VideoState::VsReady,
                VIDEO_STATE_PLAYING => VideoState::VsPlaying,
                VIDEO_STATE_BUFFERING => VideoState::VsBuffering,
                VIDEO_STATE_SEEKING => VideoState::VsSeeking,
                VIDEO_STATE_PAUSED => VideoState::VsPaused,
                VIDEO_STATE_ERROR => VideoState::VsError,
                _ => VideoState::VsNone,
            };

            tracing::info!(
                "Video event for entity {}: state={:?}, position={:.2}, length={:.2} (state_changed={}, length_changed={}, position_changed={})",
                entity_id,
                sdk_state,
                current_position,
                current_length,
                state_changed,
                length_changed,
                position_changed
            );

            // Emit the video event
            video_event_component.append(
                *entity_id,
                PbVideoEvent {
                    timestamp: video_player_data.timestamp,
                    tick_number: scene.tick_number,
                    current_offset: current_position as f32,
                    video_length: current_length as f32,
                    state: sdk_state as i32,
                },
            );

            // Update last known state
            video_player_data.last_state = current_state;
            video_player_data.last_position = current_position;
            video_player_data.last_length = current_length;
            video_player_data.timestamp += 1;
        }
    }
}

/// Update video texture from LiveKit video frame data.
/// This is called from the scene when receiving video frames from LiveKit.
pub fn update_video_texture_from_livekit(
    video_player_data: &mut VideoPlayerData,
    width: u32,
    height: u32,
    data: &[u8],
) {
    use crate::content::packed_array::PackedByteArrayFromVec;
    use godot::classes::image::Format;
    use godot::classes::Image;
    use godot::prelude::PackedByteArray;

    let Some(texture) = &mut video_player_data.texture else {
        tracing::warn!("update_video_texture_from_livekit called but no texture available");
        return;
    };

    let data_arr = PackedByteArray::from_vec(data);

    // Check if resize needed
    let current_size = texture.get_size();
    if current_size.x != width as f32 || current_size.y != height as f32 {
        // Create new image with new dimensions
        let image =
            Image::create_from_data(width as i32, height as i32, false, Format::RGBA8, &data_arr)
                .unwrap();
        texture.set_image(&image);
        texture.update(&image);
    } else {
        // Update existing texture in-place
        let mut image = texture.get_image().unwrap();
        image.set_data(width as i32, height as i32, false, Format::RGBA8, &data_arr);
        texture.update(&image);
    }
}
