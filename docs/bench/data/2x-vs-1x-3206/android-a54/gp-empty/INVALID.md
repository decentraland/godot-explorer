INVALID — do not use these numbers.

Both builds answer a transport loss with a blocking "Connection lost / EXIT / RETRY" modal
(`screen.png` in every run dir), which is one of the invalidation criteria of the protocol
(`docs/bench/2x-vs-1x-3206-protocol.md` §8 rule 2). The windows therefore measure a dialog over a
frozen world, not a playable empty plaza.

Tried: disable Wi-Fi right after `[RenderReady]`, settle 20 s, 60 s window
(`NET_OFF_AFTER_READY=1 scripts/bench/2x-vs-1x/run_scenario.sh gp-empty "" 3 60`).
Kept only as evidence of the failed method: on this build pair there is no client-side switch that
drops remote avatars while keeping the session alive (2.x's React Native shell does not forward
`?pulse=false&livekit=false` to the engine, see the report), so an avatar-free Genesis Plaza needs a
populated-vs-unpopulated comparison in the same live session instead.
