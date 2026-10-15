use std::collections::HashMap;

use godot::prelude::*;

use crate::{
    engine::local_task::spawn_local,
    fetcher::{realm_flow::world_scene_urns, scene_fetcher::SceneFetcher},
    godot_classes::dcl_global::DclGlobal,
};

use crate::dcl::scene_apis::{PortableLocation, RpcResultSender, SpawnResponse};
use crate::dcl::SceneId;

#[derive(Clone)]
pub enum PortableExperienceState {
    SpawnRequested,
    Spawning,
    Running(SceneId),
    KillRequested(SceneId),
    Killing(SceneId),
    Killed,
}

pub struct PortableExperience {
    // All refer to an identifier :S
    pid: String,
    location: PortableLocation,
    name: String,

    // How it spawns
    parent_cid: String,

    // Persistent mainly spawned by a equipped wearable, or by the explorer
    persistent: bool,
    state: PortableExperienceState,
    waiting_response: Vec<RpcResultSender<Result<SpawnResponse, String>>>,
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct DclPortableExperienceController {
    portable_experiences: HashMap<String, PortableExperience>,

    /// The urns handed to the scene fetcher as global scenes.
    desired: Vec<String>,
    entity_id_to_pid: HashMap<String, String>,
    pid_to_world: HashMap<String, String>,
    world_to_urn: HashMap<String, String>,

    _base: Base<Node>,
}

#[godot_api]
impl INode for DclPortableExperienceController {
    fn ready(&mut self) {
        let mut runner = crate::fetcher::globals::scene_runner();
        let this = self.to_gd();
        runner.connect(
            "scene_killed",
            &Callable::from_fn("portables_scene_killed", {
                let mut this = this.clone();
                move |args| {
                    this.bind_mut().on_scene_killed(args[0].to::<i32>());
                    Variant::nil()
                }
            }),
        );
        runner.connect(
            "scene_spawned",
            &Callable::from_fn("portables_scene_spawned", {
                let mut this = this.clone();
                move |args| {
                    this.bind_mut().on_scene_spawned(
                        args[0].to::<i32>(),
                        &args[1].to::<GString>().to_string(),
                    );
                    Variant::nil()
                }
            }),
        );
    }

    fn process(&mut self, _delta: f64) {
        for pid in self.consume_requested_spawn().iter_shared() {
            spawn_local(spawn_portable(self.to_gd(), pid.to_string()));
        }
        for pid in self.consume_requested_kill().iter_shared() {
            self.kill_portable_experience(&pid.to_string());
        }
    }
}

impl DclPortableExperienceController {
    fn push_desired(&self) {
        let global = DclGlobal::singleton();
        let Ok(mut fetcher) = global.get("scene_fetcher").try_to::<Gd<SceneFetcher>>() else {
            return;
        };
        let urns: Vec<GString> = self.desired.iter().map(|urn| urn.as_str().into()).collect();
        fetcher
            .bind_mut()
            .set_portable_experiences_urns(PackedStringArray::from(urns.as_slice()));
    }

    fn kill_portable_experience(&mut self, pid: &str) {
        if let Some(at) = self.desired.iter().position(|urn| urn == pid) {
            self.desired.remove(at);
        }
        self.push_desired();
    }

    fn on_scene_killed(&mut self, scene_id: i32) {
        let pid = self.announce_killed_by_scene_id(scene_id).to_string();
        if pid.is_empty() {
            return;
        }
        self.kill_portable_experience(&pid);
        if let Some(world) = self.pid_to_world.get(&pid).cloned() {
            self.kill_portable_experience(&world);
        }
    }

    fn on_scene_spawned(&mut self, scene_id: i32, entity_id: &str) {
        let Some(mut pid) = self.entity_id_to_pid.get(entity_id).cloned() else {
            return;
        };
        if !self.desired.contains(&pid) {
            return;
        }
        if let Some(world) = self.pid_to_world.get(&pid) {
            pid = world.clone();
        }
        self.announce_spawned(pid.as_str().into(), true, "".into(), scene_id);
    }

