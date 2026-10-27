# Dev intent: telling the human why this client is open

When an agent (pi, Claude, any automation) launches the client **so a human can test
something**, the human is looking at a game window with no idea what was changed or what
"working" looks like. The prompt usually arrives in the chat, and it is gone from sight by
the time the human reaches the bug.

`--dev-intent` / `--dev-intent-file` put that intent inside the client: it is parsed at startup
and echoed on stdout. The on-screen card and the debug-panel tab of the 1.x client are not on 2.x
yet (see "What the human sees").

## Rule for agents

> Whenever you launch the client for a human to test, pass `--dev-intent` or
> `--dev-intent-file`. No exceptions: a client opened without an intent is a client the
> human has to guess about.

With a prompt long enough to need a file:

```bash
godot --path godot --dev-intent-file /tmp/pi-intent.txt "$@"
```

Through the repo's xtask (builds first, then forwards everything after `--` to Godot):

```bash
cargo run -- run -- --dev-intent-file /tmp/pi-intent.txt
```

One-liner, title and steps separated by `|`:

```bash
cargo run -- run -- --dev-intent "Fix #3080 head IK | Open two clients | Look at the other avatar at extreme pitch | Expected: the neck follows"
```

## The intent file

UTF-8. The **first non-empty line is the title**; every line starting with `- ` is an
expected step. Other lines are ignored, so prose can sit next to the checklist.

```
Fix #3080 head IK between two clients
- Open two clients, one with --dev-intent and one plain
- Look at the other avatar at extreme pitch (looking at the floor)
- Expected: the neck follows instead of breaking
```

A missing or unreadable file is not fatal: the client logs a warning and boots with no
intent. When both flags are passed, `--dev-intent-file` wins.

## What the human sees

- **stdout**, where the source file and then one line per step can be grepped from any log:
  ```
  [DEV-INTENT] source: --dev-intent-file /tmp/pi-intent.txt
  [DEV-INTENT] Fix #3080 head IK between two clients
  [DEV-INTENT] 1. Open two clients, one with --dev-intent and one plain
  [DEV-INTENT] 2. Look at the other avatar at extreme pitch (looking at the floor)
  [DEV-INTENT] 3. Expected: the neck follows instead of breaking
  ```
- **Not on 2.x yet**: the 1.x client also draws a card above the whole UI (title, tickable
  steps, `X` to close, `F8` to show/hide) and a "Dev Intent" debug-panel tab. 2.x has no
  product UI on desktop (the shell is React Native), so the card needs a home in the shell
  first. The parsed intent is `DclCli::dev_intent` (title + steps) for whoever builds it.

`--dcl-help` lists both flags under `Debugging`.
