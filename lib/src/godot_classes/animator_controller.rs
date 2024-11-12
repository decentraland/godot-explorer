use std::collections::{HashMap, HashSet};

use godot::{
    builtin::{meta::ToGodot, StringName},
    engine::{
        AnimationNodeAdd2, AnimationNodeAnimation, AnimationNodeBlend2, AnimationNodeBlendTree,
        AnimationNodeTimeScale, AnimationNodeTimeSeek, AnimationPlayer, AnimationTree,
        IAnimationTree, Node3D,
    },
    obj::{Base, Gd, NewGd, WithBaseField},
    prelude::{godot_api, GodotClass},
};

use crate::dcl::components::proto_components::sdk::components::{PbAnimationState, PbAnimator};

pub const DUMMY_ANIMATION_NAME: &str = "__dummy__";
const MULTIPLE_ANIMATION_CONTROLLER_NAME: &str = "MultipleAnimator";

struct AnimationItem {
    value: PbAnimationState,
    anim_name_node: StringName,
    blend_param_ref_str: StringName,
    speed_param_ref_str: StringName,
    time_param_ref_str: StringName,
    index: usize,
}

#[derive(GodotClass)]
#[class(init, base=AnimationTree)]
pub struct MultipleAnimationController {
    base: Base<AnimationTree>,

    current_capacity: usize,
    existing_anims_duration: HashMap<String, f32>,

    current_time: HashMap<String, f32>,
    playing_anims: HashMap<String, AnimationItem>,

    finished_animations: HashSet<String>,
}

#[godot_api]
impl IAnimationTree for MultipleAnimationController {
    fn ready(&mut self) {
        // connect animation_finished
        let callable = self.base().callable("_animation_finished");
        self.base_mut()
            .connect("animation_finished".into(), callable);
    }
}

#[godot_api]
impl MultipleAnimationController {
    #[func]
    fn _animation_finished(&mut self, anim_name: StringName) {
        let anim_name = anim_name.to_string();
        if anim_name == DUMMY_ANIMATION_NAME {
            return;
        }

        // Lean comment
        //  agree with base_mut() is much safer than directly accessing the base field (you might call a method that changes the state of the object)
        //  but these cases when you only use base_mut to access godot fields which doesn't trigger any rust code :S
        //  you have to workaround it with these cloned variables

        let mut a = (None, None);
        if let Some(anim_item) = self.playing_anims.get(&anim_name.to_string()) {
            let looping = anim_item.value.r#loop.unwrap_or(true);
            // TODO: reset_on_finished is not implemented
            let reset_on_finished = false;
            if looping || reset_on_finished {
                let playing_time = if !anim_item.value.playing_backward() {
                    0.0
                } else {
                    *self
                        .existing_anims_duration
                        .get(&anim_item.value.clip)
                        .unwrap_or(&0.0)
                };

                a.0 = Some((
                    anim_item.time_param_ref_str.clone(),
                    playing_time.to_variant(),
                ));
            }

            if !looping {
                a.1 = Some((anim_item.speed_param_ref_str.clone(), 0_f32.to_variant()));
            }

            for (param, value) in [a.0, a.1].iter().flatten() {
                self.base_mut().set(param.clone(), value.clone());
            }
        } else {
            tracing::error!("finished animation {} not found!", anim_name);
        }
    }
}

impl MultipleAnimationController {
    fn new(existing_anims_duration: HashMap<String, f32>) -> Gd<MultipleAnimationController> {
        Gd::from_init_fn(|base| Self {
            base,
            current_capacity: 0,
            current_time: HashMap::new(),
            playing_anims: HashMap::new(),
            existing_anims_duration,
            finished_animations: HashSet::new(),
        })
    }

