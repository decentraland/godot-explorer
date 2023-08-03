use super::engine::{op_crdt_recv_from_renderer, op_crdt_send_to_renderer};
use crate::dcl::{RendererResponse, SceneId, SceneResponse, SharedSceneCrdtState};
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SceneLogLevel {
    Log = 1,
    SceneError = 2,
    SystemError = 3,
}

#[derive(Clone, Debug)]
pub struct SceneLogMessage {
    pub timestamp: f64, // scene local time
    pub level: SceneLogLevel,
    pub message: String,
}

pub struct JsRuntime {
    pub isolate: v8::OwnedIsolate,
    pub context: Rc<v8::Global<v8::Context>>,
}

pub struct JsRuntimeState {
    pub scene_id: SceneId,
    pub counter: u32,
    pub start_time: std::time::SystemTime,
    pub elapsed: Duration,
    pub thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    pub thread_receive_from_main: tokio::sync::mpsc::Receiver<RendererResponse>,
    pub crdt: SharedSceneCrdtState,
    pub main_crdt: Option<Vec<u8>>,
    pub main_code: String,
    pub logs: Vec<SceneLogMessage>,
    pub dying: bool,
}

pub fn init_v8() {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}

fn js_require(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut ret: v8::ReturnValue,
) {
    if args.length() != 1 {
        return;
    }

    let module_name = args.get(0);
    if !module_name.is_string() {
        return;
    }

    let module_name = module_name.to_rust_string_lossy(scope);

    // This module can be required only once
    if module_name.as_str() == "~scene.js" {
        let state = JsRuntime::state_from(scope);
        let main_code = &mut state.borrow_mut().main_code;
        let code = std::mem::take(main_code);

        let module = v8::String::new(scope, &code).unwrap();
        ret.set(module.into());
    }

    let module = match module_name.as_str() {
        // core module load
        "~system/CommunicationsController" => {
            Some(include_str!("js_modules/CommunicationsController.js").to_owned())
        }
        "~system/EngineApi" => Some(include_str!("js_modules/EngineApi.js").to_owned()),
        "~system/EnvironmentApi" => Some(include_str!("js_modules/EnvironmentApi.js").to_owned()),
        "~system/EthereumController" => {
            Some(include_str!("js_modules/EthereumController.js").to_owned())
        }
        "~system/Players" => Some(include_str!("js_modules/Players.js").to_owned()),
        "~system/PortableExperiences" => {
            Some(include_str!("js_modules/PortableExperiences.js").to_owned())
        }
        "~system/RestrictedActions" => {
            Some(include_str!("js_modules/RestrictedActions.js").to_owned())
        }
        "~system/Runtime" => Some(include_str!("js_modules/Runtime.js").to_owned()),
        "~system/Scene" => Some(include_str!("js_modules/Scene.js").to_owned()),
        "~system/SignedFetch" => Some(include_str!("js_modules/SignedFetch.js").to_owned()),
        "~system/Testing" => Some(include_str!("js_modules/Testing.js").to_owned()),
        "~system/UserActionModule" => {
            Some(include_str!("js_modules/UserActionModule.js").to_owned())
        }
        "~system/UserIdentity" => Some(include_str!("js_modules/UserIdentity.js").to_owned()),
        _ => None,
    };

    if let Some(module) = module {
        let module = v8::String::new(scope, &module).unwrap();
        ret.set(module.into());
    }
}

fn js_console_log(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _ret: v8::ReturnValue,
) {
    let state = JsRuntime::state_from(scope);

    if args.length() != 1 {
        return;
    }

    let message = args.get(0).to_rust_string_lossy(scope);
    let time = state.borrow().elapsed.as_micros();
    state.borrow_mut().logs.push(SceneLogMessage {
        timestamp: time as f64,
        level: SceneLogLevel::Log,
        message,
    });
}

fn js_console_error(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _ret: v8::ReturnValue,
) {
    let state = JsRuntime::state_from(scope);

    if args.length() != 1 {
        return;
    }

    let message = args.get(0).to_rust_string_lossy(scope);
    let time = state.borrow().elapsed.as_micros();
    state.borrow_mut().logs.push(SceneLogMessage {
        timestamp: time as f64,
        level: SceneLogLevel::SceneError,
        message,
    });
}

