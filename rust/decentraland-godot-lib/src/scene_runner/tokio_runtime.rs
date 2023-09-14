use std::sync::Arc;

use godot::{engine::Engine, prelude::*};
use tokio::runtime::{Handle, Runtime};

#[derive(GodotClass)]
#[class(base = Node)]
pub struct TokioRuntime {
    pub runtime: Option<Arc<Runtime>>,
}

#[godot_api]
impl NodeVirtual for TokioRuntime {
    fn init(_base: Base<Node>) -> Self {
        match Runtime::new() {
            Ok(rt) => Self {
                runtime: Some(Arc::new(rt)),
            },
            Err(e) => {
                godot_error!("{e}");
                Self { runtime: None }
            }
        }
    }
}

#[godot_api]
impl TokioRuntime {
    /// May return the handle to the tokio runtime, or `None` if no runtime handle is obtainable.
    pub fn try_get_handle(&self) -> Option<&Handle> {
        match self.runtime.as_ref() {
            Some(rt) => Some(rt.handle()),
            None => None,
        }
    }

    /// Panics if handle is not found!
    pub fn get_handle(&self) -> &Handle {
        self.try_get_handle().expect("Failed to get handle!")
    }
}

impl TokioRuntime {
    pub fn from_node(node: Gd<Node>) -> Option<Gd<Self>> {
        let runtime = node
            .get_node("/root/Global/tokio_runtime".into())?
            .cast::<Self>();
        Some(runtime)
    }

    pub fn from_base(base: Base<Node>) -> Option<Gd<Self>> {
        let runtime = base
            .get_node("/root/Global/tokio_runtime".into())?
            .cast::<Self>();
        Some(runtime)
    }

    pub fn from_singleton() -> Option<Gd<Self>> {
        let engine_node = Engine::singleton()
            .get_main_loop()?
            .cast::<SceneTree>()
            .get_root()?;

        let global = engine_node.get_node("Global".into())?;
        let tokio_runtime = global.get_node("tokio_runtime".into())?;

        Some(tokio_runtime.cast::<Self>())
    }

    pub fn static_clone_handle() -> Option<Handle> {
        Some(Self::from_singleton()?.bind().try_get_handle()?.clone())
    }
}
