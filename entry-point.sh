#!/bin/sh

/usr/bin/Xvfb -ac :99 -screen 0 1280x1024x24 &
export DISPLAY=:99

# Check PRESET_ARGS environment variable
if [ -z "$PRESET_ARGS" ]; then
  echo "PRESET_ARGS is not set. Using default arguments."
  PRESET_ARGS="--avatar-renderer --avatars avatars.json"
else
  echo "PRESET_ARGS is set to '$PRESET_ARGS'."
fi

./decentraland.godot.client.x86_64 --rendering-driver opengl3 $PRESET_ARGS || true