    pub fn apply_anims(&mut self, suggested_value: &PbAnimator) {
        let mut value = suggested_value.clone();
        value
            .states
            .retain(|state| self.existing_anims_duration.contains_key(&state.clip));

        let (playing_animations, stopped_animations): (_, Vec<_>) = value
            .states
            .iter()
            .partition(|state| state.playing.unwrap_or(false));

        if self.current_capacity < playing_animations.len() {
            self.generate_animation_blend_tree_needed(playing_animations.len());
        }

        let dirty_animation_playing = playing_animations
            .iter()
            .any(|playing| !self.playing_anims.contains_key(&playing.clip));
        if dirty_animation_playing {
            self.remap_animation(&playing_animations, &stopped_animations);
        }

        let mut changes = Vec::new();
        for new_state in value.states.iter() {
            let Some(anim_state) = self.playing_anims.get_mut(&new_state.clip) else {
                continue;
            };

            if anim_state.value.weight != new_state.weight {
                anim_state.value.weight = new_state.weight;
                changes.push((
                    anim_state.blend_param_ref_str.clone(),
                    new_state.weight.unwrap_or(1.0).to_variant(),
                ));
            }

            let speed = if new_state.playing.unwrap_or_default() {
                new_state.speed.unwrap_or(1.0)
            } else {
                0.0
            };
            changes.push((anim_state.speed_param_ref_str.clone(), speed.to_variant()));

            let should_reset = new_state.should_reset.unwrap_or_default();
            if should_reset {
                let playing_time = if !anim_state.value.playing_backward() {
                    0.0
                } else {
                    *self
                        .existing_anims_duration
                        .get(&anim_state.value.clip)
                        .unwrap_or(&0.0)
                };

                changes.push((
                    anim_state.time_param_ref_str.clone(),
                    playing_time.to_variant(),
                ));
            }

            anim_state.value.playing = new_state.playing;
            anim_state.value.speed = new_state.speed;
            anim_state.value.r#loop = new_state.r#loop;
            anim_state.value.should_reset = new_state.should_reset;
        }

        for (param, value) in changes {
            self.base_mut().set(param, value);
        }
    }

