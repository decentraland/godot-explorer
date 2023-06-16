use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use deno_core::{
    error::{generic_error, AnyError},
    include_js_files, op, v8, Extension, JsRuntime, OpState, RuntimeOptions,
};
use godot::prelude::godot_print;

use super::{
    crdt::message::process_many_messages, serialization::reader::DclReader, SceneDefinition,
};
use super::{RendererResponse, SceneId, SceneResponse, VM_HANDLES};
use crate::dcl::crdt::SceneCrdtState;

struct SceneJsFileContent(pub String);
struct SceneMainCrdtFileContent(pub Vec<u8>);

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
pub struct SceneElapsedTime(pub f32);

pub mod engine;

// marker to indicate shutdown has been triggered
pub struct ShuttingDown;

// main scene processing thread - constructs an isolate and runs the scene
pub(crate) fn scene_thread(
    scene_id: SceneId,
    scene_definition: SceneDefinition,
    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    thread_receive_from_main: tokio::sync::mpsc::Receiver<RendererResponse>,
    scene_crdt: Arc<Mutex<SceneCrdtState>>,
) {
    let ext = Extension::builder("decentraland")
        // add require operation
        .ops(vec![op_require::decl(), op_log::decl(), op_error::decl()])
        // add plugin registrations
        .ops(engine::ops())
        // set startup JS script
        .js(include_js_files!(
            prefix "dcl_core_init",
            "js_modules/init.js",
        ))
        // remove core deno ops that are not required
        .middleware(|op| {
            const ALLOW: [&str; 7] = [
                "op_eval_context",
                "op_require",
                "op_crdt_send_to_renderer",
                "op_crdt_recv_from_renderer",
                "op_print",
                "op_log",
                "op_error",
            ];
            if ALLOW.contains(&op.name) {
                op
            } else {
                op.disable()
            }
        })
        .build();

    // create runtime
    let mut runtime = JsRuntime::new(RuntimeOptions {
        v8_platform: v8::Platform::new(1, false).make_shared().into(),
        extensions_with_js: vec![ext],
        ..Default::default()
    });

    // store handle
    let vm_handle = runtime.v8_isolate().thread_safe_handle();
    let mut guard = VM_HANDLES.lock().unwrap();
    guard.insert(scene_id, vm_handle);
    drop(guard);

    let mut scene_main_crdt = None;
    let main_crdt_file_path = scene_definition.main_crdt_path;
    if !main_crdt_file_path.is_empty() {
        let file = godot::engine::FileAccess::open(
            godot::prelude::GodotString::from(main_crdt_file_path),
            godot::engine::file_access::ModeFlags::READ,
        );

        if let Some(file) = file {
            let buf = file.get_buffer(file.get_length()).to_vec();

            let mut stream = DclReader::new(&buf);
            let mut scene_crdt_state = scene_crdt.lock().unwrap();

            process_many_messages(&mut stream, &mut scene_crdt_state);

            let dirty = scene_crdt_state.take_dirty();
            thread_sender_to_main
                .send(SceneResponse::Ok(scene_id, dirty, Vec::new(), 0.0))
                .expect("error sending scene response!!");

            scene_main_crdt = Some(SceneMainCrdtFileContent(buf));
        }
    }

    let state = runtime.op_state();

    // store log output and initial elapsed of zero
    state.borrow_mut().put(Vec::<SceneLogMessage>::default());
    state.borrow_mut().put(SceneElapsedTime(0.0));

    // store scene detail in the runtime state
    state.borrow_mut().put(scene_crdt);

    // store channels
    state.borrow_mut().put(thread_sender_to_main);
    state.borrow_mut().put(thread_receive_from_main);
    state.borrow_mut().put(scene_id);

    if let Some(scene_main_crdt) = scene_main_crdt {
        state.borrow_mut().put(scene_main_crdt);
    }

    // store kill handle
    state
        .borrow_mut()
        .put(runtime.v8_isolate().thread_safe_handle());

    let scene_file_path = scene_definition.path;
    let file = godot::engine::FileAccess::open(
        godot::prelude::GodotString::from(scene_file_path.clone()),
        godot::engine::file_access::ModeFlags::READ,
    );

    if file.is_none() {
        let err_string = format!("Scene `{scene_file_path}` not found - file is none");
        if let Err(send_err) = state
            .borrow_mut()
            .take::<std::sync::mpsc::SyncSender<SceneResponse>>()
            .send(SceneResponse::Error(scene_id, format!("{err_string:?}")))
        {
            godot_print!("error sending error: {send_err:?}. original error {err_string:?}")
        }
        return;
    }

    let scene_code = SceneJsFileContent(file.unwrap().get_as_text(true).to_string());
    state.borrow_mut().put(scene_code);

    let script = runtime.execute_script("<loader>", "require (\"~scene.js\")");
    let script = match script {
        Err(execute_script_error) => {
            if let Err(send_err) = state
                .borrow_mut()
                .take::<std::sync::mpsc::SyncSender<SceneResponse>>()
                .send(SceneResponse::Error(
                    scene_id,
                    format!("{execute_script_error:?}"),
                ))
            {
                godot_print!(
                    "error sending error: {send_err:?}. original error {execute_script_error:?}"
                )
            }
            return;
        }
        Ok(script) => script,
    };

    // run startup function
    let result: Result<(), deno_core::anyhow::Error> =
        run_script(&mut runtime, &script, "onStart", (), |_| Vec::new());
    if let Err(start_script_error) = result {
        // ignore failure to send failure
        if let Err(send_err) = state
            .borrow_mut()
            .take::<std::sync::mpsc::SyncSender<SceneResponse>>()
            .send(SceneResponse::Error(
                scene_id,
                format!("{start_script_error:?}"),
            ))
        {
            godot_print!("error sending error: {send_err:?}. original error {start_script_error:?}")
        }

        return;
    }

    let start_time = std::time::SystemTime::now();
    let mut elapsed = Duration::default();

    loop {
        let dt = std::time::SystemTime::now()
            .duration_since(start_time)
            .unwrap_or(elapsed)
            - elapsed;
        elapsed += dt;

        state
            .borrow_mut()
            .put(SceneElapsedTime(elapsed.as_secs_f32()));

        // run the onUpdate function
        let result = run_script(&mut runtime, &script, "onUpdate", (), |scope| {
            vec![v8::Number::new(scope, dt.as_secs_f64()).into()]
        });

        if state.borrow().try_borrow::<ShuttingDown>().is_some() {
            godot_print!("exiting from the thread {:?}", scene_id);
            return;
        }

        if let Err(e) = result {
            let _ = state
                .borrow_mut()
                .take::<std::sync::mpsc::SyncSender<SceneResponse>>()
                .send(SceneResponse::Error(scene_id, format!("{e:?}")));
            return;
        }
    }
}

