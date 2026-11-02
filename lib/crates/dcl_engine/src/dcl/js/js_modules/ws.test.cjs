// Unit tests for the JS-side WebSocket shim (ws.js).
//
// These cover the parts of the shim that are NOT reachable from the Rust
// `ws_poll` tests: the readyState state machine (the #2430 fix), event
// dispatch through both `on*` handlers and addEventListener, the single-shot
// `fireClose` logic, binary delivery as ArrayBuffer, subprotocol reporting,
// and send()/close() validation.
//
// The shim only talks to the native layer through `Deno.core.ops`, so we stub
// those ops and script the sequence of `op_ws_poll` results the native task
// would produce. No npm dependencies — Node's built-in test runner only.
//
// Run with:  node --test lib/src/dcl/js/js_modules/ws.test.cjs

const test = require("node:test");
const assert = require("node:assert/strict");

// Load the shim once; its methods read `globalThis.Deno` lazily (at call time),
// so each test can install its own scripted ops before constructing a socket.
const { WebSocket } = require("./ws.js");

/**
 * Install stub ops that replay `pollScript` (an array of `op_ws_poll` results)
 * and record everything sent. Returns the recording `log`.
 *
 * A script item of `{ __throw: true }` makes `op_ws_poll` reject. When the
 * script is exhausted, `op_ws_poll` never resolves (an idle-but-open socket).
 */
// Unique id per install so a socket left polling by an earlier test (its
// _poll loop never stops) cannot steal this test's scripted events: the ops
// only serve the current id and hang for any foreign one.
let nextSocketId = 0;

function installOps(pollScript) {
  const id = ++nextSocketId;
  const log = { created: [], sent: [], cleaned: 0 };
  let i = 0;
  globalThis.Deno = {
    core: {
      ops: {
        op_get_realm() {
          return { isPreview: true };
        },
        op_ws_create(url, protocols) {
          log.created.push({ url, protocols });
          return id;
        },
        op_ws_send(sid, data) {
          if (sid === id) {
            log.sent.push(data);
          }
          return Promise.resolve();
        },
        async op_ws_poll(sid) {
          if (sid !== id || i >= pollScript.length) {
            return await new Promise(() => {}); // foreign/exhausted: idle
          }
          const item = pollScript[i++];
          if (item && item.__throw) {
            throw new Error(item.message || "native poll failure");
          }
          return item;
        },
        op_ws_cleanup(_id) {
          log.cleaned += 1;
        },
      },
    },
  };
  return log;
}

/** Resolve once `onclose` fires (or reject on timeout). */
function closed(ws) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("onclose never fired")), 2000);
    ws.onclose = (e) => {
      clearTimeout(timer);
      resolve(e);
    };
  });
}

test("constants use the WHATWG spec values", () => {
  installOps([]);
  assert.equal(WebSocket.CONNECTING, 0);
  assert.equal(WebSocket.OPEN, 1);
  assert.equal(WebSocket.CLOSING, 2);
  assert.equal(WebSocket.CLOSED, 3);
});

// The exact regression guard for issue #2430: after the native "Connected"
// event, readyState MUST report OPEN, otherwise Colyseus's
// `isOpen = readyState === WebSocket.OPEN` stays false and buffers every send.
test("Connected transitions readyState to OPEN and fires onopen once", async () => {
  installOps([{ type: "Connected" }, { type: "Closed", code: 1000, reason: "" }]);
  const ws = new WebSocket("wss://example.org/");

  let openCount = 0;
  let stateInOnOpen;
  ws.onopen = () => {
    openCount += 1;
    stateInOnOpen = ws.readyState;
  };

  await closed(ws);
  assert.equal(openCount, 1, "onopen must fire exactly once");
  assert.equal(stateInOnOpen, WebSocket.OPEN, "readyState must be OPEN inside onopen");
  assert.equal(WebSocket.OPEN, 1, "OPEN must equal the spec value 1");
});

test("Connected reports the negotiated subprotocol", async () => {
  installOps([
    { type: "Connected", protocol: "json" },
    { type: "Closed", code: 1000, reason: "" },
  ]);
  const ws = new WebSocket("wss://example.org/");
  await closed(ws);
  assert.equal(ws.protocol, "json");
});

test("TextData is delivered to onmessage as a string", async () => {
  installOps([
    { type: "Connected" },
    { type: "TextData", data: "hello" },
    { type: "Closed", code: 1000, reason: "" },
  ]);
  const ws = new WebSocket("wss://example.org/");
  const messages = [];
  ws.onmessage = (m) => messages.push(m);

  await closed(ws);
  assert.equal(messages.length, 1);
  assert.equal(messages[0].type, "message");
  assert.equal(messages[0].data, "hello");
});

test("BinaryData is delivered to onmessage as an ArrayBuffer", async () => {
  installOps([
    { type: "Connected" },
    { type: "BinaryData", data: [1, 2, 3, 255] },
    { type: "Closed", code: 1000, reason: "" },
  ]);
  const ws = new WebSocket("wss://example.org/");
  const messages = [];
  ws.onmessage = (m) => messages.push(m);

  await closed(ws);
  assert.equal(messages.length, 1);
  assert.ok(messages[0].data instanceof ArrayBuffer, "binary data must be an ArrayBuffer");
  assert.deepEqual(Array.from(new Uint8Array(messages[0].data)), [1, 2, 3, 255]);
});

test("addEventListener fires alongside the on* handlers", async () => {
  installOps([
    { type: "Connected" },
    { type: "TextData", data: "hi" },
    { type: "Closed", code: 1000, reason: "" },
  ]);
  const ws = new WebSocket("wss://example.org/");

  const seen = { open: 0, message: 0, close: 0 };
  ws.addEventListener("open", () => (seen.open += 1));
  ws.addEventListener("message", () => (seen.message += 1));
  ws.addEventListener("close", () => (seen.close += 1));

  await new Promise((resolve) => {
    ws.onclose = () => resolve();
  });
  assert.deepEqual(seen, { open: 1, message: 1, close: 1 });
});

