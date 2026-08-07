Research written to:

`/Users/lordmanuel/Projects/decentraland/godot-explorer/fix-arweave-images-mobile-1766/.pi-subagents/artifacts/outputs/3306b732/parallel-0/0-researcher/research.md`

Progress updated.

Key takeaways:
- `Image.compress(ETC2)` available in export templates if `etcpak` module built; registration not `TOOLS_ENABLED` gated. If missing, `PortableCompressedTexture2D.create_from_image(image, ETC2)` stores uncompressed bytes tagged as ETC2 → GPU misdecodes → white/garbage.
- `reqwest` 0.11 `rustls-tls` uses `rustls-native-certs`; empty/incomplete root store on Android/iOS → HTTPS fails. Switch to `rustls-tls-webpki-roots` fixes mobile TLS.

Note: web search unavailable; findings rely on repo inspection + training memory, with uncertainty flagged. Parent should verify upstream Godot issue numbers and custom fork module config.