// helper to setup, acquire, run and return results from a script function
fn run_script(
    runtime: &mut JsRuntime,
    script: &v8::Global<v8::Value>,
    fn_name: &str,
    messages_in: (),
    arg_fn: impl for<'a> Fn(&mut v8::HandleScope<'a>) -> Vec<v8::Local<'a, v8::Value>>,
) -> Result<(), AnyError> {
    let op_state = runtime.op_state();
    op_state.borrow_mut().put(messages_in);

    let promise = {
        let scope = &mut runtime.handle_scope();
        let script_this = v8::Local::new(scope, script.clone());
        // get module
        let script = v8::Local::<v8::Object>::try_from(script_this).unwrap();

        // get function
        let target_function =
            v8::String::new_from_utf8(scope, fn_name.as_bytes(), v8::NewStringType::Internalized)
                .unwrap();
        let Some(target_function) = script.get(scope, target_function.into()) else {
            return Err(AnyError::msg(format!("{fn_name} is not defined")));
        };
        let Ok(target_function) = v8::Local::<v8::Function>::try_from(target_function) else {
            return Err(AnyError::msg(format!("{fn_name} is not a function")));
        };

        // get args
        let args = arg_fn(scope);

        // call
        let res = target_function.call(scope, script_this, &args);
        let Some(res) = res else {
            return Err(AnyError::msg(format!("{fn_name} did not return a promise")));
        };

        drop(args);
        v8::Global::new(scope, res)
    };

    let f = runtime.resolve_value(promise);
    futures_lite::future::block_on(f).map(|_| ())
}

// synchronously returns a string containing JS code from the file system
#[op(v8)]
fn op_require(
    state: Rc<RefCell<OpState>>,
    module_spec: String,
) -> Result<String, deno_core::error::AnyError> {
    match module_spec.as_str() {
        // user module load
        "~scene.js" => Ok(state.borrow().borrow::<SceneJsFileContent>().0.clone()),
        // core module load
        "~system/CommunicationsController" => {
            Ok(include_str!("js_modules/CommunicationsController.js").to_owned())
        }
        "~system/EngineApi" => Ok(include_str!("js_modules/EngineApi.js").to_owned()),
        "~system/EnvironmentApi" => Ok(include_str!("js_modules/EnvironmentApi.js").to_owned()),
        "~system/EthereumController" => {
            Ok(include_str!("js_modules/EthereumController.js").to_owned())
        }
        "~system/Players" => Ok(include_str!("js_modules/Players.js").to_owned()),
        "~system/PortableExperiences" => {
            Ok(include_str!("js_modules/PortableExperiences.js").to_owned())
        }
        "~system/RestrictedActions" => {
            Ok(include_str!("js_modules/RestrictedActions.js").to_owned())
        }
        "~system/Runtime" => Ok(include_str!("js_modules/Runtime.js").to_owned()),
        "~system/Scene" => Ok(include_str!("js_modules/Scene.js").to_owned()),
        "~system/SignedFetch" => Ok(include_str!("js_modules/SignedFetch.js").to_owned()),
        "~system/Testing" => Ok(include_str!("js_modules/Testing.js").to_owned()),
        "~system/UserActionModule" => Ok(include_str!("js_modules/UserActionModule.js").to_owned()),
        "~system/UserIdentity" => Ok(include_str!("js_modules/UserIdentity.js").to_owned()),
        _ => Err(generic_error(format!(
            "invalid module request `{module_spec}`"
        ))),
    }
}

#[op(v8)]
fn op_log(state: Rc<RefCell<OpState>>, message: String) {
    let time = state.borrow().borrow::<SceneElapsedTime>().0;
    state
        .borrow_mut()
        .borrow_mut::<Vec<SceneLogMessage>>()
        .push(SceneLogMessage {
            timestamp: time as f64,
            level: SceneLogLevel::Log,
            message,
        })
}

#[op(v8)]
fn op_error(state: Rc<RefCell<OpState>>, message: String) {
    let time = state.borrow().borrow::<SceneElapsedTime>().0;
    state
        .borrow_mut()
        .borrow_mut::<Vec<SceneLogMessage>>()
        .push(SceneLogMessage {
            timestamp: time as f64,
            level: SceneLogLevel::SceneError,
            message,
        })
}
