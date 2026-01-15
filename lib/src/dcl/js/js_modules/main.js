if (globalThis.WebAssembly !== undefined) {
    globalThis.WebAssembly.Instance = function () {
        throw new Error('Wasm is not allowed in scene runtimes')
    }
    globalThis.WebAssembly.Module = function () {
        throw new Error('Wasm is not allowed in scene runtimes')
    }
}

// Scene logging: wrap individual op functions to intercept all op calls
// This is only active when the scene_logging feature is enabled
(function setupOpLogging() {
    const ops = Deno.core.ops;
    const logOpStart = ops.op_scene_log_op_start;
    const logOpEnd = ops.op_scene_log_op_end;

    // Only set up logging if the logging ops are available
    if (typeof logOpStart !== 'function' || typeof logOpEnd !== 'function') {
        return;
    }

    // Global call ID counter for correlation
    let nextCallId = 1;

    // Ops to exclude from logging (to avoid infinite recursion and noise)
    const excludedOps = new Set([
        'op_scene_log_op_start',  // Don't log the logging ops themselves
        'op_scene_log_op_end',
        'op_log',                 // Console logging
        'op_error',               // Console errors
        'op_require',             // Module loading
    ]);

    // Helper to safely serialize a value for logging
    function safeSerialize(value, depth = 0) {
        if (depth > 3) return '[max depth]';
        if (value === undefined) return undefined;
        if (value === null) return null;

        const type = typeof value;
        if (type === 'number' || type === 'string' || type === 'boolean') {
            return value;
        }
        if (type === 'function') {
            return '[Function]';
        }
        if (value instanceof Uint8Array || value instanceof ArrayBuffer) {
            const len = value.byteLength || value.length;
            return { _type: 'binary', length: len };
        }
        if (Array.isArray(value)) {
            if (value.length > 10) {
                return { _type: 'array', length: value.length, sample: value.slice(0, 5).map(v => safeSerialize(v, depth + 1)) };
            }
            return value.map(v => safeSerialize(v, depth + 1));
        }
        if (type === 'object') {
            const result = {};
            const keys = Object.keys(value);
            const maxKeys = 20;
            for (let i = 0; i < Math.min(keys.length, maxKeys); i++) {
                const key = keys[i];
                result[key] = safeSerialize(value[key], depth + 1);
            }
            if (keys.length > maxKeys) {
                result._truncated = keys.length - maxKeys;
            }
            return result;
        }
        return String(value);
    }

    // Wrap a single op function
    function wrapOp(opName, originalFn) {
        return function(...args) {
            const callId = nextCallId++;
            const startTime = performance.now();

            // Log the start of the call
            try {
                logOpStart({
                    call_id: callId,
                    op_name: opName,
                    args: safeSerialize(args)
                });
            } catch (_) {}

            try {
                const result = originalFn.apply(this, args);

                // Check if result is a Promise
                if (result && typeof result.then === 'function') {
                    return result.then(
                        (resolvedValue) => {
                            const duration = performance.now() - startTime;
                            try {
                                logOpEnd({
                                    call_id: callId,
                                    op_name: opName,
                                    result: safeSerialize(resolvedValue),
                                    is_async: true,
                                    duration_ms: duration,
                                    error: null
                                });
                            } catch (_) {}
                            return resolvedValue;
                        },
                        (err) => {
                            const duration = performance.now() - startTime;
                            try {
                                logOpEnd({
                                    call_id: callId,
                                    op_name: opName,
                                    result: null,
                                    is_async: true,
                                    duration_ms: duration,
                                    error: err ? (err.message || String(err)) : 'Unknown error'
                                });
                            } catch (_) {}
                            throw err;
                        }
                    );
                }

                // Synchronous call completed successfully
                const duration = performance.now() - startTime;
                try {
                    logOpEnd({
                        call_id: callId,
                        op_name: opName,
                        result: safeSerialize(result),
                        is_async: false,
                        duration_ms: duration,
                        error: null
                    });
                } catch (_) {}

                return result;
            } catch (err) {
                // Synchronous call failed
                const duration = performance.now() - startTime;
                try {
                    logOpEnd({
                        call_id: callId,
                        op_name: opName,
                        result: null,
                        is_async: false,
                        duration_ms: duration,
                        error: err ? (err.message || String(err)) : 'Unknown error'
                    });
                } catch (_) {}
                throw err;
            }
        };
    }

    // Wrap each op function individually
    for (const opName of Object.keys(ops)) {
        if (excludedOps.has(opName)) continue;
        if (typeof ops[opName] !== 'function') continue;

        try {
            const original = ops[opName];
            ops[opName] = wrapOp(opName, original);
        } catch (_) {
            // Skip if we can't modify this op (e.g., if it's non-configurable)
        }
    }
})();


