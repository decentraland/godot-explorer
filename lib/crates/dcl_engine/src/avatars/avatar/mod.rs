//! The avatar node. `DclAvatar` (godot_classes/dcl_avatar.rs) holds the state; the behaviors
//! are `impl DclAvatar` blocks split by concern in the submodules here.

pub mod anchors;
pub mod build;
pub mod emote;
pub mod emote_load;
pub mod glider;
pub mod lifecycle;
pub mod lod;
pub mod nametag;
pub mod script_api;

use godot::classes::{
    AnimationPlayer, AnimationTree, Area3D, AudioStream, AudioStreamPlayer, AudioStreamPlayer3D,
    Skeleton3D, Sprite3D, StaticBody3D, SubViewport, Timer,
};
use godot::prelude::*;

use crate::avatars::modifier_detector::AvatarModifierAreaDetector;
use crate::avatars::nameplate::NicknameUi;

/// The emote sound player: 3D, or plain when the avatar plays non-spatial audio.
pub enum EmoteAudio {
    Spatial(Gd<AudioStreamPlayer3D>),
    Plain(Gd<AudioStreamPlayer>),
}

impl EmoteAudio {
    pub fn stop(&mut self) {
        match self {
            Self::Spatial(player) => player.stop(),
            Self::Plain(player) => player.stop(),
        }
    }

    pub fn play(&mut self, stream: Gd<AudioStream>) {
        match self {
            Self::Spatial(player) => {
                player.set_stream(&stream);
                player.play();
            }
            Self::Plain(player) => {
                player.set_stream(&stream);
                player.play();
            }
        }
    }
}

/// Nodes of `avatar.tscn` the behaviors drive, resolved once in `ready`.
pub struct AvatarNodes {
    pub animation_tree: Gd<AnimationTree>,
    pub animation_player: Gd<AnimationPlayer>,
    pub skeleton: Gd<Skeleton3D>,
    pub audio_emote: EmoteAudio,
    pub audio_double_jump: Gd<AudioStreamPlayer3D>,
    pub nickname_ui: Gd<NicknameUi>,
    pub nickname_quad: Gd<Sprite3D>,
    /// Freed by `NameplateLayer::attach`: the plate lives in the shared layer instead.
    pub nickname_viewport: Option<Gd<SubViewport>>,
    pub timer_hide_mic: Gd<Timer>,
    pub click_area: Gd<Area3D>,
    /// Freed for scene NPCs, which have no trigger detection.
    pub trigger_detector: Option<Gd<StaticBody3D>>,
    pub glider_prop: Gd<Node3D>,
    pub modifier_detector: Gd<AvatarModifierAreaDetector>,
}

/// A network emote that arrived while the avatar was still loading (Pulse replays the peer's
/// last emote at join): playing it earlier would resolve against the default body shape and be
/// wiped by the rebuild, so it is replayed when the avatar is ready.
pub struct PendingEmote {
    pub urn: String,
    pub mask: i64,
    pub scene_id: i32,
}

/// The animation state machine of `avatar.tscn` itself (#2603).
///
/// Several bugs shared one shape: the AnimationTree lands in a state with no outgoing transition
/// for the current world condition, so the avatar stays frozen in a stale pose (walking
/// mid-air, standing while gliding). Instead of pinning one transition at a time, the test
/// parses the real graph out of the scene and asserts every state can escape:
///
///   - on `fall`: every state reaches a fall state, except the gliding family (they leave
///     through `ngliding` -> Gliding_End first);
///   - on `gliding`: every state reaches Gliding_Start, except Gliding_Idle.
///
/// Unconditional (at-end) edges count as always traversable, but an at-end edge out of a looping
/// clip never fires at runtime: the check is necessary, not sufficient.
#[cfg(test)]
mod state_graph_tests {
    use std::collections::{HashMap, HashSet};

    use regex::Regex;

    const TSCN: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../godot/src/decentraland_components/avatar/avatar.tscn"
    );

    /// `(from, to, condition)`, condition empty for at-end edges. Only the main machine, the one
    /// whose transition list starts at "Start".
    fn parse_edges(text: &str) -> Vec<(String, String, String)> {
        let block = Regex::new(
            r#"\[sub_resource type="AnimationNodeStateMachineTransition" id="([^"]+)"\]([^\[]*)"#,
        )
        .unwrap();
        let condition = Regex::new(r#"advance_condition = &"(\w+)""#).unwrap();
        let mut conditions: HashMap<String, String> = HashMap::new();
        for m in block.captures_iter(text) {
            let cond = condition
                .captures(&m[2])
                .map(|c| c[1].to_string())
                .unwrap_or_default();
            conditions.insert(m[1].to_string(), cond);
        }
        let list = Regex::new(r#"(?s)transitions = \[("Start".*?)\]"#).unwrap();
        let Some(list) = list.captures(text) else {
            return Vec::new();
        };
        let token = Regex::new(r#""([^"]+)"|SubResource\("([^"]+)"\)"#).unwrap();
        let raw: Vec<String> = token
            .captures_iter(&list[1])
            .map(|t| {
                t.get(1)
                    .or_else(|| t.get(2))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            })
            .collect();
        raw.chunks_exact(3)
            .map(|e| {
                (
                    e[0].clone(),
                    e[1].clone(),
                    conditions.get(&e[2]).cloned().unwrap_or_default(),
                )
            })
            .collect()
    }

    fn states_without_escape(
        edges: &[(String, String, String)],
        condition: &str,
        exempt: &[&str],
    ) -> Vec<String> {
        let mut states: HashSet<&str> = HashSet::new();
        for (from, to, _) in edges {
            states.insert(from);
            states.insert(to);
        }
        states.remove("Start");
        states.remove("End");

        // Reachable by following only at-end or `condition` edges backwards from the states a
        // `condition` edge enters.
        let mut ok: HashSet<&str> = edges
            .iter()
            .filter(|e| e.2 == condition)
            .map(|e| e.1.as_str())
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for (from, to, cond) in edges {
                if ok.contains(to.as_str())
                    && !ok.contains(from.as_str())
                    && (cond.is_empty() || cond == condition)
                {
                    ok.insert(from);
                    changed = true;
                }
            }
        }
        let mut stuck: Vec<String> = states
            .into_iter()
            .filter(|s| !exempt.contains(s) && !ok.contains(s))
            .map(String::from)
            .collect();
        stuck.sort();
        stuck
    }

    #[test]
    fn every_state_can_escape_to_fall_and_gliding() {
        let text = std::fs::read_to_string(TSCN).expect("avatar.tscn");
        let edges = parse_edges(&text);
        assert!(
            !edges.is_empty(),
            "no transitions parsed from avatar.tscn: the parser drifted from the file format"
        );
        let on_fall = states_without_escape(&edges, "fall", &["Gliding_Start", "Gliding_Idle"]);
        assert!(on_fall.is_empty(), "no escape on fall: {on_fall:?}");
        let on_gliding = states_without_escape(&edges, "gliding", &["Gliding_Idle"]);
        assert!(
            on_gliding.is_empty(),
            "no escape on gliding: {on_gliding:?}"
        );
    }

    #[test]
    fn a_trap_is_detected() {
        let edges = vec![
            ("Start".to_string(), "Idle".to_string(), String::new()),
            ("Idle".to_string(), "Fall".to_string(), "fall".to_string()),
            ("Trap".to_string(), "Trap".to_string(), "other".to_string()),
        ];
        assert_eq!(
            states_without_escape(&edges, "fall", &[]),
            vec!["Trap".to_string()]
        );
    }
}
