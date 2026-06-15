#!/bin/sh
set -e

# Start virtual display
/usr/bin/Xvfb -ac :99 -screen 0 1280x1024x24 &
export DISPLAY=:99

# Force Mesa to use the software rasterizer (llvmpipe). Without this, Mesa
# tries to talk to a real GPU first — in a container with no /dev/dri device
# that path can silently fall through to a degenerate driver instead of
# llvmpipe. With this set, every GL call goes through the CPU path which is
# what the asset-server bake needs (SubViewport readback for impostors,
# ETC2 compression staying in CPU memory for PCT2 serialization).
export LIBGL_ALWAYS_SOFTWARE=1
# Pin the driver name explicitly. Without it, Mesa tries pci-id detection
# against /dev/dri first; in a sandboxed container without /dev/dri this
# falls through to a no-op driver and Godot's opengl3 init fails, dropping
# the engine into the dummy renderer (texture_2d_get returns null, every
# SubViewport readback comes back blank → impostor bakes all fail with
# `fail_blank_albedo`).
export MESA_LOADER_DRIVER_OVERRIDE=llvmpipe

# Wait for Xvfb to actually accept connections before spawning Godot.
# A blind `sleep 2` raced: under cold-start container load Xvfb sometimes
# took >2s, godot started without a display, fell back to the dummy
# RenderingServer, and every impostor SubViewport readback came back null.
echo "[entry] waiting for Xvfb on :99..."
for i in 1 2 3 4 5 6 7 8 9 10; do
  if xdpyinfo -display :99 >/dev/null 2>&1; then
    echo "[entry] Xvfb ready after ${i} attempts"
    break
  fi
  sleep 0.5
  if [ "$i" = "10" ]; then
    echo "[entry] FATAL: Xvfb never came up on :99"
    exit 1
  fi
done

# Sanity-check that the GL stack actually loads — `glxinfo` will print the
# llvmpipe renderer string when everything is wired up. If it errors, the
# Godot opengl3 init below will silently fall back to the dummy renderer
# and every impostor bake will fail with fail_blank_albedo.
glxinfo -B 2>&1 | grep -E "renderer string|OpenGL version" | head -2 || echo "[entry] WARN: glxinfo failed"

# Asset Server Mode
# Usage: docker run -p 8080:8080 -e ASSET_SERVER=true [-e ASSET_SERVER_PORT=8080] image
if [ "$ASSET_SERVER" = "true" ]; then
  PORT="${ASSET_SERVER_PORT:-8080}"
  echo "[entry] Starting Asset Optimization Server on port $PORT..."
  # NO --headless. The headless display driver wires up a null renderer:
  # `SubViewport.get_texture()` returns the texture without ever rasterizing,
  # so the impostor bake reads back blank pixels (all impostor jobs would
  # report `impostors_baked=0`). Worse, `PortableCompressedTexture2D` ends
  # up serialized without its compressed buffer — every baked texture .res
  # ships as a 220-byte stub and renders magenta on the device.
  exec ./decentraland.godot.client.x86_64 --rendering-driver opengl3 --audio-driver Dummy --asset-server --asset-server-port "$PORT"
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