test("removeEventListener detaches a listener", async () => {
  installOps([
    { type: "Connected" },
    { type: "TextData", data: "hi" },
    { type: "Closed", code: 1000, reason: "" },
  ]);
  const ws = new WebSocket("wss://example.org/");
  let count = 0;
  const listener = () => (count += 1);
  ws.addEventListener("message", listener);
  ws.removeEventListener("message", listener);

  await closed(ws);
  assert.equal(count, 0, "removed listener must not fire");
});

test("clean close maps code/reason/wasClean and reaches CLOSED once", async () => {
  installOps([{ type: "Connected" }, { type: "Closed", code: 1000, reason: "bye" }]);
  const ws = new WebSocket("wss://example.org/");

  let closeCount = 0;
  const e = await new Promise((resolve) => {
    ws.onclose = (ev) => {
      closeCount += 1;
      resolve(ev);
    };
  });
  assert.equal(e.code, 1000);
  assert.equal(e.reason, "bye");
  assert.equal(e.wasClean, true);
  assert.equal(ws.readyState, WebSocket.CLOSED);
  await new Promise((r) => setTimeout(r, 30));
  assert.equal(closeCount, 1, "onclose must fire exactly once");
});

test("abnormal close (no code) defaults to 1006 / wasClean=false", async () => {
  installOps([{ type: "Connected" }, { type: "Closed" }]);
  const ws = new WebSocket("wss://example.org/");
  const e = await closed(ws);
  assert.equal(e.code, 1006);
  assert.equal(e.wasClean, false);
  assert.equal(ws.readyState, WebSocket.CLOSED);
});

test("a native poll error fires onerror then exactly one onclose(1006)", async () => {
  installOps([{ type: "Connected" }, { __throw: true, message: "boom" }]);
  const ws = new WebSocket("wss://example.org/");

  const order = [];
  let errorEvent;
  let closeEvent;
  ws.onerror = (e) => {
    order.push("error");
    errorEvent = e;
  };
  const done = new Promise((resolve) => {
    ws.onclose = (e) => {
      order.push("close");
      closeEvent = e;
      resolve();
    };
  });

  await done;
  await new Promise((r) => setTimeout(r, 30));
  assert.deepEqual(order, ["error", "close"], "error must precede a single close");
  assert.equal(errorEvent.type, "error");
  assert.equal(closeEvent.code, 1006);
  assert.equal(ws.readyState, WebSocket.CLOSED);
});

test("send() coerces string/Uint8Array/ArrayBuffer into the native shapes", async () => {
  const log = installOps([{ type: "Connected" }, { type: "Closed", code: 1000, reason: "" }]);
  const ws = new WebSocket("wss://example.org/");
  ws.onopen = () => {
    ws.send("hi");
    ws.send(new Uint8Array([1, 2, 3]));
    ws.send(new Uint8Array([9, 9]).buffer);
  };

  await closed(ws);
  assert.deepEqual(log.sent, [
    { type: "Text", data: "hi" },
    { type: "Binary", data: [1, 2, 3] },
    { type: "Binary", data: [9, 9] },
  ]);
});

test("send() before OPEN throws InvalidStateError", () => {
  installOps([]);
  const ws = new WebSocket("wss://example.org/");
  assert.equal(ws.readyState, WebSocket.CONNECTING);
  assert.throws(() => ws.send("too early"), /InvalidStateError/);
});

test("close() during CONNECTING is not reverted to OPEN by a later Connected", async () => {
  installOps([{ type: "Connected" }, { type: "Closed", code: 1000, reason: "" }]);
  const ws = new WebSocket("wss://example.org/");
  assert.equal(ws.readyState, WebSocket.CONNECTING);

  let openFired = false;
  ws.onopen = () => {
    openFired = true;
  };
  ws.close(1000);
  assert.equal(ws.readyState, WebSocket.CLOSING);

  await closed(ws);
  assert.equal(openFired, false, "onopen must not fire after close() during CONNECTING");
  assert.equal(ws.readyState, WebSocket.CLOSED);
});

test("a clean close with a non-1000 code reports wasClean=true", async () => {
  installOps([{ type: "Connected" }, { type: "Closed", code: 4001, reason: "kick" }]);
  const ws = new WebSocket("wss://example.org/");
  const e = await closed(ws);
  assert.equal(e.code, 4001);
  assert.equal(e.wasClean, true, "a received close frame (numeric code) is a clean close");
});

test("addEventListener/dispatchEvent ignore inherited type names safely", () => {
  installOps([]);
  const ws = new WebSocket("wss://example.org/");
  assert.doesNotThrow(() => ws.addEventListener("toString", () => {}));
  assert.doesNotThrow(() => ws.dispatchEvent({ type: "__proto__" }));
});

test("close() validates the code and forwards code/reason; is idempotent", async () => {
  const log = installOps([{ type: "Connected" }, { type: "Closed", code: 4001, reason: "kick" }]);
  const ws = new WebSocket("wss://example.org/");

  // Reserved/invalid application codes are rejected.
  assert.throws(() => ws.close(1001), /InvalidAccessError/);

  ws.close(4001, "kick");
  assert.equal(ws.readyState, WebSocket.CLOSING);
  ws.close(4002, "again"); // must be a no-op once CLOSING

  await closed(ws);
  const closes = log.sent.filter((s) => s.type === "Close");
  assert.deepEqual(closes, [{ type: "Close", code: 4001, reason: "kick" }]);
});
