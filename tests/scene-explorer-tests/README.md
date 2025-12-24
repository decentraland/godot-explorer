# Decentraland Alternative Explorers - Unit Tests

## Introduction:

This tool under construction will be used to test the different components of the SDK7.

The main objective of the tests is to ensure that the different clients under development (Godot and Bevy) work in accordance with the foundation client.
As a consequence we will also collect and report unexpected behaviors that we observe in the foundation client.

## How to clean directory:

```
git clean -xdf
```

## Build and run:

### For all components:

In root folder:

```
npm run build
npm start
```

note: this way doesn't have hot reload

### For a specific component:

```
cd component-test-folder-scene
npm run build
npm start
```

note: this way does have reload

## To test in Godot Explorer:

### For all componentes:

```
cargo run -- run -- --rendering-driver opengl3 --scene-test "[[52,-52],[52,-54],[52,-56],[52,-58],[52,-60],[52,-62],[52,-64],[52,-66],[52,-68],[54,-52],[54,-54],[54,-56],[54,-58],[54,-60],[54,-64],[54,-66]]" --realm "http://localhost:8000"
```


### For specific components:

```
cargo run -- run -- --rendering-driver opengl3 --scene-test "[[coord.x,coord.y]]" --realm "http://localhost:8000"
```

where coord.x and coord.y are the scene coordinates
