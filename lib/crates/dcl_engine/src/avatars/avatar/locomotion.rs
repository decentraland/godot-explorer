//! The locomotion animation state: the gait blend, the jump chain, the glide and the stun,
//! published as the AnimationTree conditions the state machine in `avatar.tscn` reads.

use godot::classes::{AnimationNodeAnimation, AnimationNodeBlendTree, AnimationNodeStateMachine};
use godot::prelude::*;

use crate::godot_classes::dcl_avatar::DclAvatar;

// #2856 gait blend (Unity CharacterControllerSettings): blend normalization caps per gait kind
// (0 idle / 1 walk / 2 jog / 3 run) and approach speed.
const GAIT_BLEND_MAX_SPEEDS: [f32; 4] = [0.0, 1.5, 8.0, 10.0];
const GAIT_BLEND_SPEED: f32 = 5.0;
const STILL_BLEND_THRESHOLD: f32 = 0.25;
/// #2855: Unity LongFallStunTime — the hard-landing stun animation covers the input lock.
const LONG_FALL_STUN_TIME: f32 = 0.75;
/// Unity JumpHeightStun: the stun comes from a fall HEIGHT, not from the fall-speed flag the wire
/// carries (that one trips at ~3.3m). Remotes measure their own height against this.
const HARD_LANDING_FALL_HEIGHT: f32 = 8.0;
/// The blend space's first gait clip: every gait clip shares its length.
const GAIT_IDLE_CLIP: &str = "loco/Idle";
/// Authored cycle length per gait. The clips ship retimed to one common length so the blend space
/// cannot phase them apart; these restore each gait's cadence.
const GAIT_CYCLE_LENGTHS: [f32; 4] = [2.0, 1.0, 0.666_666_7, 0.533_333_3];

const DOUBLE_JUMP_RISE: &str = "Double_Jump_Rise";

macro_rules! cond {
    ($name:literal) => {
        concat!("parameters/Locomotion/conditions/", $name)
    };
}

pub struct LocomotionState {
    /// Continuous gait blend (Unity MovementBlend), also read by the emote gate.
    movement_blend: f32,
    /// Held until the machine enters Double_Jump_Rise (or the avatar lands): a one-frame pulse
    /// races the tree's advance order and can be swallowed.
    double_jump_latch: bool,
    last_jump_count: i32,
    /// The first frame must not read a wire-provided jump_count >= 2 as a rising edge, or a remote
    /// first seen mid-double-jump plays its sound out of nothing.
    jump_count_sync_pending: bool,
    remote_fall_apex_y: f32,
    remote_stun_timer: f32,
    glide_forward_blend: f32,
    gait_clip_length: f32,
    double_jump_rise: Option<Gd<AnimationNodeAnimation>>,
}

impl Default for LocomotionState {
    fn default() -> Self {
        Self {
            movement_blend: 0.0,
            double_jump_latch: false,
            last_jump_count: 0,
            jump_count_sync_pending: true,
            remote_fall_apex_y: 0.0,
            remote_stun_timer: 0.0,
            glide_forward_blend: 0.0,
            gait_clip_length: 0.0,
            double_jump_rise: None,
        }
    }
}

/// What the avatar read from itself this frame.
#[derive(Clone, Copy, Default)]
pub struct LocomotionInput {
    pub run: bool,
    pub jog: bool,
    pub walk: bool,
    pub rise: bool,
    pub fall: bool,
    pub land: bool,
    pub grounded: bool,
    pub fall_fast: bool,
    pub long_fall: bool,
    pub jump_count: i32,
    pub stunned: bool,
    pub glide_state: i32,
    pub movement_speed: f32,
    pub position_y: f32,
    pub local: bool,
    pub emote_single: bool,
    pub emote_mixed: bool,
    /// A uniform roll in [0, 1) for the jump variation.
    pub roll: f32,
}

