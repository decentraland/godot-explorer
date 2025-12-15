/**
 * This module provides a createTestRuntime function that returns an object with a test function that can be used to define tests.
 */
import { onEnterScene, onLeaveScene } from '@dcl/sdk/observables'
import { IEngine, Transform } from '@dcl/ecs'
import { assertEquals } from './assert'
import type {
  TestingModule,
  TestFunction,
  TestHelpers,
  TestFunctionContext
} from './types'
import { PlayerIdentityData, engine } from '@dcl/sdk/ecs'
import { getUserData } from '~system/UserIdentity'
import { withInterval } from '../utils/helpers'
import { getPlayersInScene } from '~system/Players'

// This function creates a test runtime that can be used to define and run tests.
// It takes a `TestingModule` instance (loaded from require('~system/Testing')) and an `IEngine` instance (from Decentraland's SDK).
// It returns an object with a `test` function that can be used to define tests.
/* @__PURE__ */

export function createTestRuntime(
  testingModule: TestingModule,
  engine: IEngine
) {
  type TestPlanEntry = { name: string; fn: TestFunction }
  type RunnerEnvironment = {
    resolve: () => void
    reject: (error: any) => void
    helpers: TestHelpers
    generator: Generator
  }

  // this flag ensures no tests are added asynchronously
  let runtimeFrozen = false

  let currentFrameCounter = 0
  let currentFrameTime = 0
  let currentTestNumber = 0

  // array to hold the scheduled tests
  const scheduledTests: TestPlanEntry[] = []

  // an array of promises that are resolved on the next frame (after the current frame is finished)
  const nextTickFuture: Array<(dt: number) => void> = []

  // this function returns a promise that resolves on the next frame
  async function nextTick() {
    return new Promise<number>((resolve) => {
      nextTickFuture.push(resolve)
    })
  }

  const playerIsInside = getPlayerIsInside()
  const waitingFn = withInterval(1, () => {
    console.log('Waiting for the player to be inside the scene...')
  })

  // add a system to the engine that resolves all promises in the `nextTickFuture` array
  engine.addSystem(function TestingFrameworkCoroutineRunner(dt) {
    currentFrameCounter++
    currentFrameTime += dt

    if (nextTickFuture.length) {
      // Avoids the test to begin without the player in the scene
      if (!playerIsInside.get()) {
        waitingFn(dt)
        return
      }

      // resolve all nextTick futures.
      nextTickFuture.splice(0, nextTickFuture.length).forEach((_) => _(dt))
    }
  })

  // this function schedules a value to be processed on the next frame, the test runner will
  // continue to run until it reaches a yield point
  function scheduleValue(value: any, env: RunnerEnvironment) {
    if (
      value &&
      typeof value === 'object' &&
      typeof value.then === 'function'
    ) {
      console.log('⏱️ yield promise')
      // if the value is a promise, schedule it to be awaited after the current frame is finished
      nextTickFuture.push(async () => {
        try {
          scheduleValue(await value, env)
        } catch (err) {
          env.reject(err)
        }
      })
    } else if (typeof value === 'function') {
      console.log('⏱️ yield function')
      // if the value is a function, schedule it to be called on the next frame
      nextTickFuture.push(() => {
        scheduleValue(value(), env)
      })
      return
    } else if (typeof value === 'undefined' || value === null) {
      console.log('⏱️ yield')
      // if the value is undefined or null, continue processing the generator the next frame
      nextTickFuture.push(() => {
        consumeGenerator(env)
      })
    } else throw new Error(`Unexpected value from test generator: ${value}`)
  }

  // this function processes a generator function by scheduling its values to be processed on the next frame
  function consumeGenerator(env: RunnerEnvironment) {
    try {
      const ret = env.generator.next()
      if (!ret.done) {
        scheduleValue(ret.value, env)
      } else {
        env.resolve()
      }
    } catch (err) {
      env.reject(err)
    }
  }

  // this function schedules a test run on the next frame
  function scheduleNextRun() {
    if (scheduledTests.length) {
      nextTickFuture.push(runTests)
    }
  }

  // this function runs the scheduled tests
  function runTests() {
    if (scheduledTests.length) {
      const entry = scheduledTests.shift()!
      const initialFrame = currentFrameCounter
      const startTime = currentFrameTime

      let resolved = false

      // this function should be called only once. it makes the current test pass
      const resolve = () => {
        if (resolved) throw new Error('resolved twice')
        resolved = true

        console.log(`🟢 Test passed ${entry.name}`)

        testingModule
          .logTestResult({
            name: entry.name,
            ok: true,
            totalFrames: currentFrameCounter - initialFrame,
            totalTime: currentFrameTime - startTime
          })
          .finally(scheduleNextRun)
      }

      const reject = (err: any) => {
        if (resolved) throw new Error('resolved twice')
        resolved = true

        console.log(`🔴 Test failed ${entry.name}`)
        console.error(err)

        testingModule
          .logTestResult({
            name: entry.name,
            ok: false,
            error: err.toString(),
            stack: err && typeof err === 'object' && err.stack,
            totalFrames: currentFrameCounter - initialFrame,
            totalTime: currentFrameTime - startTime
          })
          .finally(scheduleNextRun)
      }

      try {
        console.log(`🧪 Running test ${entry.name}`)

        const testHelpers: TestHelpers = {
          async waitTicksUntil(
            fn: () => boolean,
            timeoutMs: number = 10000
          ): Promise<boolean> {
            if (timeoutMs < 0) throw new Error(`Timeout must be positive`)
            if (timeoutMs > 10 * 60000)
              throw new Error(`Timeout must be less than 10 minutes`)
            const start = new Date().getTime()
            while (!fn()) {
              if (new Date().getTime() - start > timeoutMs) {
                return false
              }

              await nextTick()
            }
            return true
          },
          async waitNTicks(n: number) {
            for (let i = 0; i < n; i++) await nextTick()
          },
          async setCameraTransform(transform) {
            await testingModule.setCameraTransform(transform)
            await nextTick()

            const TransformComponent = engine.getComponent(
              Transform.componentId
            ) as typeof Transform
            const actualTransform = TransformComponent.get(engine.CameraEntity)

            assertEquals(
              actualTransform.position,
              transform.position,
              "positions don't match"
            )
            assertEquals(
              actualTransform.rotation,
              transform.rotation,
              "rotations don't match"
            )
          }
        }

        currentTestNumber += 1
        const testContext: TestFunctionContext = {
          helpers: testHelpers,
          currentTestNumber
        }

        const returnValue = entry.fn(testContext)

        if (returnValue && typeof returnValue === 'object') {
          if (isGenerator(returnValue)) {
            const env: RunnerEnvironment = {
              generator: returnValue,
              helpers: testHelpers,
              resolve,
              reject
            }
            consumeGenerator(env)
          } else if (isPromise(returnValue)) {
            returnValue.then(resolve).catch(reject)
          } else {
            throw new Error(`Unknown test result type: ${returnValue}`)
          }
        } else {
          resolve()
        }
      } catch (err: any) {
        reject(err)
      }
    }
  }

  // schedule the test runner start for the next frame
  nextTickFuture.push(() => {
    // once we run the next tick, the test runtime becomes frozen. that means no new
    // test definitions are accepted
    runtimeFrozen = true

    if (!scheduledTests.length) return

    // inform the test runner about the plans for this test run
    testingModule
      .plan({ tests: scheduledTests })
      .then(scheduleNextRun)
      .catch(globalFail)
  })

  // this is the function that is used to plan a test functionn
  /* @__PURE__ */
  function test(name: string, fn: TestFunction) {
    if (runtimeFrozen)
      throw new Error("New tests can't be added at this stage.")

    if (scheduledTests.some(($) => $.name === name))
      throw new Error(`Test with name ${name} already exists`)

    scheduledTests.push({ fn, name })
  }

  return {
    test
  }
}

function isGenerator(t: any): t is Generator {
  return t && typeof t === 'object' && typeof t[Symbol.iterator] === 'function'
}

function isPromise(t: any): t is Promise<unknown> {
  return t && typeof t === 'object' && typeof t.then === 'function'
}

function globalFail(error: any) {
  // for now, the failure is only writing to the console.error.
  console.error(error)
}

function getPlayerIsInside() {
  let playerIsInside = false
  let currentUserId: string | undefined = undefined

  getUserData({}).then((player) => {
    if (player) {
      currentUserId = player.data?.userId
    }

    getPlayersInScene({}).then((data) => {
      for (const player of data.players) {
        toggleState(player.userId, true)
      }
    })
  })

  function toggleState(userId: string, value: boolean) {
    const currentPlayerAddress = PlayerIdentityData.getOrNull(
      engine.PlayerEntity
    )?.address
    const primaryPlayerUserId = currentPlayerAddress || currentUserId
    if (primaryPlayerUserId === userId) {
      playerIsInside = value
    }
  }

  onEnterScene.add((player) => {
    toggleState(player.userId, true)
  })

  onLeaveScene.add((player) => {
    toggleState(player.userId, false)
  })

  return {
    get() {
      return playerIsInside
    }
  }
}