    /// Asks the scene fetcher to load `pid` as a global scene. A world's name resolves to the
    /// urn of the scene it holds.
    fn begin_spawn(&mut self, mut pid: String, world_urns: Option<Vec<String>>) {
        if dcl_core::realm::is_dcl_ens(&pid) {
            let world = pid.clone();
            pid = match self.world_to_urn.get(&world) {
                Some(urn) => urn.clone(),
                None => match world_urns.as_deref() {
                    Some([urn]) => {
                        self.world_to_urn.insert(world.clone(), urn.clone());
                        urn.clone()
                    }
                    _ => {
                        tracing::error!("the portable experience world should have only one urn");
                        return;
                    }
                },
            };
            self.pid_to_world.insert(pid.clone(), world);
        }
        let Some(urn) = dcl_core::realm::parse_urn(&pid) else {
            tracing::error!("portable experience {pid} is not an urn");
            return;
        };
        self.entity_id_to_pid.insert(urn.entity_id, pid.clone());
        self.desired.push(pid);
        self.push_desired();
    }
}

async fn spawn_portable(mut this: Gd<DclPortableExperienceController>, pid: String) {
    let cached = dcl_core::realm::is_dcl_ens(&pid) && this.bind().world_to_urn.contains_key(&pid);
    let world_urns = if dcl_core::realm::is_dcl_ens(&pid) && !cached {
        world_scene_urns(&pid).await
    } else {
        None
    };
    this.bind_mut().begin_spawn(pid, world_urns);
}

#[godot_api]
impl DclPortableExperienceController {
    pub fn spawn(
        &mut self,
        location: PortableLocation,
        response: RpcResultSender<Result<SpawnResponse, String>>,
        parent_cid: &str,
        persistent: bool,
    ) {
        let (pid, existing_portable_experience) = match &location {
            PortableLocation::Ens(ens_str) => {
                let look_at_location = PortableLocation::Ens(ens_str.clone());
                let pe = self
                    .portable_experiences
                    .iter_mut()
                    .find(|(_, pe)| pe.location == look_at_location)
                    .map(|(_, pe)| pe);

                (ens_str.clone(), pe)
            }
            PortableLocation::Urn(urn) => (urn.clone(), self.portable_experiences.get_mut(urn)),
        };

        let portable_experience = match existing_portable_experience {
            Some(pe) => pe,
            None => {
                let _ = self.portable_experiences.insert(
                    pid.clone(),
                    PortableExperience {
                        pid: pid.clone(),
                        parent_cid: parent_cid.to_owned(),
                        persistent,
                        state: PortableExperienceState::SpawnRequested,
                        location: location.clone(),
                        name: String::new(),
                        waiting_response: Vec::new(),
                    },
                );

                self.portable_experiences.get_mut(&pid).unwrap()
            }
        };

        match portable_experience.state.clone() {
            PortableExperienceState::SpawnRequested | PortableExperienceState::Spawning => {
                portable_experience.waiting_response.push(response);
            }
            PortableExperienceState::Killed => {
                portable_experience.state = PortableExperienceState::SpawnRequested;
                portable_experience.waiting_response.push(response);
            }
            PortableExperienceState::KillRequested(_) | PortableExperienceState::Killing(_) => {
                response.send(Err(
                    "operation not available, the portable experiences is being killed".to_string(),
                ))
            }
            PortableExperienceState::Running(_) => response.send(Ok(SpawnResponse {
                pid: portable_experience.pid.clone(),
                parent_cid: portable_experience.parent_cid.clone(),
                name: portable_experience.name.clone(),
                ens: match &portable_experience.location {
                    PortableLocation::Ens(ens) => Some(ens.clone()),
                    _ => None,
                },
            })),
        }
    }

    pub fn kill(&mut self, location: PortableLocation, response: RpcResultSender<bool>) {
        let existing_portable_experience = self
            .portable_experiences
            .iter_mut()
            .find(|(_, pe)| pe.location == location);

        let Some(existing_portable_experience) = existing_portable_experience else {
            response.send(false);
            return;
        };
        let existing_portable_experience = existing_portable_experience.1;

        let status_ok = match existing_portable_experience.state.clone() {
            PortableExperienceState::Running(scene_id) => {
                existing_portable_experience.state =
                    PortableExperienceState::KillRequested(scene_id);
                true
            }
            PortableExperienceState::Killing(_) | PortableExperienceState::KillRequested(_) => true,
            _ => false,
        };

        response.send(status_ok);
    }

    pub fn get_running_portable_experience_list(&self) -> Vec<SpawnResponse> {
        self.portable_experiences
            .iter()
            .filter(|(_, pe)| matches!(pe.state, PortableExperienceState::Running(_)))
            .map(|(_, portable_experience)| SpawnResponse {
                pid: portable_experience.pid.clone(),
                parent_cid: portable_experience.parent_cid.clone(),
                name: portable_experience.name.clone(),
                ens: match &portable_experience.location {
                    PortableLocation::Ens(ens) => Some(ens.clone()),
                    _ => None,
                },
            })
            .collect()
    }

