# Decentraland Alternative Explorers - Unit Tests

## Introduction:

This tool under construction will be used to test the different components of the SDK7.

The main objective of the tests is to ensure that the different clients under development (Godot and Bevy) work in accordance with the foundation client.
As a consequence we will also collect and report strange behaviors that we observe in the foundation client.

## How choice the component tests:

To run the test you must import the one you want to run in the file `src/index.ts` for example:

```Typescript

export * from ``dcl/sdk``.
import { setupUi } from '../tests/camera-mode/ui'
import '../tests/camera-mode/index.test'

export function main() {
    setupUi()
}
```

## How to start:

You can use the command line. Inside this scene root directory run:

```
npm run start
```
