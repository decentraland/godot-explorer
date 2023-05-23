use super::SceneEntityId;

#[derive(Debug, Default, Clone)]
pub struct DclTransformAndParent {
    pub translation: godot::prelude::Vector3,
    pub rotation: godot::prelude::Quaternion,
    pub scale: godot::prelude::Vector3,
    pub parent: SceneEntityId,
}

impl DclTransformAndParent {
    pub fn parent(&self) -> SceneEntityId {
        self.parent
    }
}
