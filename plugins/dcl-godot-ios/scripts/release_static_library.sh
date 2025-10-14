#!/bin/bash

GODOT_PLUGINS="dcl_godot_ios"

# Compile Plugin
for lib in $GODOT_PLUGINS; do
    ./scripts/generate_static_library.sh $lib release
    ./scripts/generate_static_library.sh $lib release_debug
    mv ./bin/${lib}.release_debug.a ./bin/${lib}.debug.a
done

# Move to release folder

rm -rf ./bin/release
mkdir ./bin/release

# Move Plugin
for lib in $GODOT_PLUGINS; do
    mkdir ./bin/release/${lib}
    mv ./bin/${lib}.{release,debug}.a ./bin/release/${lib}
done