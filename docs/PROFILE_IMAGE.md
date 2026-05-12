# Profile Image (Avatar Snapshot) Docker Image

The `godot-explorer:profile-snapshot` image runs the Godot client in
`--avatar-renderer` mode to produce body and face PNG snapshots for one or
more wallet addresses.

The image is **not** published to a registry — it's built locally from a
freshly exported Linux client by `scripts/docker_profile_snapshot.sh`, which
also wraps the end-to-end run.

## TL;DR

```bash
# 1. Build the image and snapshot one address against mainnet (.org)
scripts/docker_profile_snapshot.sh 0x4274C2545F2263f820F4E5Dc19CcA999C955238c

# 2. Same, but against the testnet catalyst (.zone)
scripts/docker_profile_snapshot.sh --dclenv zone \
  0x4274C2545F2263f820F4E5Dc19CcA999C955238c
# --dclenv zone switches the renderer's wearable lookups to .zone AND
# bumps the script's --catalyst/--content defaults to peer.decentraland.zone.
```

Outputs land in `./avatars-output/<address>.png` and
`./avatars-output/<address>_face.png`.

## What the helper script does

`scripts/docker_profile_snapshot.sh`:

1. Builds the Rust lib (`cargo run -- build -r`) and exports the Linux Godot
   client (`cargo run -- export --target linux`) — skip with `--skip-build`.
2. Stages a Docker build context with the binary, `.pck`, `libdclgodot.so`,
   `libsentry`, `crashpad_handler`, `entry-point.sh`, and the `Dockerfile`.
3. Builds the image (tag: `godot-explorer:profile-snapshot`, override with
   `--image`) — skip with `--skip-image`.
4. For each wallet address:
   - Fetches the profile from `<catalyst>/profiles/<address>`.
   - Extracts `avatars[0].avatar` and writes an `avatars.json` payload using
     the `--content` URL as `baseUrl`.
5. Runs the container with `avatars.json` and the output dir mounted, then
   verifies that each PNG was produced.

### Common flags

| Flag                | Default                                              |
| ------------------- | ---------------------------------------------------- |
| `--dclenv ENV`      | `org` — set to `zone` or `today` for testnets        |
| `--catalyst URL`    | derived from `--dclenv` (defaults to `peer.decentraland.org/lambdas`) |
| `--content URL`     | derived from `--dclenv` (defaults to `peer.decentraland.org/content`) |
| `--output DIR`      | `./avatars-output`                                   |
| `--width N`         | 256 (body)                                           |
| `--height N`        | 512 (body)                                           |
| `--face-width N`    | 256                                                  |
| `--face-height N`   | 256                                                  |
| `--no-face`         | (off — face snapshot enabled by default)             |
| `--skip-build`      | reuse existing exports/                              |
| `--skip-image`      | reuse existing Docker image                          |
| `--debug`           | build Rust lib in dev mode                           |

`scripts/docker_profile_snapshot.sh --help` lists the full set.

## Running the image directly

The script is a convenience wrapper. The image itself accepts:

- A read-only mount at `/app/avatars.json` describing the avatars to render.
- A writable mount at `/app/output` where PNGs are written.
- `PRESET_ARGS` env var, forwarded verbatim to the Godot binary. Default:
  `--avatar-renderer --avatars avatars.json`.

```bash
docker run --rm \
  -v "$PWD/avatars.json:/app/avatars.json:ro" \
  -v "$PWD/avatars-output:/app/output" \
  godot-explorer:profile-snapshot
```

`avatars.json` schema:

```json
{
  "baseUrl": "https://peer.decentraland.org/content",
  "payload": [
    {
      "destPath": "output/0xabc.png",
      "width": 256,
      "height": 512,
      "faceDestPath": "output/0xabc_face.png",
      "faceWidth": 256,
      "faceHeight": 256,
      "avatar": { "...": "from /lambdas/profiles/<address> -> avatars[0].avatar" }
    }
  ]
}
```

`destPath` / `faceDestPath` are interpreted relative to the container's
working dir (`/app`), so prefix them with `output/` to land in the mounted
output volume.

## Catalyst environment (`.org` vs `.zone`)

This is the part that catches people out, so read this section if your
snapshot comes back **bald** or with **wearables missing**.

### What each URL controls

The renderer talks to two distinct services per environment:

- **Content server** (`/content/...`): hosts wearable entities and their
  asset hashes. Wearable lookups POST to `<content>/entities/active`.
- **Lambdas server** (`/lambdas/...`): hosts profiles and aggregates.

There are two production environments:

