#!/usr/bin/env node
// MCP server for the unified Decentraland debug channel.
//
// Wraps the desktop debug-hub's consumer port (ws://127.0.0.1:9230) and exposes
// it as native MCP tools: run GDScript against the live client (`dcl_eval`),
// query the scene/entity/avatar trees (`dcl_query`), and tail logs (`dcl_logs`).
// Speaks the scene-inspector CMD protocol — the same contract everything else
// uses. Zero deps: Node 22+ has a global `WebSocket`; MCP stdio is hand-rolled
// newline-delimited JSON-RPC 2.0.
//
// Prereq: `cargo run -- debug-hub` running and a client connected to it.
// Config: DCL_HUB_URL (default ws://127.0.0.1:9230).

import { createInterface } from "node:readline";

const HUB = process.env.DCL_HUB_URL || "ws://127.0.0.1:9230";

// --- hub RPC over the scene-inspector CMD protocol -------------------------

function newId() {
  return "mcp-" + Date.now().toString(36) + "-" + Math.floor(Math.random() * 1e6).toString(36);
}

function rpc(cmd, args = {}, timeoutMs = 8000) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(HUB);
    const id = newId();
    const timer = setTimeout(() => {
      try { ws.close(); } catch {}
      reject(new Error(`timeout: no ACK for '${cmd}' (is a client connected to the hub and in-world?)`));
    }, timeoutMs);
    ws.onopen = () => ws.send(JSON.stringify({ type: "SCENE_INSPECTOR_CMD", cmd, args, id }));
    ws.onmessage = (m) => {
      let o;
      try { o = JSON.parse(typeof m.data === "string" ? m.data : m.data.toString()); } catch { return; }
      if (o.type === "SCENE_INSPECTOR_CMD_ACK" && o.id === id) {
        clearTimeout(timer);
        try { ws.close(); } catch {}
        if (o.ok) resolve(o.data);
        else reject(new Error(o.error || `command '${cmd}' failed`));
      }
    };
    ws.onerror = () => {
      clearTimeout(timer);
      reject(new Error(`cannot reach hub at ${HUB} — run \`cargo run -- debug-hub\``));
    };
  });
}

function collectLogs(streams, seconds) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(HUB);
    const out = [];
    const wanted = new Set(streams);
    const timer = setTimeout(() => {
      try { ws.close(); } catch {}
      resolve(out);
    }, Math.max(0.2, seconds) * 1000);
    ws.onopen = () =>
      ws.send(JSON.stringify({ type: "SCENE_INSPECTOR_CMD", cmd: "subscribe", args: { streams }, id: newId() }));
    ws.onmessage = (m) => {
      let o;
      try { o = JSON.parse(typeof m.data === "string" ? m.data : m.data.toString()); } catch { return; }
      if (o.type === "SCENE_INSPECTOR" && Array.isArray(o.payload?.entries)) {
        for (const e of o.payload.entries) if (wanted.has(e.type)) out.push(e);
      }
    };
    ws.onerror = () => {
      clearTimeout(timer);
      reject(new Error(`cannot reach hub at ${HUB} — run \`cargo run -- debug-hub\``));
    };
  });
}

// --- tools -----------------------------------------------------------------

const TOOLS = [
  {
    name: "dcl_eval",
    description:
      "Run a GDScript snippet against the live Decentraland client and return the result. " +
      "Locals in scope: `tree` (SceneTree), `global` (Global autoload), `server`. Use `return X`. " +
      "Synchronous only (no await). Disabled on production builds.",
    inputSchema: {
      type: "object",
      properties: { code: { type: "string", description: "GDScript expression or statement body" } },
      required: ["code"],
    },
  },
  {
    name: "dcl_query",
    description:
      "Query the live client's read-only trees. cmd ∈ ping | scenes | scene | entity | ui_scene | " +
      "ui_entity | avatars | avatar | app_ui | focus. Pass cmd-specific params in `args` " +
      "(e.g. {scene_id, entity_id, filters} or {by:'local'}). Filters support component/property_is/" +
      "collect_nodes/include_children/limit. See the debug-ws-inspector skill.",
    inputSchema: {
      type: "object",
      properties: {
        cmd: { type: "string", description: "ping|scenes|scene|entity|ui_scene|ui_entity|avatars|avatar|app_ui|focus" },
        args: { type: "object", description: "command-specific arguments" },
      },
      required: ["cmd"],
    },
  },
  {
    name: "dcl_logs",
    description:
      "Subscribe to and collect log/network/crdt/perf entries from the live client for a few seconds, " +
      "then return them. Capture is opt-in + connection-gated, so this both subscribes and tails.",
    inputSchema: {
      type: "object",
      properties: {
        streams: { type: "array", items: { type: "string" }, description: "log | network | crdt | perf (default: ['log'])" },
        seconds: { type: "number", description: "how long to collect (default 3)" },
      },
    },
  },
];

async function callTool(name, a) {
  if (name === "dcl_eval") return rpc("eval", { code: String(a.code ?? "") });
  if (name === "dcl_query") return rpc(String(a.cmd ?? ""), a.args || {});
  if (name === "dcl_logs") return collectLogs(a.streams?.length ? a.streams : ["log"], a.seconds ?? 3);
  throw new Error(`unknown tool: ${name}`);
}

// --- MCP stdio (newline-delimited JSON-RPC 2.0) ----------------------------

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

const rl = createInterface({ input: process.stdin });
rl.on("line", async (line) => {
  line = line.trim();
  if (!line) return;
  let req;
  try { req = JSON.parse(line); } catch { return; }
  const { id, method, params } = req;

  // Notifications carry no id and want no response.
  if (id === undefined || id === null) return;

  try {
    if (method === "initialize") {
      send({
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion: params?.protocolVersion || "2024-11-05",
          capabilities: { tools: {} },
          serverInfo: { name: "dcl-debug", version: "0.1.0" },
        },
      });
    } else if (method === "tools/list") {
      send({ jsonrpc: "2.0", id, result: { tools: TOOLS } });
    } else if (method === "tools/call") {
      const { name, arguments: a } = params || {};
      try {
        const data = await callTool(name, a || {});
        const text = typeof data === "string" ? data : JSON.stringify(data, null, 2);
        send({ jsonrpc: "2.0", id, result: { content: [{ type: "text", text }] } });
      } catch (e) {
        send({ jsonrpc: "2.0", id, result: { content: [{ type: "text", text: `Error: ${e.message || e}` }], isError: true } });
      }
    } else if (method === "ping") {
      send({ jsonrpc: "2.0", id, result: {} });
    } else {
      send({ jsonrpc: "2.0", id, error: { code: -32601, message: `Method not found: ${method}` } });
    }
  } catch (e) {
    send({ jsonrpc: "2.0", id, error: { code: -32000, message: String(e.message || e) } });
  }
});