    fn remap_animation(
        &mut self,
        playing_animation: &Vec<&PbAnimationState>,
        stopped_animation: &Vec<&PbAnimationState>,
    ) {
        let mut changes = Vec::new();
        // First remove the animations that are not playing
        // Animation that are not playing anymore has two behavior:
        //  - if shouldReset is enabled, the animation will be reseted before removing it
        //  - otherwise, the current time is stored (to be used later)
        for anim in stopped_animation {
            let Some(anim_state) = self.playing_anims.get(&anim.clip) else {
                continue;
            };

            // TODO: should_reset will be deprecated
            let should_reset = anim_state.value.should_reset.unwrap_or_default();
            if should_reset {
                let playing_time = if !anim_state.value.playing_backward() {
                    0.0
                } else {
                    *self.existing_anims_duration.get(&anim.clip).unwrap_or(&0.0)
                };
                changes.push((
                    anim_state.time_param_ref_str.clone(),
                    playing_time.to_variant(),
                ));
                self.current_time.remove(&anim.clip);
            } else {
                let time = self.base().get(anim_state.time_param_ref_str.clone());
                if let Ok(time) = time.try_to::<f32>() {
                    self.current_time.insert(anim.clip.clone(), time);
                }
            }

            self.playing_anims.remove(&anim.clip);
        }

        for (param, value) in changes {
            self.base_mut().set(param, value);
        }

        let playing_index_values = self
            .playing_anims
            .values()
            .map(|item| item.index)
            .collect::<Vec<_>>();
        let mut available_index = (0..self.current_capacity)
            .filter(|index| !playing_index_values.contains(index))
            .collect::<Vec<_>>();

        // Then add the new animations
        for anim in playing_animation {
            if self.playing_anims.contains_key(&anim.clip) {
                continue;
            }

            let Some(index) = available_index.pop() else {
                tracing::error!("No available index to add the animation {}", anim.clip);
                continue;
            };

            let anim_item = AnimationItem {
                index,
                value: (*anim).clone(),
                anim_name_node: format!("anim_{}", index).into(),
                blend_param_ref_str: format!("parameters/blend_{}/blend_amount", index).into(),
                speed_param_ref_str: format!("parameters/sanim_{}/scale", index).into(),
                time_param_ref_str: format!("parameters/tanim_{}/seek_request", index).into(),
            };

            self.base_mut().set(
                anim_item.speed_param_ref_str.clone(),
                anim.speed.unwrap_or(1.0).to_variant(),
            );
            self.base_mut().set(
                anim_item.blend_param_ref_str.clone(),
                anim.weight.unwrap_or(1.0).to_variant(),
            );

            let playing_time = if let Some(playing_time) = self.current_time.remove(&anim.clip) {
                if self.finished_animations.contains(&anim.clip) {
                    self.finished_animations.remove(&anim.clip);
                    0.0
                } else {
                    playing_time
                }
            } else if !anim_item.value.playing_backward() {
                0.0
            } else {
                *self.existing_anims_duration.get(&anim.clip).unwrap_or(&0.0)
            };

            self.base_mut().set(
                anim_item.time_param_ref_str.clone(),
                playing_time.to_variant(),
            );

            let mut anim_node = self
                .base()
                .get_tree_root()
                .expect("Failed to get tree root")
                .cast::<AnimationNodeBlendTree>()
                .get_node(anim_item.anim_name_node.clone())
                .expect("Failed to get node")
                .cast::<AnimationNodeAnimation>();

            anim_node.set_animation(anim.clip.clone().into());

            self.playing_anims.insert(anim.clip.clone(), anim_item);
        }

        // Finally set dummy animation to not used slots
        for anim in available_index {
            let anim_name_node = format!("anim_{}", anim).into();
            let mut anim_node = self
                .base()
                .get_tree_root()
                .expect("Failed to get tree root")
                .cast::<AnimationNodeBlendTree>()
                .get_node(anim_name_node)
                .expect("Failed to get node")
                .cast::<AnimationNodeAnimation>();

            anim_node.set_animation(DUMMY_ANIMATION_NAME.into());
        }
    }

    // Generates the nodes neccesary to handle the animation
    // Note: In this implementation, the blend tree only can grow
    fn generate_animation_blend_tree_needed(&mut self, n: usize) {
        let n = n.max(2);
        let first_new_index = self.current_capacity;

        // Ensure the tree root is set
        if self.base().get_tree_root().is_none() {
            self.base_mut()
                .set_tree_root(AnimationNodeBlendTree::new_gd().upcast());
        }

        let mut tree = self
            .base_mut()
            .get_tree_root()
            .unwrap()
            .cast::<AnimationNodeBlendTree>();

        for i in first_new_index..n {
            let mut anim_node = AnimationNodeAnimation::new_gd();
            let mut dummy_anim_node = AnimationNodeAnimation::new_gd();
            let blend_anim_node = AnimationNodeBlend2::new_gd();
            let time_anim_node = AnimationNodeTimeSeek::new_gd();
            let speed_anim_node = AnimationNodeTimeScale::new_gd();

            anim_node.set_animation(DUMMY_ANIMATION_NAME.into());
            dummy_anim_node.set_animation(DUMMY_ANIMATION_NAME.into());

            tree.add_node(format!("tanim_{}", i).into(), time_anim_node.upcast());
            tree.add_node(format!("danim_{}", i).into(), dummy_anim_node.upcast());
            tree.add_node(format!("tanim_{}", i).into(), time_anim_node.upcast());
            tree.add_node(format!("sanim_{}", i).into(), speed_anim_node.upcast());
            tree.add_node(format!("blend_{}", i).into(), blend_anim_node.upcast());
            tree.add_node(format!("anim_{}", i).into(), anim_node.upcast());

            tree.connect_node(
                format!("tanim_{}", i).into(),
                0,
                format!("anim_{}", i).into(),
            );
            tree.connect_node(
                format!("sanim_{}", i).into(),
                0,
                format!("tanim_{}", i).into(),
            );
            tree.connect_node(
                format!("blend_{}", i).into(),
                0,
                format!("danim_{}", i).into(),
            );
            tree.connect_node(
                format!("blend_{}", i).into(),
                1,
                format!("sanim_{}", i).into(),
            );

            self.base_mut().set(
                format!("parameters/blend_{}/blend_amount", i).into(),
                1_f32.to_variant(),
            );

            if i < n - 1 {
                let add_node = AnimationNodeAdd2::new_gd();
                tree.add_node(format!("add_{}", i).into(), add_node.upcast());
            }
        }

        for i in first_new_index..(n - 1) {
            if i == 0 {
                tree.connect_node("add_0".into(), 0, "blend_0".into());
                tree.connect_node("add_0".into(), 1, "blend_1".into());
            } else {
                tree.connect_node(
                    format!("add_{}", i).into(),
                    0,
                    format!("add_{}", i - 1).into(),
                );
                tree.connect_node(
                    format!("add_{}", i).into(),
                    1,
                    format!("blend_{}", i + 1).into(),
                );
            }

            self.base_mut().set(
                format!("parameters/add_{}/add_amount", i).into(),
                1_f32.to_variant(),
            );
        }

        tree.connect_node("output".into(), 0, format!("add_{}", n - 2).into());
        self.current_capacity = n;
    }
}

