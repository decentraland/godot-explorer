import {
  EngineInfo,
  MeshRenderer,
  Transform,
  VisibilityComponent,
  engine
} from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { test } from 'testing-library/src/testing'

test('visibility: true - if exist a reference snapshot should match with it', async function (context) {
  await context.helpers.waitTicksUntil(() => {
    const tickNumber = EngineInfo.getOrNull(engine.RootEntity)?.tickNumber ?? 0
    if (tickNumber > 100) {
      return true
    } else {
      return false
    }
  }, 10000)

  customAddEntity.clean()
  const cube = customAddEntity.addEntity()
  Transform.create(cube, {
    position: Vector3.create(8, 1, 8)
  })
  MeshRenderer.create(cube, {
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  VisibilityComponent.create(cube, { visible: true })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_visibility_true.png',
    Vector3.create(6, 2, 6),
    Vector3.create(8, 1, 8)
  )
})

test('visibility: false - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const cube = customAddEntity.addEntity()
  Transform.create(cube, {
    position: Vector3.create(8, 1, 8)
  })
  MeshRenderer.create(cube, {
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  VisibilityComponent.create(cube, { visible: false })
  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_visibility_false.png',
    Vector3.create(6, 2, 6),
    Vector3.create(8, 1, 8)
  )
})