impl JsRuntime {
    pub(crate) fn state_from(isolate: &v8::Isolate) -> Rc<RefCell<JsRuntimeState>> {
        let state_ptr = isolate.get_data(0);
        let state_rc =
      // SAFETY: We are sure that it's a valid pointer for whole lifetime of
      // the runtime.
      unsafe { Rc::from_raw(state_ptr as *const RefCell<JsRuntimeState>) };
        let state = state_rc.clone();
        std::mem::forget(state_rc);
        state
    }

    pub fn new(state: JsRuntimeState) -> Result<JsRuntime, String> {
        // Create the V8 sandbox
        let mut isolate = v8::Isolate::new(Default::default());
        let state_rc = Rc::new(RefCell::new(state));

        isolate.set_data(0, Rc::into_raw(state_rc) as *mut std::ffi::c_void);

        let context = {
            // Create global variables and functions
            let mut scope = v8::HandleScope::new(&mut isolate);
            let global = v8::ObjectTemplate::new(&mut scope);

            global.set(
                v8::String::new(&mut scope, "js_require").unwrap().into(),
                v8::FunctionTemplate::new(&mut scope, js_require).into(),
            );

            global.set(
                v8::String::new(&mut scope, "console_log").unwrap().into(),
                v8::FunctionTemplate::new(&mut scope, js_console_log).into(),
            );

            global.set(
                v8::String::new(&mut scope, "console_error").unwrap().into(),
                v8::FunctionTemplate::new(&mut scope, js_console_error).into(),
            );

            global.set(
                v8::String::new(&mut scope, "op_crdt_send_to_renderer")
                    .unwrap()
                    .into(),
                v8::FunctionTemplate::new(&mut scope, op_crdt_send_to_renderer).into(),
            );

            global.set(
                v8::String::new(&mut scope, "op_crdt_recv_from_renderer")
                    .unwrap()
                    .into(),
                v8::FunctionTemplate::new(&mut scope, op_crdt_recv_from_renderer).into(),
            );

            let context = v8::Context::new_from_template(&mut scope, global);

            // Wrap the context in a global object so its lifetime is unbound
            v8::Global::new(&mut scope, context)
        };

        let context = JsRuntime {
            isolate,
            context: context.into(),
        };

        Ok(context)
    }

    pub fn run_str(&mut self, filename: &str, src: &str) -> Result<String, String> {
        self.do_scoped(filename, |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).ok_or("could not build v8 string")?;
            let value = v8::Script::compile(scope, src, None)
                .ok_or("failed to compile script")?
                .run(scope)
                .ok_or("missing return value")?;
            let value = value.to_rust_string_lossy(scope);
            Ok(value)
        })
    }

    pub fn run(&mut self, filename: &str, src: &str) -> Result<v8::Local<'_, v8::Value>, String> {
        self.do_scoped(filename, |scope| {
            // Build and run the script
            let src = v8::String::new(scope, src).ok_or("could not build v8 string")?;
            let value = v8::Script::compile(scope, src, None)
                .ok_or("failed to compile script")?
                .run(scope)
                .ok_or("missing return value")?;
            Ok(value)
        })
    }

    pub fn do_scoped<'scope, T>(
        &'scope mut self,
        filename: &str,
        mut callback: impl FnMut(&mut v8::HandleScope<'scope>) -> Result<T, String>,
    ) -> Result<T, String> {
        // "Raw" script scope
        let mut scope = v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(&mut scope, &*self.context.clone());

        // Script scope with globals + error handling
        let mut scope = v8::ContextScope::new(&mut scope, context);
        let mut scope = v8::TryCatch::new(&mut scope);

        // Run user callback using the scope
        let script_result = callback(&mut scope);

        if scope.has_caught() {
            let message = scope.message().ok_or("could not extract error message")?;
            let msg = format!(
                "{} ({filename}:{})",
                message.get(&mut scope).to_rust_string_lossy(&mut scope),
                message.get_line_number(&mut scope).unwrap_or(0),
            );
            return Err(msg);
        }

        script_result
    }

    pub fn handle_scope<'s>(&self, isolate: &'s mut v8::Isolate) -> v8::HandleScope<'s> {
        v8::HandleScope::with_context(isolate, &*self.context.clone())
    }
}
