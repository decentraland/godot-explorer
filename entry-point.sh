#!/bin/sh

/usr/bin/Xvfb -ac :99 -screen 0 1280x1024x24 &
export DISPLAY=:99
./decentraland.godot.client.x86_64 --rendering-driver opengl3 --avatar-renderer --avatars avatars.json