/// One frame of tree parameters, and what the avatar still needs from it.
#[derive(Debug, Default, PartialEq)]
pub struct LocomotionFrame {
    pub conditions: Vec<(&'static str, bool)>,
    pub gait_blend: f32,
    /// None while airborne: the cadence warp only applies to the grounded blend.
    pub gait_blend_for_cadence: Option<f32>,
    pub glide_forward_blend: f32,
    /// The double-jump clip for the air jump that just started, if one did.
    pub double_jump_clip: Option<String>,
    pub jump_rising_edge: bool,
    pub stunned: bool,
}

fn move_toward(from: f32, to: f32, delta: f32) -> f32 {
    if (to - from).abs() <= delta {
        to
    } else {
        from + (to - from).signum() * delta
    }
}

/// The time scale that puts each gait's authored cadence back on clips retimed to one length,
/// blended across the pair the blend sits between.
fn cadence_scale(blend: f32, clip_length: f32) -> f32 {
    let last = GAIT_CYCLE_LENGTHS.len() - 1;
    let blend = blend.clamp(0.0, last as f32);
    let low = blend.floor() as usize;
    let high = (low + 1).min(last);
    let t = blend - low as f32;
    let wanted = GAIT_CYCLE_LENGTHS[low] + (GAIT_CYCLE_LENGTHS[high] - GAIT_CYCLE_LENGTHS[low]) * t;
    clip_length / wanted
}

impl LocomotionState {
    pub fn step(&mut self, delta: f32, s: &LocomotionInput) -> LocomotionFrame {
        let mut c: Vec<(&'static str, bool)> = Vec::with_capacity(24);
        // Full-body emotes only from a standstill; masked (upper-body) mixes stay available while
        // moving.
        let still = self.movement_blend < STILL_BLEND_THRESHOLD;
        c.push((cond!("emote"), s.emote_single && still));
        c.push((cond!("nemote"), !s.emote_single));
        c.push((cond!("emix"), s.emote_mixed && still));
        c.push((cond!("nemix"), !s.emote_mixed));

        // #2856: continuous gait blend (Unity MovementBlend): kind from the walk/jog/run
        // classification, speed the measured horizontal speed; blend = clamp01(speed/blendMax) *
        // kind, approached at MoveAnimBlendSpeed (5/s, scaled by distance).
        let kind: usize = if s.run {
            3
        } else if s.jog {
            2
        } else if s.walk {
            1
        } else {
            0
        };
        let target = if kind > 0 {
            (s.movement_speed / GAIT_BLEND_MAX_SPEEDS[kind]).clamp(0.0, 1.0) * kind as f32
        } else {
            0.0
        };
        let rate = GAIT_BLEND_SPEED.max(GAIT_BLEND_SPEED * (target - self.movement_blend).abs());
        self.movement_blend = move_toward(self.movement_blend, target, rate * delta);

        // #2856: jump-chain selection by gait. #2855: bounce pads launch without a jump
        // (jump_count stays 0) and skip the Jump_Start anticipation.
        c.push((cond!("rise_walk"), s.rise && kind < 3));
        c.push((cond!("rise_run"), s.rise && kind == 3));
        c.push((cond!("rise_bounce"), s.rise && s.jump_count == 0));
        c.push((cond!("rise"), s.rise));
        c.push((cond!("fall"), s.fall));
        c.push((cond!("land"), s.land));
        // #1553: Unity anim thresholds — fall pose past vy<-3 (AnimationFallSpeed); apex =
        // airborne between rise end and fall start (the Mid state floats it).
        c.push((cond!("fall_fast"), s.fall_fast));
        c.push((cond!("apex"), !s.grounded && !s.rise && !s.fall_fast));
        // nfall reads is_grounded, not `land`: `land` is a short local pulse, so remotes would
        // diverge. is_grounded has the same shape on both sides.
        c.push((cond!("nfall"), s.grounded));

        // #2855: long fall and hard-landing stun. The local stun comes from the player's
        // fall-height timer; a remote derives it from its own y, since the wire only carries the
        // fall-speed flag and an 8m drop looks like a 3.3m one there.
        let stunned = if s.local {
            s.stunned
        } else {
            if s.grounded {
                if self.remote_fall_apex_y - s.position_y > HARD_LANDING_FALL_HEIGHT {
                    self.remote_stun_timer = LONG_FALL_STUN_TIME;
                }
                self.remote_fall_apex_y = s.position_y;
            } else {
                self.remote_fall_apex_y = self.remote_fall_apex_y.max(s.position_y);
            }
            self.remote_stun_timer = (self.remote_stun_timer - delta).max(0.0);
            self.remote_stun_timer > 0.0
        };
        c.push((cond!("long_fall"), s.long_fall));
        c.push((cond!("fall_short"), s.fall_fast && !s.long_fall));
        c.push((cond!("stunned"), stunned));
        c.push((cond!("nstunned"), !stunned));
        // Long_Fall exits: the hard landing (stunned) must beat the normal landing.
        c.push((cond!("land_soft"), s.grounded && !stunned));

        let mut jump_rising_edge = s.jump_count > self.last_jump_count && s.jump_count >= 2;
        if self.jump_count_sync_pending {
            jump_rising_edge = false;
            self.jump_count_sync_pending = false;
        }
        self.double_jump_latch |= jump_rising_edge;
        // #2856: every air jump re-rolls a 0|1 variation and the double-jump rise swaps between
        // the two clip variants for the current gait (Unity's Jump state blends
        // Double_Jump_<gait> x JumpVariation).
        let mut double_jump_clip = None;
        if s.jump_count != self.last_jump_count {
            let gait = if s.run {
                "Run"
            } else if s.jog {
                "Jog"
            } else {
                "Base"
            };
            let side = if s.roll < 0.5 { "" } else { "_Right" };
            double_jump_clip = Some(format!("double_jump_and_glide/Double_Jump_{gait}{side}"));
        }
        self.last_jump_count = s.jump_count;
        let gliding = s.glide_state == 1 || s.glide_state == 2;
        c.push((cond!("double_jump"), self.double_jump_latch));
        c.push((cond!("gliding"), gliding));
        c.push((cond!("ngliding"), !gliding));

        let glide_target = if s.walk || s.jog || s.run { 1.0 } else { 0.0 };
        self.glide_forward_blend = move_toward(self.glide_forward_blend, glide_target, delta * 4.0);

        LocomotionFrame {
            conditions: c,
            gait_blend: self.movement_blend,
            gait_blend_for_cadence: s.grounded.then_some(self.movement_blend),
            glide_forward_blend: self.glide_forward_blend,
            double_jump_clip,
            jump_rising_edge,
            stunned,
        }
    }

    /// Clears the double-jump latch once consumed (the state was entered) or moot (landed).
    fn settle_latch(&mut self, in_double_jump: bool, grounded: bool) {
        if self.double_jump_latch && (in_double_jump || grounded) {
            self.double_jump_latch = false;
        }
    }
}

impl DclAvatar {
    /// Resolves the state the jump variation swaps the clip of, on the avatar's own copy of the
    /// tree (`setup_emotes` duplicates it).
    pub(crate) fn locomotion_setup(&mut self) {
        let Some(nodes) = self.nodes.as_ref() else {
            return;
        };
        self.locomotion.double_jump_rise = nodes
            .animation_tree
            .get_tree_root()
            .and_then(|root| root.try_cast::<AnimationNodeBlendTree>().ok())
            .and_then(|root| root.get_node("Locomotion"))
            .and_then(|sm| sm.try_cast::<AnimationNodeStateMachine>().ok())
            .and_then(|sm| sm.get_node(DOUBLE_JUMP_RISE))
            .and_then(|node| node.try_cast::<AnimationNodeAnimation>().ok());
    }

