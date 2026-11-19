import { engine } from '@dcl/ecs'
import { createTestRuntime } from './runtime'
import { TestDefinitionFunction, TestingModule } from './types'

declare let require: any

/**
 * In development builds, this function serves as test runner for automated test scenarios
 * if the runtime accepts the `~system/Testing` module
 * @public
 */
/* @__PURE__ */
const runtime = DEBUG ? /* @__PURE__ */ createRuntime() : undefined

export const test: TestDefinitionFunction = runtime?.test ?? (() => {})

/**
 * A test that needs a renderer, such as a screenshot: it runs when Godot draws and is left out of
 * headless runs, so a scene keeps its state checks and its pictures together.
 */
export const renderTest: TestDefinitionFunction = runtime?.renderTest ?? (() => {})

/**
 * A test switched off: a known bug (its comment names the issue) or a scene that cannot run yet.
 * It never runs, and each run logs it as skipped so it is not forgotten.
 */
export const skipTest: TestDefinitionFunction = runtime?.skipTest ?? (() => {})

function createRuntime() {
  let testingModule: TestingModule
  try {
    testingModule = /* @__PURE__ */ require('~system/Testing')
  } catch (err) {
    console.error(err)

    console.error(
      `🔴🚨‼️ WARNING: The test runner is not available. The test runner will be mocked. ‼️🚨🔴`
    )

    testingModule = {
      async logTestResult(data) {
        console.log(`🧪 mocked '~system/Testing'.logResult`, data)
        return {}
      },
      async plan(data) {
        console.log(`🧪 mocked '~system/Testing'.plan`, data)
        return {}
      },
      async setCameraTransform(transform) {
        console.log(`🧪 mocked '~system/Testing'.setCameraTransform`, transform)
        return {}
      }
    }
  }

  return createTestRuntime(testingModule, engine)
}
