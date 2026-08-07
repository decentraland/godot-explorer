# Scout: Arweave NFT image white on mobile

## 1. Platform-specific branches in texture/image/http/download paths

### `lib/src/content/texture.rs`
- **Lines 151, 223, 384-385** — `std::env::consts::OS == "ios" || std::env::consts::OS == "android"` branches force ETC2 compression for GIF/WebP frames and static images on mobile.
- **Line 384-415** — Static images go through `create_compressed_texture(&mut image, max_size)` on mobile vs `resize_image` + `ImageTexture::create_from_image` on desktop.
- **Lines 421-462** — `create_compressed_texture()` pads to multiple-of-4, compresses ETC2, wraps in `PortableCompressedTexture2D` with `set_keep_compressed_buffer(true)`. Falls back to uncompressed on compression failure.
- **Line 449-451** — If `pct2.get_width() == 0 || pct2.get_height() == 0` returns `create_placeholder_texture()` (2x2 magenta), not white. White frame is unlikely to be this fallback.

### `lib/src/content/gltf/common.rs`
- **Line 42** — `should_compress = force_compress || std::env::consts::OS == "ios" || std::env::consts::OS == "android";` same ETC2 path for GLTF textures.

### `lib/src/godot_classes/dcl_global.rs`
- **Lines 389-390** — `is_android` / `is_ios` set from `std::env::consts::OS`.
- **Lines 388** — `is_mobile` from `Os::singleton().has_feature("mobile") || force_mobile`.
- **Lines 481-497** — `is_android`, `is_ios`, `is_android_or_emulating`, `is_ios_or_emulating` exposed to GDScript.
- **Lines 276-282** — `#[cfg(target_os = ...)]` logger init branches.

### `lib/src/godot_classes/dcl_ios_plugin.rs` / `dcl_android_plugin.rs`
- Mobile plugin wrappers, not directly in image download path but used for `is_available()` checks elsewhere.

### `godot/src/decentraland_components/`
- `nft_shape.gd` — no platform branches; uses `Global.content_provider.fetch_texture_by_url`.
- `av_player.gd`, `exo_player.gd`, `video_player.gd` — platform branches for iOS/Android video playback (not NFT images).
- `gltf_load_timeout_coalescer.gd` — Android-specific work-around comment.

### `godot/src/logic/`
- No `android`/`ios`/`has_feature` branches found in `godot/src/logic/`.

## 2. `DclUrls.open_sea_proxy()`

- `lib/src/urls/mod.rs:245` — `open_sea_proxy()` returns `format!("https://opensea.decentraland.{}", default_suffix())`.
- `lib/src/godot_classes/dcl_urls.rs:161-162` — static GDScript binding calls `urls::open_sea_proxy()`.
- **No platform difference** — only environment (`org`/`zone`/`today`) via `default_suffix()`. On production default env it resolves to `https://opensea.decentraland.org`.
- The OpenSea proxy then serves the NFT JSON including `image_url`, which for Arweave assets is an `arweave.net/...` URL passed to `Global.content_provider.fetch_texture_by_url`.

## 3. NftShape loading material

- `godot/src/decentraland_components/nft_frame_style_loader.gd:9` — `loading_material = load("res://assets/nftshape/material_loading_animation.tres")`.
- `assets/nftshape/material_loading_animation.tres` — `ShaderMaterial` using `standard_material_animated.tres` shader + `MulticolorDotsLoading.png` sprite sheet.
- Shader (`standard_material_animated.tres`) sets `ALBEDO = albedo.rgb * albedo_tex.rgb` with `albedo = Color(1,1,1,1)`. The loading look is the multicolor dots animation, **not a plain white/unshaded material**.
- `godot/src/decentraland_components/nft_shape.gd:142-147` — `_set_loading_material()` applies this loading material to the `PictureFrame` surface before the real texture arrives.
- `nft_shape.gd:120-143` — once the image promise resolves, `_set_picture_frame_texture()` sets `material.albedo_texture = texture`. If `texture` is `null`, it still sets `tex_width/tex_height = 256` and scales accordingly, but the material would show whatever albedo/texture is present (white-ish if texture missing).

## 4. `Global.http_requester`

- `Global.http_requester` is a Rust autoload field: `lib/src/godot_classes/dcl_global.rs:182` — `pub http_requester: Gd<RustHttpQueueRequester>`, instantiated at line 414.
- `lib/src/http_request/rust_http_queue_requester.rs` — thin GDExtension wrapper around `HttpQueueRequester`.
- `lib/src/http_request/http_queue_requester.rs` — uses `reqwest::Client` directly (not Godot `HTTPRequest`), queues up to 10 parallel requests, default 60 s timeout.
- `lib/Cargo.toml:27` — `reqwest = { ..., features = ["json", "rustls-tls", "blocking", "stream"] }`. TLS is rustls, no mobile certificate special-casing.
- **No mobile-specific handling** in requester. No known TLS override. Redirects handled by reqwest default.

## 5. Git history (verbatim)

