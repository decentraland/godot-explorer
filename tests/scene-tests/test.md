```typescript
// Scene Code
// 1) Compiled => file.**js**

// Explorer
// 2) Evaluate `file.js`
// 3) read object returned => module.exports = obj
// obj = { onStart, onUpdate } // => it's exposed by `@dcl/sdk`

// @dcl/sdk
// read object exported by src/index.ts
//      module.exports = obj
// get `obj.main`

/**
 * engine.addSystem(() => {
 *      obj.main()
 *      engine.removeSystem('main-function')
 * }, 'main-function')
 */

// 3.1) read main.crdt and evaluated
// 4) await obj.onStart()
// 5) while(true) { await obj.onUpdate() }
//                      ^ in the middle of here, the `main` function is executed

function asd() {}
```
