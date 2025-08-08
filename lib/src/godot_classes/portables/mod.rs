use std::collections::HashMap;

use godot::prelude::*;

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

    _base: Base<Node>,
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
    pub fn get_portable_experience_list(&self) -> Array<Dictionary> {
        self.portable_experiences
            .iter()
            .filter(|(_, pe)| matches!(pe.state, PortableExperienceState::Running(_)))
            .map(|(_, portable_experience)| {
                let mut item = Dictionary::new();
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
                ret.push(&portable_experience.pid);
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
                ret.push(&portable_experience.pid);
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

        let ret = GString::from(portable_experience.0);
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
