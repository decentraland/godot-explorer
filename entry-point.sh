#!/bin/sh

# Start virtual display (kept for the non-asset-server presets that still
# render a viewport, e.g. avatar-renderer / scene-renderer).
/usr/bin/Xvfb -ac :99 -screen 0 1280x1024x24 &
export DISPLAY=:99

# Asset Server Mode
# Usage: docker run -p 8080:8080 -e ASSET_SERVER=true [-e ASSET_SERVER_PORT=8080] image
#
# `--headless` is fine here now: the asset-preprocessor module that used
# the SubViewport / opengl3 rasterizer (octahedral impostors, BoxOccluder
# auto-spawn) was removed (godot-explorer commits 16da5eec / 70047448).
# The remaining bake pipeline (texture compress, LOD chain, splitter,
# material flips) runs entirely on CPU — the dummy RenderingServer it
# pulls in saves ~3x wall time vs the Mesa llvmpipe + Xvfb path.
if [ "$ASSET_SERVER" = "true" ]; then
  PORT="${ASSET_SERVER_PORT:-8080}"
  echo "[entry] Starting Asset Optimization Server on port $PORT..."
  exec ./decentraland.godot.client.x86_64 --headless --asset-server --asset-server-port "$PORT"
fi

# Default Mode (Avatar Renderer, etc.)
# Usage: docker run -e PRESET_ARGS="--avatar-renderer --avatars avatars.json" image
if [ -z "$PRESET_ARGS" ]; then
  echo "[entry] PRESET_ARGS is not set. Using default arguments."
  PRESET_ARGS="--avatar-renderer --avatars avatars.json"
else
  echo "[entry] PRESET_ARGS is set to '$PRESET_ARGS'."
fi

./decentraland.godot.client.x86_64 --rendering-driver opengl3 $PRESET_ARGS || true
