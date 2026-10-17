//! The glider prop and the double-jump sound, driven by the wire's `jump_count` and `glide_state`.

use godot::classes::{AnimationPlayer, AudioStreamPlayer3D};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;

// glide_state values.
const CLOSED: i32 = 0;
const OPENING: i32 = 1;
const GLIDING: i32 = 2;
const CLOSING: i32 = 3;

const HIDE_DELAY_SECONDS: f64 = 0.25;

pub struct GliderState {
    /// Previous `jump_count`, for the rising edge of the double-jump sound.
    pub last_jump_count: i32,
    /// The first tick must not read a wire `jump_count >= 2` as a rising edge: a remote first
    /// seen mid-double-jump would play the sound out of nothing.
    pub jump_sync_pending: bool,
    /// Latched so the close audio, the end clip and the hide timer are not repeated.
    pub close_initiated: bool,
    pub prop_last_state: i32,
    /// The first tick adopts the state that came in without firing open or close sounds.
    pub prop_sync_pending: bool,
    pub forward_blend: f32,
}

impl Default for GliderState {
    fn default() -> Self {
        Self {
            last_jump_count: 0,
            jump_sync_pending: true,
            close_initiated: false,
            prop_last_state: CLOSED,
            prop_sync_pending: true,
            forward_blend: 0.0,
        }
    }
}

/// What the glider prop does on a `glide_state` change.
#[derive(Debug, PartialEq, Eq)]
pub enum GliderStep {
    None,
    Open,
    Close,
    HideLater,
    CloseThenHide,
}

/// The transition for `prev -> curr`, given whether the prop shows and the close latch.
pub fn glider_step(prev: i32, curr: i32, visible: bool, close_initiated: bool) -> GliderStep {
    if prev == CLOSED && curr == OPENING {
        GliderStep::Open
    } else if (prev == OPENING || prev == GLIDING) && curr == CLOSING {
        // Start closing on the CLOSING edge: waiting for CLOSED shows open wings mid-close.
        GliderStep::Close
    } else if prev == CLOSING && curr == CLOSED && visible {
        GliderStep::HideLater
    } else if curr == CLOSED && visible && !close_initiated {
        // The state jumped straight to CLOSED without passing CLOSING.
        GliderStep::CloseThenHide
    } else {
        GliderStep::None
    }
}

impl DclAvatar {
    fn glider_audio(&self, name: &str) -> Option<Gd<AudioStreamPlayer3D>> {
        self.nodes
            .as_ref()?
            .glider_prop
            .get_node_or_null(name)?
            .try_cast()
            .ok()
    }

    fn play_glider_audio(&self, name: &str) {
        if let Some(mut player) = self.glider_audio(name) {
            player.play();
        }
    }

    fn stop_glider_audio(&self, name: &str) {
        if let Some(mut player) = self.glider_audio(name) {
            player.stop();
        }
    }

    fn glider_animation_player(&self) -> Option<Gd<AnimationPlayer>> {
        self.nodes
            .as_ref()?
            .glider_prop
            .get_node_or_null("AnimationPlayer")?
            .try_cast()
            .ok()
    }

    fn play_glider_clip(&self, clip: &str) {
        if let Some(mut player) = self.glider_animation_player() {
            if player.has_animation(clip) {
                player.play_ex().name(clip).done();
            }
        }
    }

    fn play_glider_clip_if_different(&self, clip: &str, blend: f64) {
        if let Some(mut player) = self.glider_animation_player() {
            if player.has_animation(clip) && player.get_current_animation() != clip {
                player.play_ex().name(clip).custom_blend(blend).done();
            }
        }
    }

