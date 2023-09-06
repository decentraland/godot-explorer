globalThis.WebAssembly.Instance = function () {
    throw new Error('Wasm is not allowed in scene runtimes')
}
globalThis.WebAssembly.Module = function () {
    throw new Error('Wasm is not allowed in scene runtimes')
}


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

function customLog(...values) {
    return values.map(value => logValue(value, new WeakSet())).join(' ')
}

function logValue(value, seen) {
    const valueType = typeof value
    if (valueType === 'number' || valueType === 'string' || valueType === 'boolean') {
        return JSON.stringify(value)
    } else if (valueType === 'function') {
        return '[Function]'
    } else if (value === null) {
        return 'null'
    } else if (Array.isArray(value)) {
        if (seen.has(value)) {
            return '[CircularArray]';
        } else {
            seen.add(value);
            return `Array(${value.length}) [${value.map(item => logValue(item, seen)).join(', ')}]`;
        }
    } else if (valueType === 'object') {
        if (seen.has(value)) {
            return '[CircularObject]'
        } else {
            seen.add(value);
            return `Object {${Object.keys(value).map(key => `${key}: ${logValue(value[key], seen)}`).join(', ')}}`;
        }
    } else if (valueType === 'symbol') {
        return `Symbol (${value.toString()})`;
    } else if (valueType === 'bigint') {
        return `BigInt (${value.toString()})`;
    } else {
        return '[Unsupported Type]';
    }
}

// minimal console
const console = {
    log: function (...args) {
        Deno.core.ops.op_log("LOG " + customLog(...args))
    },
    error: function (...args) {
        Deno.core.ops.op_error("ERROR " + customLog(...args))
    },
    warn: function (...args) {
        Deno.core.ops.op_log("WARN " + customLog(...args))
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