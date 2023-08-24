pub mod engine;
pub mod js_runtime;
pub mod runtime_mod;

use self::js_runtime::{JsRuntime, JsRuntimeState};
use super::{
    crdt::message::process_many_messages, serialization::reader::DclReader, SceneDefinition,
    SharedSceneCrdtState,
};
use super::{RendererResponse, SceneId, SceneResponse};
use godot::prelude::godot_print;
use std::collections::HashMap;
use std::time::Duration;
use v8::Promise;

// main scene processing thread - constructs an isolate and runs the scene
pub(crate) fn scene_thread(
    scene_id: SceneId,
    scene_definition: SceneDefinition,
    content_mapping: HashMap<String, String>,
    base_url: String,
    thread_sender_to_main: std::sync::mpsc::SyncSender<SceneResponse>,
    thread_receive_from_main: tokio::sync::mpsc::Receiver<RendererResponse>,
    scene_crdt: SharedSceneCrdtState,
) {
    let mut scene_main_crdt = None;
    let main_crdt_file_path = scene_definition.main_crdt_path;
    if !main_crdt_file_path.is_empty() {
        let file = godot::engine::FileAccess::open(
            godot::prelude::GodotString::from(main_crdt_file_path),
            godot::engine::file_access::ModeFlags::READ,
        );

        if let Some(file) = file {
            let buf = file.get_buffer(file.get_length() as i64).to_vec();

            let mut stream = DclReader::new(&buf);
            let mut scene_crdt_state = scene_crdt.lock().unwrap();

            process_many_messages(&mut stream, &mut scene_crdt_state);

            let dirty = scene_crdt_state.take_dirty();
            thread_sender_to_main
                .send(SceneResponse::Ok(scene_id, dirty, Vec::new(), 0.0))
                .expect("error sending scene response!!");

            scene_main_crdt = Some(buf);
        }
    }

    let scene_file_path = scene_definition.path;
    let file = godot::engine::FileAccess::open(
        godot::prelude::GodotString::from(scene_file_path.clone()),
        godot::engine::file_access::ModeFlags::READ,
    );

    if file.is_none() {
        let err_string = format!("Scene `{scene_file_path}` not found - file is none");
        if let Err(send_err) =
            thread_sender_to_main.send(SceneResponse::Error(scene_id, format!("{err_string:?}")))
        {
            godot_print!("error sending error: {send_err:?}. original error {err_string:?}")
        }
        return;
    }

    let js_context_state = JsRuntimeState {
        scene_id,
        counter: 0,
        start_time: std::time::SystemTime::now(),
        elapsed: Duration::default(),
        thread_sender_to_main: thread_sender_to_main.clone(),
        thread_receive_from_main,
        crdt: scene_crdt,
        main_crdt: scene_main_crdt,
        logs: Vec::new(),
        main_code: file.unwrap().get_as_text().to_string(),
        dying: false,
        content_mapping,
        base_url,
    };

    // Eval Init Code
    let js_context = JsRuntime::new(js_context_state);
    if js_context.is_err() {
        let err_string = format!("Scene {:?} failed", scene_definition.title);
        if let Err(send_err) =
            thread_sender_to_main.send(SceneResponse::Error(scene_id, format!("{err_string:?}")))
        {
            godot_print!("error sending error: {send_err:?}. original error {err_string:?}")
        }
        return;
    }

    let mut js_context = js_context.unwrap();

    // Setup the global context
    match js_context.run("dcl_init", include_str!("js_modules/main.js")) {
        Ok(_) => {
            // println!("init script run");
        }
        Err(err) => {
            println!("error init script {err:?}");
            return;
        }
    };

    match js_context.run("dcl_init", "globalThis.onStart()") {
        Ok(value) => {
            // println!("onStart script run");
            if value.is_promise() {
                let _promise = v8::Local::<Promise>::try_from(value).unwrap();
                // println!("is a promise {:?}", promise.state());
            }
            let _pending_task = js_context.isolate.has_pending_background_tasks();
            // println!("there is pending tasks? {:?}", pending_task);

            js_context.isolate.perform_microtask_checkpoint();

            let _pending_task = js_context.isolate.has_pending_background_tasks();
            // println!("2) there is pending tasks? {:?}", pending_task);
        }
        Err(err) => {
            println!("error init script {err:?}");
            return;
        }
    };

    // let ctx = js_context.context.clone();
    // ctx.
    let start_time = std::time::SystemTime::now();
    let mut elapsed = Duration::default();

    loop {
        let dt = std::time::SystemTime::now()
            .duration_since(start_time)
            .unwrap_or(elapsed)
            - elapsed;
        elapsed += dt;

        match js_context.run(
            "dcl_init",
            format!("globalThis.onUpdate({:?})", dt.as_secs_f32()).as_str(),
        ) {
            Ok(value) => {
                // println!("onUpdate script run");
                if value.is_promise() {
                    let _promise = v8::Local::<Promise>::try_from(value).unwrap();
                    // println!("is a promise {:?}", promise.state());
                }
                let _pending_task = js_context.isolate.has_pending_background_tasks();
                // println!("there is pending tasks? {:?}", pending_task);

                js_context.isolate.perform_microtask_checkpoint();

                let _pending_task = js_context.isolate.has_pending_background_tasks();
                // println!("2) there is pending tasks? {:?}", pending_task);
            }
            Err(err) => {
                println!("error init script {err:?}");
                return;
            }
        };

        let state = JsRuntime::state_from(&js_context.isolate);
        if state.borrow().dying {
            break;
        }
    }

    // println!("finishing thread");

    // let scope = js_context.handle_scope(&mut js_context.isolate);

    // // Evaluate the scene code
    // let script_export = match js_context.run("scene.js", "require('~scene.js')") {
    //     Ok(value) => v8::Local::<v8::Object>::try_from(value).unwrap(),
    //     Err(err) => {
    //         println!("error scene script {:?}", err);
    //         return;
    //     }
    // };

    // let script_export = script_export.clone();

    // js_context.run("asd", "asd");

    // script_export;

    // js_context.do_scoped("initial", move |&mut scope| {
    //     script_export.get(
    //         &mut scope,
    //         v8::String::new(&mut scope, "onStart").unwrap().into(),
    //     );
    //     Ok(())
    // });

    // let init_context = v8::Context::new(&mut init_scope);
    // let init_scope = &mut v8::ContextScope::new(&mut init_scope, context);

    // let init_code = include_str!("js_modules/init_v8.js");
    // let code = v8::String::new(scope, init_code).unwrap();
    // let script = v8::Script::compile(scope, code, None).unwrap();
    // let result = script.run(scope);

    // // Eval Scene Code
    // let scene_code = SceneJsFileContent(file.unwrap().get_as_text().to_string());
    // let code = v8::String::new(scope, scene_code.0.as_str()).unwrap();
    // let script = v8::Script::compile(scope, code, None).unwrap();
    // let result = script.run(scope);

    // let script = runtime.execute_script("<loader>", "require (\"~scene.js\")");
    // let script = match script {
    //     Err(execute_script_error) => {
    //         if let Err(send_err) = thread_sender_to_main.send(SceneResponse::Error(
    //             scene_id,
    //             format!("{execute_script_error:?}"),
    //         )) {
    //             godot_print!(
    //                 "error sending error: {send_err:?}. original error {execute_script_error:?}"
    //             )
    //         }
    //         return;
    //     }
    //     Ok(script) => script,
    // };

    // run startup function
    // let result: Result<(), deno_core::anyhow::Error> =
    //     run_script(&mut runtime, &script, "onStart", (), |_| Vec::new());
    // if let Err(start_script_error) = result {
    //     // ignore failure to send failure
    //     if let Err(send_err) = state
    //         .borrow_mut()
    //         .take::<std::sync::mpsc::SyncSender<SceneResponse>>()
    //         .send(SceneResponse::Error(
    //             scene_id,
    //             format!("{start_script_error:?}"),
    //         ))
    //     {
    //         godot_print!("error sending error: {send_err:?}. original error {start_script_error:?}")
    //     }

    //     return;
    // }

    // let start_time = std::time::SystemTime::now();
    // let mut elapsed = Duration::default();

    // loop {
    //     let dt = std::time::SystemTime::now()
    //         .duration_since(start_time)
    //         .unwrap_or(elapsed)
    //         - elapsed;
    //     elapsed += dt;

    //     // js_context;
    //     // state
    //     //     .borrow_mut()
    //     //     .put(SceneElapsedTime(elapsed.as_secs_f32()));

    //     // // run the onUpdate function
    //     // let result = run_script(&mut runtime, &script, "onUpdate", (), |scope| {
    //     //     vec![v8::Number::new(scope, dt.as_secs_f64()).into()]
    //     // });

    //     // if state.borrow().try_borrow::<ShuttingDown>().is_some() {
    //     //     godot_print!("exiting from the thread {:?}", scene_id);
    //     //     return;
    //     // }

    //     // if let Err(e) = result {
    //     //     let _ = state
    //     //         .borrow_mut()
    //     //         .take::<std::sync::mpsc::SyncSender<SceneResponse>>()
    //     //         .send(SceneResponse::Error(scene_id, format!("{e:?}")));
    //     //     return;
    //     // }
    // }
}