function require(moduleName) {
    // dynamically load the module source
    var source = Deno.core.ops.op_require(moduleName);

    source = `(function (exports, require, module, __filename, __dirname) { (function (exports, require, module, __filename, __dirname) {${source}}).call(this, exports, require, module, __filename, __dirname); })`
    const [wrapped, err] = Deno.core.evalContext(source, `file://${moduleName}`);
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

            const objName = value?.constructor?.name ?? 'Object'
            if (objName === 'Object') {
                return `Object {${Object.keys(value).map(key => `${key}: ${logValue(value[key], seen)}`).join(', ')}}`;
            } else {
                if (value instanceof Error) {
                    return `[${objName} ${value.message} ${value.stack}`;
                } else {
                    return `${objName} {${Object.keys(value).map(key => `${key}: ${logValue(value[key], seen)}`).join(', ')}}`;
                }
            }
        }
    } else if (valueType === 'symbol') {
        return `Symbol (${value.toString()})`;
    } else if (valueType === 'bigint') {
        return `BigInt (${value.toString()})`;
    } else if (valueType === 'undefined') {
        return 'undefined';
    } else {
        return `[Unsupported Type = ${valueType} toString() ${value?.toString ? value.toString() : 'none'} valueOf() ${value}}]`;
    }
}

// minimal console
const console = {
    log: function (...args) {
        Deno.core.ops.op_log("LOG " + customLog(...args), DEBUG)
    },
    error: function (...args) {
        Deno.core.ops.op_error("ERROR " + customLog(...args), DEBUG)
    },
    warn: function (...args) {
        Deno.core.ops.op_log("WARN " + customLog(...args), DEBUG)
    },
    trace: function (...args) {
        Deno.core.ops.op_log("TRACE " + customLog(...args), DEBUG)
    },
    debug: function (...args) {
        Deno.core.ops.op_log("DEBUG " + customLog(...args), DEBUG)
    },
    info: function (...args) {
        Deno.core.ops.op_log("INFO " + customLog(...args), DEBUG)
    }
}

const _internal_console = {
    log: function (...args) {
        Deno.core.ops.op_log("LOG " + customLog(...args), true)
    },
    error: function (...args) {
        Deno.core.ops.op_error("ERROR " + customLog(...args), true)
    },
    warn: function (...args) {
        Deno.core.ops.op_log("WARN " + customLog(...args), true)
    },
    trace: function (...args) {
        Deno.core.ops.op_log("TRACE " + customLog(...args), true)
    },
    debug: function (...args) {
        Deno.core.ops.op_log("DEBUG " + customLog(...args), true)
    },
    info: function (...args) {
        Deno.core.ops.op_log("INFO " + customLog(...args), true)
    }
}

// timeout handler
globalThis.setImmediate = (fn) => Promise.resolve().then(fn)

globalThis.require = require;
globalThis.console = console;
globalThis._internal_console = _internal_console;
globalThis.DEBUG = false

const fetchModule = require('fetch');
globalThis.fetch = fetchModule.fetch;
globalThis.Headers = fetchModule.Headers;
globalThis.Response = fetchModule.Response;
globalThis.WebSocket = require('ws').WebSocket;


globalThis.UnityOpsApi = undefined
globalThis.global = globalThis

var nowOffset = Date.now();
globalThis.performance = {
    now: () => Date.now() - nowOffset
}

Deno.core.setUnhandledPromiseRejectionHandler((promise, reason) => {
    console.error('Unhandled promise: ', reason)
    return false
})


Deno.core.setHandledPromiseRejectionHandler((promise, reason) => {
    console.error('Handled promise: ', reason)
    return false
})

Deno.core.ops.op_set_handled_promise_rejection_handler((type, promise, reason) => {
    console.error('Unhandled promise: ', reason)
    Deno.core.ops.op_promise_reject();
})
