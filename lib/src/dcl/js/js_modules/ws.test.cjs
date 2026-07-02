// Unit tests for the JS-side WebSocket shim (ws.js).
//
// These cover the half of the fix for issue #2430 that lives purely in
// JavaScript and is therefore NOT reachable from the Rust `ws_poll` tests:
// the readyState state machine, the on*-handler dispatch, and the
// single-shot `fireClose` logic (code/reason/wasClean mapping).
//
// The shim only talks to the native layer through `Deno.core.ops`, so we stub
// those four ops and script the sequence of `op_ws_poll` results the native
// task would produce. No npm dependencies — Node's built-in test runner only.
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
 * A script item of `{ __throw: true }` makes `op_ws_poll` reject (simulating a
 * native error). When the script is exhausted, `op_ws_poll` never resolves,
 * mirroring an idle-but-open socket.
 */
function installOps(pollScript) {
  const log = { created: [], sent: [], cleaned: 0 };
  let i = 0;
  globalThis.Deno = {
    core: {
      ops: {
        op_ws_create(url, protocols) {
          log.created.push({ url, protocols });
          return 1;
        },
        op_ws_send(_id, data) {
          log.sent.push(data);
          return Promise.resolve();
        },
        async op_ws_poll(_id) {
          if (i >= pollScript.length) {
            return await new Promise(() => {}); // idle: no more native events
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

test("TextData is delivered to onmessage", async () => {
  installOps([
    { type: "Connected" },
    { type: "TextData", data: "hello" },
    { type: "Closed", code: 1000, reason: "" },
  ]);
  const ws = new WebSocket("wss://example.org/");
  const messages = [];
  ws.onmessage = (m) => messages.push(m);

  await closed(ws);
  assert.deepEqual(messages, [{ type: "text", data: "hello" }]);
});

test("BinaryData is delivered to onmessage as a Uint8Array", async () => {
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
  assert.equal(messages[0].type, "binary");
  assert.ok(messages[0].data instanceof Uint8Array);
  assert.deepEqual(Array.from(messages[0].data), [1, 2, 3, 255]);
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
  // Give any stray extra close a chance to (incorrectly) fire.
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
  let closeEvent;
  ws.onerror = () => order.push("error");
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

test("close() is idempotent: sends one Close and transitions to CLOSING", async () => {
  const log = installOps([{ type: "Connected" }, { type: "Closed", code: 1000, reason: "" }]);
  const ws = new WebSocket("wss://example.org/");

  let stateAfterFirstClose;
  ws.onopen = () => {
    ws.close();
    stateAfterFirstClose = ws.readyState;
    ws.close(); // second call must be a no-op
  };

  await closed(ws);
  const closeSends = log.sent.filter((s) => s.type === "Close");
  assert.equal(closeSends.length, 1, "close() must enqueue exactly one Close");
  assert.equal(stateAfterFirstClose, WebSocket.CLOSING);
});
