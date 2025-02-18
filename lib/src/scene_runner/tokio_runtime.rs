use std::{pin::Pin, sync::{Arc, Mutex}};

use godot::prelude::*;
use tokio::{runtime::{Handle, Runtime}};

use crate::godot_classes::dcl_global::DclGlobal;

struct DummyWaker;
impl futures_util::task::ArcWake for DummyWaker {
    fn wake_by_ref(_: &Arc<Self>) {}
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct TokioRuntime {
    pub runtime: Option<Arc<Runtime>>,
    waker: Option<Arc<DummyWaker>>,
    tasks: Arc<Mutex<Vec<Pin<Box<dyn futures_util::Future<Output = ()> + Send>>>>>,
}

#[godot_api]
impl INode for TokioRuntime {
    fn init(_base: Base<Node>) -> Self {
        let mut waker: Option<Arc<DummyWaker>> = None;

        #[cfg(not(target_arch = "wasm32"))]
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("dcl-godot-tokio")
            .build();

        #[cfg(target_arch = "wasm32")]
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .thread_name("dcl-godot-tokio")
            .build();

        let tasks = Arc::new(Mutex::new(Vec::new()));

        #[cfg(target_arch = "wasm32")]
        {
            waker = Some(Arc::new(DummyWaker));   
        }
        match rt {
            Ok(rt) => Self {
                runtime: Some(Arc::new(rt)),
                waker,
                tasks,
            },
            Err(e) => {
                godot_error!("{e}");
                Self { runtime: None, waker: None, tasks }
            }
        }
    }

    
    fn process(&mut self, _delta: f64) {
        self.on_tick();
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
    #[cfg(target_arch = "wasm32")]
    pub fn on_tick(&self) {
        use std::task::Poll;

        use futures_util::task::waker_ref;

        let Some(waker) = self.waker.as_ref() else {
            return;
        };

        let waker = waker_ref(&waker);
        let mut context = std::task::Context::from_waker(&*waker);

        let mut tasks = self.tasks.lock().unwrap();
        let mut i = 0;
        while i < tasks.len() {
            let task = &mut tasks[i];
            match task.as_mut().poll(&mut context) {
                Poll::Ready(_) => {
                    // Eliminar la tarea si ha terminado
                    tasks.remove(i);
                }
                Poll::Pending => {
                    i += 1;
                }
            }
        }
    }

    pub fn static_clone_handle() -> Option<Handle> {
        Some(
            DclGlobal::try_singleton()?
                .bind()
                .tokio_runtime
                .bind()
                .try_get_handle()?
                .clone(),
        )
    }

    pub fn static_tokio_godot_runtime() -> Option<Gd<TokioRuntime>> {
        Some(
            DclGlobal::try_singleton()?
                .bind()
                .tokio_runtime.clone()
        )
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn<F>(future: F)
    where
        F: futures_util::Future<Output = ()> + Send + 'static,
    {
        let Some(runtime) = Self::static_tokio_godot_runtime() else {
            return;
        };

        let runtime_binded = runtime.bind();
            
        let mut tasks = runtime_binded.tasks.lock().unwrap();
        tasks.push(Box::pin(future));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn<F>(future: F)
    where
        F: futures_util::Future<Output = ()> + Send + 'static,
    {

        if let Some(handle) = Self::static_clone_handle() {
            handle.spawn(future);
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new();
                if runtime.is_err() {
                    panic!("Failed to create runtime {:?}", runtime.err());
                }
                let runtime = runtime.unwrap();

                runtime.block_on(async move {
                    future.await;
                });
            });

            #[cfg(target_arch = "wasm32")]
            panic!("Failed to get handle in wasm runtime!");
        }
    }
}