    /// One frame of locomotion conditions. Returns `(grounded, stunned)`, what the IK gates need.
    pub(crate) fn locomotion_process(&mut self, delta: f64) -> (bool, bool) {
        let input = LocomotionInput {
            run: self.run,
            jog: self.jog,
            walk: self.walk,
            rise: self.rise,
            fall: self.fall,
            land: self.land,
            grounded: self.is_grounded,
            fall_fast: self.fall_fast,
            long_fall: self.long_fall,
            jump_count: self.jump_count,
            stunned: self.stunned,
            glide_state: self.glide_state,
            movement_speed: self.movement_speed,
            position_y: self.base().get_global_position().y,
            local: self.is_local_player,
            emote_single: self.emote.playing_single,
            emote_mixed: self.emote.playing_mixed,
            roll: godot::global::randf() as f32,
        };
        let frame = self.locomotion.step(delta as f32, &input);
        if let Some(clip) = frame.double_jump_clip.as_deref() {
            if let Some(node) = self.locomotion.double_jump_rise.as_mut() {
                node.set_animation(clip);
            }
        }
        let Some(nodes) = self.nodes.as_mut() else {
            return (input.grounded, frame.stunned);
        };
        let tree = &mut nodes.animation_tree;
        for (param, value) in &frame.conditions {
            tree.set(*param, &value.to_variant());
        }
        tree.set(
            "parameters/Locomotion/Grounded/gait/blend_position",
            &frame.gait_blend.to_variant(),
        );
        let cadence = match frame.gait_blend_for_cadence {
            Some(blend) => {
                if self.locomotion.gait_clip_length <= 0.0 {
                    self.locomotion.gait_clip_length = tree
                        .get_animation(GAIT_IDLE_CLIP)
                        .map(|anim| anim.get_length())
                        .unwrap_or(GAIT_CYCLE_LENGTHS[0]);
                }
                cadence_scale(blend, self.locomotion.gait_clip_length)
            }
            None => 1.0,
        };
        tree.set(
            "parameters/Locomotion/Grounded/cadence/scale",
            &cadence.to_variant(),
        );
        tree.set(
            "parameters/Locomotion/Gliding_Idle/Blend2/blend_amount",
            &frame.glide_forward_blend.to_variant(),
        );
        if let Some(playback) = self.locomotion_playback() {
            self.locomotion.settle_latch(
                playback.get_current_node() == DOUBLE_JUMP_RISE,
                input.grounded,
            );
        }
        if frame.jump_rising_edge {
            self.play_double_jump();
        }
        (input.grounded, frame.stunned)
    }
}

// itests live in a non-cfg(test) module so they compile into the extension and register with the
// Godot test runner.
mod itest {
    use godot::classes::animation_mixer::AnimationCallbackModeProcess;
    use godot::classes::{AnimationNodeStateMachinePlayback, AnimationTree, PackedScene};
    use godot::prelude::*;