// // helper to setup, acquire, run and return results from a script function
// fn run_script(
//     runtime: &mut JsRuntime,
//     script: &v8::Global<v8::Value>,
//     fn_name: &str,
//     messages_in: (),
//     arg_fn: impl for<'a> Fn(&mut v8::HandleScope<'a>) -> Vec<v8::Local<'a, v8::Value>>,
// ) -> Result<(), AnyError> {
//     let op_state = runtime.op_state();
//     op_state.borrow_mut().put(messages_in);

//     let promise = {
//         let scope = &mut runtime.handle_scope();
//         let script_this = v8::Local::new(scope, script.clone());
//         // get module
//         let script = v8::Local::<v8::Object>::try_from(script_this).unwrap();

//         // get function
//         let target_function =
//             v8::String::new_from_utf8(scope, fn_name.as_bytes(), v8::NewStringType::Internalized)
//                 .unwrap();
//         let Some(target_function) = script.get(scope, target_function.into()) else {
//             return Err(AnyError::msg(format!("{fn_name} is not defined")));
//         };
//         let Ok(target_function) = v8::Local::<v8::Function>::try_from(target_function) else {
//             return Err(AnyError::msg(format!("{fn_name} is not a function")));
//         };

//         // get args
//         let args = arg_fn(scope);

