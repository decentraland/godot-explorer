use super::SceneEntityId;

#[derive(Debug, Clone)]
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

    pub fn to_godot_transform_3d(&self) -> godot::prelude::Transform3D {
        godot::prelude::Transform3D {
            basis: godot::prelude::Basis::from_quat(godot::prelude::Quaternion {
                x: self.rotation.x,
                y: self.rotation.y,
                z: -self.rotation.z,
                w: -self.rotation.w,
            })
            .scaled(self.scale),
            origin: godot::prelude::Vector3 {
                x: self.translation.x,
                y: self.translation.y,
                z: -self.translation.z,
            },
        }
    }

    pub fn to_godot_transform_3d_without_scaled(&self) -> godot::prelude::Transform3D {
        godot::prelude::Transform3D {
            basis: godot::prelude::Basis::from_quat(godot::prelude::Quaternion {
                x: self.rotation.x,
                y: self.rotation.y,
                z: -self.rotation.z,
                w: -self.rotation.w,
            }),
            origin: godot::prelude::Vector3 {
                x: self.translation.x,
                y: self.translation.y,
                z: -self.translation.z,
            },
        }
    }
}

impl Default for DclTransformAndParent {
    fn default() -> Self {
        Self {
            translation: godot::prelude::Vector3::ZERO,
            rotation: godot::prelude::Quaternion::default(),
            scale: godot::prelude::Vector3::ONE,
            parent: SceneEntityId::ROOT,
        }
    }
}
