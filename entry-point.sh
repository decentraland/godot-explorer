#!/bin/sh

# Start virtual display
/usr/bin/Xvfb -ac :99 -screen 0 1280x1024x24 &
export DISPLAY=:99

# Asset Server Mode
# Usage: docker run -p 8080:8080 -e ASSET_SERVER=true [-e ASSET_SERVER_PORT=8080] image
if [ "$ASSET_SERVER" = "true" ]; then
  PORT="${ASSET_SERVER_PORT:-8080}"
  echo "Starting Asset Optimization Server on port $PORT..."
  exec ./decentraland.godot.client.x86_64 --headless --asset-server --asset-server-port "$PORT"
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
