// this code is executed as the runtime is created, all scopes get these definitions

// required for async ops (engine.sendMessage is declared as async)
// Deno.core.initializeAsyncOps();

// load a cjs/node-style module
// TODO: consider using deno.land/std/node's `createRequire` directly.
// Deno's node polyfill doesn't work without the full deno runtime, and i
// note that decentraland examples use ESM syntax which deno_core does support,
// so i haven't gone very deep into making full support work.
// this is a very simplified version of the deno_std/node `createRequire` implementation.
function require(moduleName) {
    // dynamically load the module source
    var source = Deno.core.ops.op_require(moduleName);

    // create a wrapper for the imported script
    source = source.replace(/^#!.*?\n/, "");
    const head = "(function (exports, require, module, __filename, __dirname) { (function (exports, require, module, __filename, __dirname) {";
    const foot = "\n}).call(this, exports, require, module, __filename, __dirname); })";
    source = `${head}${source}${foot}`;
    const [wrapped, err] = Deno.core.evalContext(source, "file://${moduleName}");
    if (err) {
        throw err.thrown;
    }

    // create minimal context for the execution
    var module = {
        exports: {}
    };
    // call the script
    // note: `require` function base path would need to be updated for proper support
    wrapped.call(
        module.exports,             // this
        module.exports,             // exports
        require,                    // require
        module,                     // module
        moduleName.substring(1),    // __filename
        moduleName.substring(0, 1)   // __dirname
    );

    return module.exports;
}

// minimal console
const console = {
    log: function (...args) {
        Deno.core.ops.op_log("" + args.join(' '))
    },
    error: function (...args) {
        Deno.core.ops.op_error("" + args.join(' '))
    },
}

// timeout handler
globalThis.setImmediate = (fn) => Promise.resolve().then(fn)

globalThis.require = require;
globalThis.console = console;

// this does NOT seem like the nicest way to do re-exports but i can't figure out how to do it otherwise
import { Request } from "ext:deno_fetch/23_request.js"
globalThis.Request = Request;

import * as fetch from "ext:deno_fetch/26_fetch.js";
globalThis.fetch = fetch.fetch;

// we need to ensure all modules are evaluated, else deno complains in debug mode
import * as _0 from "ext:deno_url/01_urlpattern.js"
import * as _1 from "ext:deno_web/02_structured_clone.js"
import * as _2 from "ext:deno_web/04_global_interfaces.js"
import * as _3 from "ext:deno_web/05_base64.js"
import * as _4 from "ext:deno_web/08_text_encoding.js"
import * as _5 from "ext:deno_web/10_filereader.js"
import * as _6 from "ext:deno_web/13_message_port.js"
import * as _7 from "ext:deno_web/14_compression.js"
import * as _8 from "ext:deno_web/15_performance.js"