```
$ git log --oneline -15 -- lib/src/content/texture.rs
09240baf3 fix(ci): clippy clean — useless_vec in pct2 selfcheck + unused ArrayMesh
516fdb75c fix(review): PCT2 self-test at first use + tween last_emitted_state reset
fb979dd1c fix(textures): switch baked source textures from ImageTexture to PCT2
e67c78e69 feat(content): ETC2 texture pipeline (ImageTexture) end-to-end + main-thread resource-pack load
7fce8cdc6 Revert "feat: bake textures at multiple resolutions for runtime quality selec…" (#2239)
96cab5c9d feat: bake textures at multiple resolutions for runtime quality selection (#2228)
61e7a2081 perf(textures): pixel-budget resize with hard 4K dimension cap (#2078)
22197d859 refactor(content): quality-parameterized texture fetch API + AsyncImage fallback detection (#2077)
583274466 fix: plane mesh UV coordinate conversion and unlit alpha texture (#1471)
012b1d2f5 refactor: content provider, add tracking for all assets, improve loading screen and loading progress bar and import fixes on original assets (emotes mainly) (#995)
f97a05ca9 bump gdext to v0.4.5 (#1032)
fb70332ec refactor: bump `gdext` to previous version of setting MSRV 1.78 (#486)
f96488b60 fix: crashes in empty data textures and infinite rotations (#474)
e6634db40 revert: godot 4.3 and gdext 0.1.3 (#468)
1b9191b8 chore: bump to godot 4.3 and gdext 0.1.3 (#450)
```

```
$ git log --oneline --grep=ETC2 -10
6341f7aa3 Merge pull request #2191 from decentraland/feat/octahedral-impostors-v2
516fdb75c fix(review): PCT2 self-test at first use + tween last_emitted_state reset
fb979dd1c fix(textures): switch baked source textures from ImageTexture to PCT2
a3dfc6142 fix(impostors): restore PCT2 atlas — ImageTexture corrupts ETC2 on llvmpipe
e67c78e69 feat(content): ETC2 texture pipeline (ImageTexture) end-to-end + main-thread resource-pack load
96cab5c9d feat: bake textures at multiple resolutions for runtime quality selection (#2228)
583274466 fix: plane mesh UV coordinate conversion and unlit alpha texture (#1471)
01c2908fc feat: asset optimization server with ZIP packing (#1285)
012b1d2f5 refactor: content provider, add tracking for all assets, improve loading screen and loading progress bar and import fixes on original assets (emotes mainly) (#995)
5d7ec6c01 feat: ios texture compression to ETC2 (#431)
```

```
$ git log --oneline --grep=mobile -i -10
6341f7aa3 Merge pull request #2191 from decentraland/feat/octahedral-impostors-v2
e1c68b03c fix(avatar): render nameplates in a 2D layer, not per-avatar viewports (#2215) (#2224)
5d6082576 chore: point iOS deploy pipeline to -2 repo via DEPLOY_REPO secret (#2288)
6d3f5fa4e feat(impostors): octahedral impostors + packed normal atlas + cap High shadow_quality (Mali)
4014ebf8b feat(bench): Genesis Plaza profiling harness + diagnostic knobs (#1862)
1c282f33d feat(content): optimized-content-base-url override (CLI + deeplink)
599f9503a feat: add Hide Scene Interface toggle and improve mobile controls during hide UI (#2253)
e36313179 fix: scene UI no longer blocks app UI input on mobile (#2247)
820f0b792 fix: scene UI no longer blocks app UI input on mobile (#2247)
96cab5c9d feat: bake textures at multiple resolutions for runtime quality selection (#2228)
```

## 6. Export / rendering / texture compression settings

### `godot/project.godot`
- `renderer/rendering_method="mobile"` — mobile forward+ renderer.
- `rendering_device/driver.ios="vulkan"`, `rendering_device/driver.macos="vulkan"`. `driver.android` not set; Godot 4 default on Android is Vulkan.
- `textures/vram_compression/import_etc2_astc=true` — enables ETC2/ASTC import.
- `[importer_defaults]` `texture={&"compress/mode": 2}` — default texture import is VRAM compressed.

### `godot/export_presets.cfg`
- Android preset (`[preset.0]`) and iOS preset (`[preset.1]`) have **no `texture_format/...` overrides**.
- Only Linux preset (`[preset.2.options]`) sets:
  - `texture_format/s3tc_bptc=true`
  - `texture_format/etc2_astc=false`
  - `texture_format/bptc=true`, `texture_format/s3tc=true`, `texture_format/etc=false`, `texture_format/etc2=false`

## Key observations / hypotheses

1. The only mobile-only code in the NFT image path is the ETC2/PCT2 compression in `texture.rs`. Desktop uses `ImageTexture`; mobile uses `PortableCompressedTexture2D`.
2. The loading material is animated multicolor dots, not white. If the final frame is white, the issue is likely in the texture object set after download, not the loading material.
3. `create_compressed_texture` returns a 2x2 **magenta** placeholder on PCT2 creation failure, not white. A white frame is more consistent with a successfully created but blank PCT2/ETC2 texture, or with the material getting a null/blank texture object.
4. `fetch_texture_by_url` caches by `{hash}_q{N}`. If a previous failed/null result is cached, the frame could stay white. There is no observed per-platform cache invalidation.
5. HTTP requester is Rust reqwest/rustls; no mobile-specific TLS or redirect handling. Arweave URLs are HTTP(S) direct fetches; failures should surface as promise errors and show loading/fallback, not white.
6. Recent ETC2/PCT2 changes (`e67c78e69`, `fb979dd1c`, `516fdb75c`, `09240baf3`) are prime suspects for a mobile-only GPU blank texture.

## Start here

Open `lib/src/content/texture.rs` lines 384-462. Verify whether `create_compressed_texture` produces valid ETC2/PCT2 data for all incoming Arweave image dimensions and formats on actual mobile devices, and whether `infer_mime` correctly identifies Arweave-served image bytes when served through the OpenSea proxy redirect.
