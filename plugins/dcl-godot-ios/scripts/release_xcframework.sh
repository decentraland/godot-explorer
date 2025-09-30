#!/bin/bash
set -e

GODOT_PLUGINS="dcl_godot_ios"

# Compile Plugin
for lib in $GODOT_PLUGINS; do
    echo "# Compile ${lib}"
    ./scripts/generate_xcframework.sh $lib release
    ./scripts/generate_xcframework.sh $lib release_debug
    mv ./bin/${lib}.release_debug.xcframework ./bin/${lib}.debug.xcframework
done

# Move to release folder

rm -rf ./bin/release
mkdir ./bin/release

# Move Plugin
for lib in $GODOT_PLUGINS; do
    echo "# Move ${lib}"
    mkdir ./bin/release/${lib}
    mv ./bin/${lib}.{release,debug}.xcframework ./bin/release/${lib}
    cp ./plugins/${lib}/${lib}.gdip ./bin/release/${lib}

    rsync -av --delete ./bin/release/${lib} ./demo/ios/plugins/
done