    use super::{AnimationNodeBlendTree, AnimationNodeStateMachine};

    const AVATAR_SCENE: &str = "res://src/decentraland_components/avatar/avatar.tscn";
    const TICK: f64 = 1.0 / 60.0;
    // Two seconds of animation time: past a cross-fade and past the end of the longest clip the
    // machine can be waiting on (the at-end edges).
    const STEP_TICKS: usize = 120;
    // Let the tree run its own first update before playback.start(), or that update lands on top
    // of it and the machine restarts from Start.
    const SETTLE_TICKS: usize = 2;

    #[derive(Default, Clone, Copy)]
    struct World {
        grounded: bool,
        rise: bool,
        fall: bool,
        fall_fast: bool,
        long_fall: bool,
        stunned: bool,
        gliding: bool,
        double_jump: bool,
        emote: bool,
        emix: bool,
        gait: i32,
        jump_count: i32,
    }

    /// The condition vector the runtime publishes for a world (see `LocomotionState::step`).
    fn vector(w: World) -> [(&'static str, bool); 21] {
        [
            ("nfall", w.grounded),
            ("rise", w.rise),
            ("rise_walk", w.rise && w.gait < 3),
            ("rise_run", w.rise && w.gait == 3),
            ("rise_bounce", w.rise && w.jump_count == 0),
            ("fall", w.fall),
            ("fall_fast", w.fall_fast),
            ("fall_short", w.fall_fast && !w.long_fall),
            ("long_fall", w.long_fall),
            ("apex", !w.grounded && !w.rise && !w.fall_fast),
            ("land", false),
            ("land_soft", w.grounded && !w.stunned),
            ("stunned", w.stunned),
            ("gliding", w.gliding),
            ("double_jump", w.double_jump),
            ("emote", w.emote),
            ("emix", w.emix),
            ("nemote", !w.emote),
            ("nemix", !w.emix),
            ("nstunned", !w.stunned),
            ("ngliding", !w.gliding),
        ]
    }

    fn set_conditions(tree: &mut Gd<AnimationTree>, w: Option<World>) {
        for (name, value) in vector(w.unwrap_or_default()) {
            let value = w.is_some() && value;
            tree.set(
                &format!("parameters/Locomotion/conditions/{name}"),
                &value.to_variant(),
            );
        }
    }

    /// Seeds one state, publishes one world and returns the state the machine settles in.
    fn settle(ctx: &crate::framework::TestContext, seed: &str, w: World) -> String {
        let scene = load::<PackedScene>(AVATAR_SCENE);
        let mut inst = scene.instantiate().expect("avatar.tscn instantiates");
        // Only the animation half and the rig it binds to: the mixer advances no clip time with
        // its tracks unbound, and the unparented avatar keeps its own ready out of the test.
        let mut rig = Node3D::new_alloc();
        for path in ["AnimationPlayer", "AnimationTree", "Armature"] {
            let node = inst.get_node_as::<Node>(path);
            inst.remove_child(&node);
            rig.add_child(&node);
        }
        inst.free();
        ctx.scene_tree.clone().add_child(&rig);
        let mut tree = rig.get_node_as::<AnimationTree>("AnimationTree");
        tree.set_callback_mode_process(AnimationCallbackModeProcess::MANUAL);
        tree.set_active(true);
        let machine = tree
            .get_tree_root()
            .and_then(|r| r.try_cast::<AnimationNodeBlendTree>().ok())
            .and_then(|r| r.get_node("Locomotion"))
            .and_then(|m| m.try_cast::<AnimationNodeStateMachine>().ok())
            .expect("the Locomotion state machine");
        assert!(machine.has_node(seed), "seed '{seed}' is not a state");

        set_conditions(&mut tree, None);
        for _ in 0..SETTLE_TICKS {
            tree.advance(TICK);
        }
        set_conditions(&mut tree, Some(w));
        let mut playback: Gd<AnimationNodeStateMachinePlayback> =
            tree.get("parameters/Locomotion/playback").to();
        playback.start(seed);
        for _ in 0..STEP_TICKS {
            tree.advance(TICK);
        }
        let current = playback.get_current_node().to_string();
        rig.free();
        current
    }

    /// Behavioural regression for the locomotion state machine (#2603, #2699): drives the real
    /// tree with the runtime's condition vectors and reads the live state back. A state the world
    /// can never pull out of is the avatar frozen mid-air, or standing while gliding.
    #[godot::test::itest]
    fn itest_locomotion_states_follow_the_world(ctx: &crate::framework::TestContext) {
        let glide = World {
            gliding: true,
            fall: true,
            ..Default::default()
        };
        let fall = World {
            fall: true,
            ..Default::default()
        };
        let fall_fast = World {
            fall: true,
            fall_fast: true,
            ..Default::default()
        };
        let ground = World {
            grounded: true,
            ..Default::default()
        };
        let rise = World {
            rise: true,
            ..Default::default()
        };
        let cases: Vec<(&str, World, &[&str])> = vec![
            // Gliding wins from every airborne state: Unity starts it through an Any State
            // transition, and a state without the edge freezes the pre-glide pose (#2699).
            ("Grounded", glide, &["Gliding_Start", "Gliding_Idle"]),
            (
                "Jump_Start",
                glide,
                &["Gliding_Start", "Gliding_Idle", "Jump_Rise"],
            ),
            (
                "Run_Jump_Start",
                glide,
                &["Gliding_Start", "Gliding_Idle", "Run_Jump_Rise"],
            ),
            ("Jump_Rise", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Run_Jump_Rise", glide, &["Gliding_Start", "Gliding_Idle"]),
            (
                "Double_Jump_Rise",
                glide,
                &["Gliding_Start", "Gliding_Idle", "Jump_Fall"],
            ),
            ("Jump_Mid", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Run_Jump_Mid", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Jump_Fall", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Run_Jump_Fall", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Long_Fall", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Jump_End", glide, &["Gliding_Start", "Gliding_Idle"]),
            ("Run_Jump_End", glide, &["Gliding_Start", "Gliding_Idle"]),
            (
                "Hard_Landing",
                World {
                    stunned: true,
                    ..glide
                },
                &["Gliding_Start", "Gliding_Idle"],
            ),
            (
                "Gliding_End",
                glide,
                &["Gliding_Start", "Gliding_Idle", "Jump_Fall"],
            ),
            // Falling: no state holds a grounded pose while going down. Grounded tests `fall`,
            // not `fall_fast`, so a ledge step-off changes pose on the first falling frame.
            ("Grounded", fall, &["Jump_Fall"]),
            ("Grounded", World { gait: 1, ..fall }, &["Jump_Fall"]),
            ("Grounded", fall_fast, &["Jump_Fall"]),
            ("Jump_Mid", fall_fast, &["Jump_Fall"]),
            ("Run_Jump_Mid", fall_fast, &["Run_Jump_Fall"]),
            ("Long_Fall", fall_fast, &["Jump_Fall"]),
            (
                "Gliding_Idle",
                fall_fast,
                &["Gliding_End", "Jump_Fall", "Run_Jump_Fall", "Long_Fall"],
            ),
            // Rising, and the jump chain out of the grounded blend.
            (
                "Grounded",
                World { gait: 3, ..rise },
                &["Run_Jump_Start", "Run_Jump_Rise"],
            ),
            ("Grounded", rise, &["Jump_Start", "Jump_Rise"]),
            ("Jump_Rise", World::default(), &["Jump_Mid"]),
            ("Run_Jump_Rise", World::default(), &["Run_Jump_Mid"]),
            (
                "Jump_Rise",
                World {
                    double_jump: true,
                    ..Default::default()
                },
                &["Double_Jump_Rise"],
            ),
            ("Jump_End", rise, &["Jump_Rise", "Jump_Start"]),
            ("Long_Fall", rise, &["Jump_Rise", "Jump_Start"]),
            // An air jump rises out of the long fall whatever the gait is.
            (
                "Long_Fall",
                World {
                    gait: 3,
                    jump_count: 1,
                    ..rise
                },
                &["Jump_Rise", "Jump_Start"],
            ),
            // The double jump opens the flipper from the long fall and wins the tie.
            (
                "Long_Fall",
                World {
                    double_jump: true,
                    gait: 3,
                    jump_count: 2,
                    ..rise
                },
                &["Double_Jump_Rise"],
            ),
            // Landings, including the long fall that stuns.
            ("Jump_Fall", ground, &["Jump_End", "Grounded"]),
            ("Run_Jump_Fall", ground, &["Run_Jump_End", "Grounded"]),
            (
                "Long_Fall",
                World {
                    stunned: true,
                    ..ground
                },
                &["Hard_Landing"],
            ),
            ("Long_Fall", ground, &["Jump_End", "Grounded"]),
            ("Jump_End", ground, &["Grounded"]),
            ("Run_Jump_End", ground, &["Grounded"]),
            ("Hard_Landing", ground, &["Grounded"]),
            // Gliding out and back in.
            (
                "Gliding_Idle",
                World::default(),
                &["Gliding_End", "Grounded"],
            ),
            ("Gliding_End", fall, &["Jump_Fall"]),
            // Emotes only from a standstill, and back out of them.
            (
                "Grounded",
                World {
                    emote: true,
                    ..ground
                },
                &["Emote"],
            ),
            (
                "Grounded",
                World {
                    emix: true,
                    ..ground
                },
                &["Emote_Mix"],
            ),
            ("Emote", ground, &["Grounded"]),
            ("Emote_Mix", ground, &["Grounded"]),
            // A grounded avatar with nothing else going on stays grounded.
            ("Grounded", ground, &["Grounded"]),
        ];
        let failures: Vec<String> = cases
            .iter()
            .filter_map(|(seed, world, expected)| {
                let current = settle(ctx, seed, *world);
                (!expected.contains(&current.as_str())).then(|| {
                    format!("seed '{seed}': settled in '{current}', expected {expected:?}")
                })
            })
            .collect();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condition(frame: &LocomotionFrame, name: &str) -> bool {
        frame
            .conditions
            .iter()
            .find(|(param, _)| param.rsplit('/').next() == Some(name))
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("no condition {name}"))
    }

    fn grounded() -> LocomotionInput {
        LocomotionInput {
            grounded: true,
            land: true,
            local: true,
            ..Default::default()
        }
    }

    #[test]
    fn gait_blend_approaches_kind_scaled_speed() {
        let mut state = LocomotionState::default();
        let input = LocomotionInput {
            run: true,
            movement_speed: 10.0,
            ..grounded()
        };
        let mut frame = state.step(0.1, &input);
        // Distance-scaled rate: 5/s * |3 - 0| = 15/s, so 1.5 after 0.1 s.
        assert!((frame.gait_blend - 1.5).abs() < 1e-5);
        for _ in 0..10 {
            frame = state.step(0.1, &input);
        }
        assert!((frame.gait_blend - 3.0).abs() < 1e-5);
        // Half the walk cap blends half way to walk.
        let mut state = LocomotionState::default();
        let walk = LocomotionInput {
            walk: true,
            movement_speed: 0.75,
            ..grounded()
        };
        for _ in 0..50 {
            frame = state.step(0.1, &walk);
        }
        assert!((frame.gait_blend - 0.5).abs() < 1e-5);
    }

    #[test]
    fn full_body_emotes_wait_for_a_standstill() {
        let mut state = LocomotionState::default();
        let moving = LocomotionInput {
            jog: true,
            movement_speed: 8.0,
            emote_single: true,
            emote_mixed: true,
            ..grounded()
        };
        // The gate reads last frame's blend, as the tree saw it.
        state.step(0.5, &moving);
        let frame = state.step(0.5, &moving);
        assert!(!condition(&frame, "emote"));
        assert!(!condition(&frame, "emix"));
        assert!(!condition(&frame, "nemote"));
    }

    #[test]
    fn rise_picks_the_jump_chain_by_gait() {
        let mut state = LocomotionState::default();
        let run_jump = LocomotionInput {
            run: true,
            rise: true,
            grounded: false,
            land: false,
            jump_count: 1,
            local: true,
            ..Default::default()
        };
        let frame = state.step(0.016, &run_jump);
        assert!(condition(&frame, "rise_run"));
        assert!(!condition(&frame, "rise_walk"));
        assert!(!condition(&frame, "rise_bounce"));
        // A bounce pad launches with no jump.
        let mut state = LocomotionState::default();
        let pad = LocomotionInput {
            rise: true,
            local: true,
            ..Default::default()
        };
        assert!(condition(&state.step(0.016, &pad), "rise_bounce"));
    }

    #[test]
    fn apex_floats_until_real_falling_speed() {
        let mut state = LocomotionState::default();
        let apex = LocomotionInput {
            fall: true,
            local: true,
            ..Default::default()
        };
        let frame = state.step(0.016, &apex);
        assert!(condition(&frame, "apex"));
        let falling = LocomotionInput {
            fall_fast: true,
            ..apex
        };
        let frame = state.step(0.016, &falling);
        assert!(!condition(&frame, "apex"));
        assert!(condition(&frame, "fall_short"));
    }

    #[test]
    fn remote_stun_comes_from_fall_height() {
        let mut state = LocomotionState::default();
        let mut remote = LocomotionInput {
            local: false,
            grounded: true,
            position_y: 0.0,
            ..Default::default()
        };
        state.step(0.016, &remote);
        // Up to a 9 m apex, then land: stunned for LONG_FALL_STUN_TIME.
        remote.grounded = false;
        remote.position_y = 9.0;
        state.step(0.016, &remote);
        remote.grounded = true;
        remote.position_y = 0.5;
        let frame = state.step(0.016, &remote);
        assert!(frame.stunned);
        assert!(!condition(&frame, "land_soft"));
        let frame = state.step(1.0, &remote);
        assert!(!frame.stunned);
        // A 3 m drop never stuns, whatever the wire's fall flags said.
        remote.grounded = false;
        remote.position_y = 3.5;
        remote.long_fall = true;
        state.step(0.016, &remote);
        remote.grounded = true;
        remote.position_y = 0.5;
        assert!(!state.step(0.016, &remote).stunned);
    }

    #[test]
    fn local_stun_is_the_players() {
        let mut state = LocomotionState::default();
        let input = LocomotionInput {
            stunned: true,
            ..grounded()
        };
        let frame = state.step(0.016, &input);
        assert!(condition(&frame, "stunned"));
        assert!(!condition(&frame, "nstunned"));
    }

    #[test]
    fn double_jump_latches_and_picks_a_variant() {
        let mut state = LocomotionState::default();
        let mut input = LocomotionInput {
            jump_count: 1,
            rise: true,
            local: true,
            ..Default::default()
        };
        // First observation is never an edge.
        let frame = state.step(0.016, &input);
        assert!(!frame.jump_rising_edge);
        input.jump_count = 2;
        input.run = true;
        input.roll = 0.7;
        let frame = state.step(0.016, &input);
        assert!(frame.jump_rising_edge);
        assert_eq!(
            frame.double_jump_clip.as_deref(),
            Some("double_jump_and_glide/Double_Jump_Run_Right")
        );
        // Held across frames until the state is entered.
        let frame = state.step(0.016, &input);
        assert!(!frame.jump_rising_edge);
        assert!(condition(&frame, "double_jump"));
        state.settle_latch(true, false);
        assert!(!condition(&state.step(0.016, &input), "double_jump"));
    }

    #[test]
    fn cadence_restores_each_gait_length() {
        // Clips retimed to 1 s: idle plays at half speed, run at 1/0.533.
        assert!((cadence_scale(0.0, 1.0) - 0.5).abs() < 1e-5);
        assert!((cadence_scale(1.0, 1.0) - 1.0).abs() < 1e-5);
        assert!((cadence_scale(3.0, 1.0) - 1.875).abs() < 1e-4);
        assert!((cadence_scale(1.5, 1.0) - 1.0 / 0.833_333_3).abs() < 1e-4);
    }
}
