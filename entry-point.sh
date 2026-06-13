#!/bin/sh

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

# Asset Server Mode
# Usage: docker run -p 8080:8080 -e ASSET_SERVER=true [-e ASSET_SERVER_PORT=8080] image
if [ "$ASSET_SERVER" = "true" ]; then
  PORT="${ASSET_SERVER_PORT:-8080}"
  echo "Starting Asset Optimization Server on port $PORT..."
  # NO --headless. The headless display driver wires up a null renderer:
  # `SubViewport.get_texture()` returns the texture without ever rasterizing,
  # so the impostor bake reads back blank pixels (all impostor jobs would
  # report `impostors_baked=0`). Worse, `PortableCompressedTexture2D` ends
  # up serialized without its compressed buffer — every baked texture .res
  # ships as a 220-byte stub and renders magenta on the device.
  #
  # Default display driver + the Xvfb on :99 above gives an offscreen X11
  # surface; opengl3 + Mesa llvmpipe (LIBGL_ALWAYS_SOFTWARE=1) execute the
  # shaders on CPU, no GPU required. PCT2 keeps the ETC2 bytes through
  # serialization (godot 4.6.2.gh.9ee6af7ab carries the engine-side fix).
  sleep 2
  exec ./decentraland.godot.client.x86_64 --rendering-driver opengl3 --audio-driver Dummy --asset-server --asset-server-port "$PORT"
fi

# Default Mode (Avatar Renderer, etc.)
# Usage: docker run -e PRESET_ARGS="--avatar-renderer --avatars avatars.json" image
if [ -z "$PRESET_ARGS" ]; then
  echo "PRESET_ARGS is not set. Using default arguments."
  PRESET_ARGS="--avatar-renderer --avatars avatars.json"
else
  echo "PRESET_ARGS is set to '$PRESET_ARGS'."
fi

./decentraland.godot.client.x86_64 --rendering-driver opengl3 $PRESET_ARGS || true