//         // call
//         let res = target_function.call(scope, script_this, &args);
//         let Some(res) = res else {
//             return Err(AnyError::msg(format!("{fn_name} did not return a promise")));
//         };

//         drop(args);
//         v8::Global::new(scope, res)
//     };

//     let f = runtime.resolve_value(promise);
//     futures_lite::future::block_on(f).map(|_| ())
// }

// // synchronously returns a string containing JS code from the file system
// #[op(v8)]
// fn op_require(
//     state: Rc<RefCell<OpState>>,
//     module_spec: String,
// ) -> Result<String, deno_core::error::AnyError> {
//     match module_spec.as_str() {
//         // user module load
//         "~scene.js" => Ok(state.borrow().borrow::<SceneJsFileContent>().0.clone()),
//         // core module load
//         "~system/CommunicationsController" => {
//             Ok(include_str!("js_modules/CommunicationsController.js").to_owned())
//         }
//         "~system/EngineApi" => Ok(include_str!("js_modules/EngineApi.js").to_owned()),
//         "~system/EnvironmentApi" => Ok(include_str!("js_modules/EnvironmentApi.js").to_owned()),
//         "~system/EthereumController" => {
//             Ok(include_str!("js_modules/EthereumController.js").to_owned())
//         }
//         "~system/Players" => Ok(include_str!("js_modules/Players.js").to_owned()),
//         "~system/PortableExperiences" => {
//             Ok(include_str!("js_modules/PortableExperiences.js").to_owned())
//         }
//         "~system/RestrictedActions" => {
//             Ok(include_str!("js_modules/RestrictedActions.js").to_owned())
//         }
//         "~system/Runtime" => Ok(include_str!("js_modules/Runtime.js").to_owned()),
//         "~system/Scene" => Ok(include_str!("js_modules/Scene.js").to_owned()),
//         "~system/SignedFetch" => Ok(include_str!("js_modules/SignedFetch.js").to_owned()),
//         "~system/Testing" => Ok(include_str!("js_modules/Testing.js").to_owned()),
//         "~system/UserActionModule" => Ok(include_str!("js_modules/UserActionModule.js").to_owned()),
//         "~system/UserIdentity" => Ok(include_str!("js_modules/UserIdentity.js").to_owned()),
//         _ => Err(generic_error(format!(
//             "invalid module request `{module_spec}`"
//         ))),
//     }
// }
