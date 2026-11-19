import { EngineInfo, engine } from '@dcl/sdk/ecs'
import { assertEquals } from 'testing-library/src/testing/assert'
import { test } from 'testing-library/src/testing'

test('engine-info: testing engine information (tickNumber and quantity of properties)', async function (context) {
  const firstEngineInfo = EngineInfo.getOrNull(engine.RootEntity)
  console.log(EngineInfo.get(engine.RootEntity))
  await context.helpers.waitNTicks(5)

  console.log(EngineInfo.get(engine.RootEntity))
  const secondEngineInfo = EngineInfo.getOrNull(engine.RootEntity)
  if (firstEngineInfo != null && secondEngineInfo != null) {
    assertEquals(
      secondEngineInfo.tickNumber,
      firstEngineInfo.tickNumber + 5,
      `ticknumber should be  ${firstEngineInfo.tickNumber + 5}`
    )
    const keysArray: string[] = Object.keys(secondEngineInfo)
    const count: number = keysArray.length
    // frameNumber, totalRuntime, tickNumber, sceneHidden (PBEngineInfo)
    assertEquals(count, 4, `engineInfo should have 4 keys but it have ${count}`)
  }
})