    #[func]
    pub fn get_portable_experience_list(&self) -> Array<VarDictionary> {
        self.portable_experiences
            .iter()
            .filter(|(_, pe)| matches!(pe.state, PortableExperienceState::Running(_)))
            .map(|(_, portable_experience)| {
                let mut item = VarDictionary::new();
                item.set("pid", portable_experience.pid.clone());
                match portable_experience.state {
                    PortableExperienceState::Running(scene_id)
                    | PortableExperienceState::Killing(scene_id)
                    | PortableExperienceState::KillRequested(scene_id) => {
                        item.set("scene_id", scene_id.0);
                    }
                    _ => {}
                }

                match portable_experience.state {
                    PortableExperienceState::Running(_) => item.set("state", "running"),
                    PortableExperienceState::Killing(_) => item.set("state", "killing"),
                    PortableExperienceState::KillRequested(_) => {
                        item.set("state", "kill_requested")
                    }
                    PortableExperienceState::SpawnRequested => item.set("state", "spawn_requested"),
                    PortableExperienceState::Spawning => item.set("state", "spawning"),
                    PortableExperienceState::Killed => item.set("state", "killed"),
                }

                item.set("name", portable_experience.name.clone());
                item
            })
            .collect()
    }

    #[func]
    pub fn consume_requested_spawn(&mut self) -> Array<GString> {
        let mut ret = Array::new();
        for (_, portable_experience) in self.portable_experiences.iter_mut() {
            if let PortableExperienceState::SpawnRequested = portable_experience.state {
                portable_experience.state = PortableExperienceState::Spawning;
                ret.push(&GString::from(portable_experience.pid.as_str()));
            }
        }
        ret
    }

    #[func]
    pub fn consume_requested_kill(&mut self) -> Array<GString> {
        let mut ret = Array::new();
        for (_, portable_experience) in self.portable_experiences.iter_mut() {
            if let PortableExperienceState::KillRequested(scene_id) = portable_experience.state {
                portable_experience.state = PortableExperienceState::Killing(scene_id);
                ret.push(&GString::from(portable_experience.pid.as_str()));
            }
        }
        ret
    }

    #[func]
    pub fn announce_killed_by_scene_id(&mut self, killed_scene_id: i32) -> GString {
        let killed_scene_id = SceneId(killed_scene_id);
        let Some(portable_experience) =
            self.portable_experiences
                .iter_mut()
                .find(
                    |(_, portable_experience)| match portable_experience.state.clone() {
                        PortableExperienceState::Killing(scene_id)
                        | PortableExperienceState::KillRequested(scene_id)
                        | PortableExperienceState::Running(scene_id) => scene_id == killed_scene_id,
                        _ => false,
                    },
                )
        else {
            return GString::default();
        };

        let ret = portable_experience.0.to_godot();
        if portable_experience.1.persistent {
            match portable_experience.1.state.clone() {
                PortableExperienceState::Killing(_) => {
                    portable_experience.1.state = PortableExperienceState::Killed;
                }
                _ => {
                    tracing::error!("announce_killed: portable experience is not being killed");
                }
            }
        } else {
            let pid = portable_experience.0.clone();
            let _ = self.portable_experiences.remove(&pid);
        }
        ret
    }

    #[func]
    pub fn announce_spawned(&mut self, pid: GString, success: bool, name: GString, scene_id: i32) {
        let Some(portable_experience) = self.portable_experiences.get_mut(&pid.to_string()) else {
            tracing::error!("announce_spawned: portable experience not found");
            return;
        };

        if !success {
            for waiting_response in portable_experience.waiting_response.drain(..) {
                waiting_response.send(Err("spawn failed".to_string()));
            }
            if portable_experience.persistent {
                portable_experience.state = PortableExperienceState::Killed;
            } else {
                let _ = self.portable_experiences.remove(&pid.to_string());
            }
        } else {
            portable_experience.state = PortableExperienceState::Running(SceneId(scene_id));
            portable_experience.name = name.to_string();
            for waiting_response in portable_experience.waiting_response.drain(..) {
                waiting_response.send(Ok(SpawnResponse {
                    pid: portable_experience.pid.clone(),
                    parent_cid: portable_experience.parent_cid.clone(),
                    name: portable_experience.name.clone(),
                    ens: match &portable_experience.location {
                        PortableLocation::Ens(ens) => Some(ens.clone()),
                        _ => None,
                    },
                }));
            }
        }
    }
}
