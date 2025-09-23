# DCL Godot iOS Plugins

This repository is a fork of the [Godot iOS Plugins SDK Integrations](https://github.com/godot-sdk-integrations/godot-ios-plugins). It provides a collection of plugins designed for use with the Godot Engine on iOS.

## Available Plugins

The plugins are located in the `plugins/` directory. To add a new plugin, simply create a new folder within this directory.

## Building the Plugins

To build the plugins, execute the following commands:

```sh
./scripts/generate_headers.sh || true
./scripts/release_xcframework.sh
```

The generated output will be available in the `bin/release/{plugin}` directory.

To import a plugin into your project, use the following command:

```sh
cp -r ./bin/release/{plugin} {project_root}/ios/plugins/
```

## Documentation

Each plugin includes a `README.md` file containing detailed documentation and usage examples. Please refer to these files for specific information about each plugin.