fn create_and_add_multiple_animation_controller(
    mut gltf_node: Gd<Node3D>,
) -> Option<Gd<MultipleAnimationController>> {
    let anim_player = gltf_node.try_get_node_as::<AnimationPlayer>("AnimationPlayer")?;
    let anim_list = anim_player.get_animation_list();
    if anim_list.is_empty() {
        return None;
    }

    let mut anim_builder = MultipleAnimationController::new(
        anim_list
            .as_slice()
            .iter()
            .map(|anim_clip| {
                let anim = anim_player
                    .get_animation(StringName::from(anim_clip))
                    .unwrap();
                let anim_duration = anim.get_length();
                (anim_clip.to_string(), anim_duration)
            })
            .collect(),
    );
    anim_builder.set_name(MULTIPLE_ANIMATION_CONTROLLER_NAME.into());

    if !anim_player.has_animation(DUMMY_ANIMATION_NAME.into()) {
        anim_player
            .get_animation_library("".into())
            .unwrap()
            .add_animation(DUMMY_ANIMATION_NAME.into(), Default::default());
    }

    gltf_node.add_child(anim_builder.clone().upcast());
    anim_builder.set_animation_player("../AnimationPlayer".into());

    Some(anim_builder)
}

pub fn apply_anims(gltf_container_node: Gd<Node3D>, value: &PbAnimator) {
    if let Some(mut already_exist_node) = gltf_container_node
        .try_get_node_as::<MultipleAnimationController>(MULTIPLE_ANIMATION_CONTROLLER_NAME)
    {
        already_exist_node.bind_mut().apply_anims(value);
        return;
    }

    let mut playing_count = 0;
    for state in value.states.iter() {
        if state.playing.unwrap_or_default() {
            playing_count += 1;
            if playing_count > 1 {
                break;
            }
        }
    }

    // TODO: this is an optimizacion to avoid creating AnimationTree for every animation player
    //  with just one animation, we can use the AnimationPlayer directly, but a proper controller is needed
    // let need_multiple_animation = playing_count > 1;

    let need_multiple_animation = true;

    // For handling multiple animations, we need to create a new MultipleAnimationController
    if need_multiple_animation {
        let Some(mut new_blend_builder) =
            create_and_add_multiple_animation_controller(gltf_container_node)
        else {
            // No animations available
            return;
        };
        new_blend_builder.bind_mut().apply_anims(value);
    } else {
        todo!("single animation not implemented yet")
    }
}