| Env  | Content                                 | Lambdas                                 | Use for                    |
| ---- | --------------------------------------- | --------------------------------------- | -------------------------- |
| org  | `https://peer.decentraland.org/content` | `https://peer.decentraland.org/lambdas` | mainnet (Ethereum/Polygon) |
| zone | `https://peer.decentraland.zone/content`| `https://peer.decentraland.zone/lambdas`| testnet (Amoy)             |

Wearables live on the catalyst that matches the chain they were minted on.
Mainnet collections are on `.org`; Amoy testnet collections (URN segment
`amoy`, e.g. `urn:decentraland:amoy:collections-v2:0x...:0:1`) are on
`.zone`. Querying the wrong one returns an empty entity, the renderer
logs `WearableLoader: wearable ... is null`, the wearable's representation
fails validation (`invalid wearable ... for body_shape ...`), and the
avatar renders without it.

### Why `--catalyst` / `--content` alone are not enough

The script's `--catalyst` flag only controls which server the **shell
script** queries to fetch the profile JSON via curl. The `--content` flag
only sets the `baseUrl` field in the staged `avatars.json`, which the
renderer uses as `Global.realm.content_base_url` — but that field is
**not** what the wearable fetcher reads.

Internally, every wearable lookup calls
`Global.realm.get_profile_content_url()`, which returns
`urls::peer_content()`. That URL is derived from a global `dclenv` set via
`DclGlobal.set_dcl_environment(...)`. `dclenv` defaults to `org`.

Net effect: pointing `--catalyst` / `--content` at `.zone` fetches a
testnet profile correctly but the renderer still POSTs `entities/active`
to `peer.decentraland.org`, where Amoy wearables don't exist.

### Fix: pass `--dclenv` to the renderer

The Godot client accepts `--dclenv <env>` directly. Both `--dclenv zone`
and `--dclenv=zone` are accepted. It runs at startup, before the first
wearable fetch.

The helper scripts forward this flag for you:

```bash
# Local renderer
scripts/local_profile_snapshot.sh --dclenv zone <address>

# Docker
scripts/docker_profile_snapshot.sh --dclenv zone <address>
```

When you call the binary directly:

```bash
./decentraland.godot.client.x86_64 \
  --rendering-method gl_compatibility --rendering-driver opengl3 \
  --avatar-renderer --avatars avatars.json \
  --dclenv zone
```

For Docker, override `PRESET_ARGS`:

```bash
docker run --rm \
  -v "$PWD/avatars.json:/app/avatars.json:ro" \
  -v "$PWD/avatars-output:/app/output" \
  -e PRESET_ARGS='--avatar-renderer --avatars avatars.json --dclenv zone' \
  godot-explorer:profile-snapshot
```

`--dclenv` accepts the same grammar as the `dclenv` deeplink param —
plain values like `zone` / `today`, or per-service-group overrides like
`auth::zone,org`.

For `today` (internal dev catalyst at `peer-testing.decentraland.org`),
swap `zone` for `today`.

## Output

```
avatars-output/
  0x4274C2545F2263f820F4E5Dc19CcA999C955238c.png         # body, 256x512
  0x4274C2545F2263f820F4E5Dc19CcA999C955238c_face.png    # face, 256x256
```

Background is transparent. The body and face snapshots come from the same
avatar render via two different camera framings.

## Troubleshooting

| Symptom                                                    | Cause / fix                                                                                  |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Avatar is bald or missing accessories                      | Testnet wearables not resolving — pass `--dclenv zone` (see "Catalyst environment").         |
| `WearableLoader: wearable ... is null` in logs             | Same as above. Wearable URN does not exist on the catalyst the renderer queried.             |
| `invalid wearable ... for body_shape ...` in logs          | Same as above (downstream symptom).                                                          |
| `[GLOBAL] Environment set to: zone (source: --dclenv)`     | Sanity-check line printed at startup confirming the flag took effect.                        |
| `image format ETC2_RGBA8 not supported by hardware`        | Cosmetic GLES3 warning when textures are KTX2/ETC2 — image is auto-converted to RGBA8.       |
| `body missing for <addr>` after the run                    | Look at the renderer's stderr for earlier failures (auth, network, malformed avatar JSON).   |
| Renderer can't reach the catalyst                          | The container needs network egress; check the host's Docker network.                         |

## Related

- Local (no-Docker) version of the same flow: `scripts/local_profile_snapshot.sh`.
- Renderer entry point: `godot/src/tool/avatar_renderer/avatar_renderer_standalone.gd`.
- URL resolution: `lib/src/urls/mod.rs`, `lib/src/env/mod.rs`.