    fn set_glider_visible(&mut self, visible: bool) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.glider_prop.set_visible(visible);
        }
    }

    fn glider_visible(&self) -> bool {
        self.nodes
            .as_ref()
            .is_some_and(|n| n.glider_prop.is_visible())
    }

    pub(crate) fn schedule_glider_hide(&mut self) {
        let Some(mut tree) = self.base().get_tree_or_null() else {
            self.hide_glider_prop();
            return;
        };
        let avatar = self.to_gd();
        let timer = tree.create_timer(HIDE_DELAY_SECONDS);
        timer.signals().timeout().connect(move || {
            if avatar.is_instance_valid() {
                avatar.clone().bind_mut().hide_glider_prop();
            }
        });
    }

    /// Skips the hide when glide was reopened during the fade-out window.
    fn hide_glider_prop(&mut self) {
        if self.glide_state == CLOSED {
            self.set_glider_visible(false);
        }
    }

    /// Shows the prop by `glide_state` transitions. It is a persistent child (rotated 180° Y to
    /// undo the Unity axis flip), so its audio and animation player stay warm across glides.
    pub(crate) fn update_glider_prop(&mut self) {
        if self.nodes.is_none() {
            return;
        }
        if self.is_avatar_shape {
            if self.glider_visible() {
                self.set_glider_visible(false);
            }
            self.glider.prop_last_state = self.glide_state;
            self.glider.prop_sync_pending = false;
            return;
        }
        let curr = self.glide_state;

        if self.glider.prop_sync_pending {
            self.glider.prop_sync_pending = false;
            self.glider.prop_last_state = curr;
            if curr == OPENING || curr == GLIDING {
                self.set_glider_visible(true);
                self.glider.close_initiated = false;
                // Straight to the idle loop: no start or open sound for a state we joined into.
                self.play_glider_clip_if_different("Glider_Idle", -1.0);
                self.play_glider_audio("AudioPlayer_Idle");
            } else if curr == CLOSING {
                // Joined mid-close: show the prop and let the CLOSING -> CLOSED step hide it.
                self.set_glider_visible(true);
                self.glider.close_initiated = true;
            }
            return;
        }

        let prev = self.glider.prop_last_state;
        self.glider.prop_last_state = curr;
        match glider_step(
            prev,
            curr,
            self.glider_visible(),
            self.glider.close_initiated,
        ) {
            GliderStep::Open => {
                self.set_glider_visible(true);
                self.glider.close_initiated = false;
                self.play_glider_audio("AudioPlayer_Open");
                self.play_glider_audio("AudioPlayer_Idle");
                self.play_glider_clip("Glider_Start");
            }
            GliderStep::Close => self.begin_glider_close(false),
            GliderStep::CloseThenHide => self.begin_glider_close(true),
            GliderStep::HideLater => self.schedule_glider_hide(),
            GliderStep::None => {}
        }

        if curr == GLIDING && self.glider_visible() {
            let moving = self.walk || self.jog || self.run;
            let clip = if moving {
                "Glider_Forward"
            } else {
                "Glider_Idle"
            };
            self.play_glider_clip_if_different(clip, 0.25);
        }
    }

    fn begin_glider_close(&mut self, then_hide: bool) {
        self.glider.close_initiated = true;
        self.stop_glider_audio("AudioPlayer_Idle");
        self.play_glider_audio("AudioPlayer_Close");
        self.play_glider_clip("Glider_End");
        if then_hide {
            self.schedule_glider_hide();
        }
    }

    pub(crate) fn play_double_jump(&mut self) {
        if let Some(nodes) = self.nodes.as_mut() {
            nodes.audio_double_jump.play();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_edge() {
        assert_eq!(glider_step(CLOSED, OPENING, false, false), GliderStep::Open);
    }

    #[test]
    fn close_edge_does_not_wait_for_closed() {
        assert_eq!(
            glider_step(GLIDING, CLOSING, true, false),
            GliderStep::Close
        );
        assert_eq!(
            glider_step(OPENING, CLOSING, true, false),
            GliderStep::Close
        );
    }

    #[test]
    fn closed_after_closing_hides_later() {
        assert_eq!(
            glider_step(CLOSING, CLOSED, true, true),
            GliderStep::HideLater
        );
        assert_eq!(glider_step(CLOSING, CLOSED, false, true), GliderStep::None);
    }

    #[test]
    fn straight_to_closed_closes_once() {
        assert_eq!(
            glider_step(GLIDING, CLOSED, true, false),
            GliderStep::CloseThenHide
        );
        assert_eq!(glider_step(GLIDING, CLOSED, true, true), GliderStep::None);
    }
}
