import {
  Billboard,
  BillboardMode,
  CameraModeArea,
  ColliderLayer,
  MeshCollider,
  MeshRenderer,
  Raycast,
  RaycastQueryType,
  RaycastResult,
  Transform,
  engine
} from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { test } from 'testing-library/src/testing'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'
import { waitForMeshColliderApplied } from 'testing-library/src/utils/godot'

// BM_NONE = 0,
// BM_X = 1,
// BM_Y = 2,
// BM_Z = 4,
// BM_ALL = 7

const sceneCenter = Vector3.create(8, 1, 8)

test('billboard: mode BM_NONE', async function (context) {
  customAddEntity.clean()
  const cameraModeAreaE = customAddEntity.addEntity()
  Transform.create(cameraModeAreaE, { position: sceneCenter })
  CameraModeArea.create(cameraModeAreaE, {
    area: Vector3.create(16, 5, 16),
    mode: 0
  })
  const colliderToRaycast = customAddEntity.addEntity()
  Transform.create(colliderToRaycast, {
    parent: engine.CameraEntity,
    position: Vector3.create(0, 0, 0),
    scale: Vector3.create(0.1, 0.1, 0.1)
  })
  MeshCollider.create(colliderToRaycast, {
    collisionMask: ColliderLayer.CL_CUSTOM5,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  await waitForMeshColliderApplied(context)

  const billboardEntity = customAddEntity.addEntity()
  Transform.create(billboardEntity, { position: sceneCenter })
  Billboard.create(billboardEntity, { billboardMode: BillboardMode.BM_NONE })
  MeshRenderer.create(billboardEntity, {
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  // TODO: This tick waits that billboard creating was finished
  await context.helpers.waitNTicks(1)

  await assertMovePlayerTo(context, Vector3.create(4, 0, 4), sceneCenter)

  Raycast.create(billboardEntity, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Backward()
    },
    maxDistance: 30,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: ColliderLayer.CL_CUSTOM5
  })

  // TODO: These ticks wait that raycast creating was finished
  await context.helpers.waitNTicks(3)

  const rayResult = RaycastResult.get(billboardEntity)

  assertEquals(
    rayResult.hits.length,
    0,
    'raycast from entity should not hit the collider '
  )
})

test('billboard: mode BM_Y', async function (context) {
  customAddEntity.clean()

  // The camera mode area is used to fix the camera in the player position
  const cameraModeAreaE = customAddEntity.addEntity()
  Transform.create(cameraModeAreaE, { position: sceneCenter })
  CameraModeArea.create(cameraModeAreaE, {
    area: Vector3.create(16, 5, 16),
    mode: 0
  })
  await assertMovePlayerTo(context, Vector3.create(4, 0, 4), sceneCenter)

  // Animation of the camera mode area takes several ticks
  await context.helpers.waitNTicks(100)

  // Setup billboard
  const colliderToRaycast = customAddEntity.addEntity()
  Transform.create(colliderToRaycast, {
    parent: engine.PlayerEntity,
    position: Vector3.create(0, 1, 0),
    scale: Vector3.create(0.1, 0.1, 0.1)
  })
  MeshCollider.create(colliderToRaycast, {
    collisionMask: ColliderLayer.CL_CUSTOM5,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  await waitForMeshColliderApplied(context)

  const billboardEntity = customAddEntity.addEntity()
  Transform.create(billboardEntity, { position: sceneCenter })
  Billboard.create(billboardEntity, { billboardMode: BillboardMode.BM_Y })
  MeshRenderer.create(billboardEntity, {
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  // TODO: This tick waits that billboard creating was finished
  await context.helpers.waitNTicks(1)

  Raycast.create(billboardEntity, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Backward()
    },
    maxDistance: 30,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: ColliderLayer.CL_CUSTOM5
  })

  // TODO: These ticks wait that raycast creating was finished
  await context.helpers.waitNTicks(3)

  const rayResult = RaycastResult.get(billboardEntity)

  assertEquals(
    rayResult.hits.length,
    1,
    'raycast from entity should hit the collider'
  )
})

test('billboard: mode BM_ALL', async function (context) {
  customAddEntity.clean()

  // Create a step box to place the player on top of it
  const stepBoxSize = 4.0
  const stepBoxPosition = Vector3.create(4, stepBoxSize / 2.0, 4)
  const stepBox = customAddEntity.addEntity()
  Transform.create(stepBox, {
    position: stepBoxPosition,
    scale: Vector3.create(1, stepBoxSize, 1)
  })

  // The camera mode area is used to fix the camera in the player position
  const cameraModeAreaE = customAddEntity.addEntity()
  Transform.create(cameraModeAreaE, { position: sceneCenter })
  CameraModeArea.create(cameraModeAreaE, {
    area: Vector3.create(16, stepBoxSize + 10, 16),
    mode: 0
  })
  MeshCollider.create(stepBox, {
    collisionMask: ColliderLayer.CL_PHYSICS | ColliderLayer.CL_POINTER
  })
  await waitForMeshColliderApplied(context)

  await assertMovePlayerTo(
    context,
    Vector3.add(stepBoxPosition, Vector3.create(0, 3, 0)),
    sceneCenter
  )

  // Animation of the camera mode area takes several ticks
  // Also the gravity to make the player fall
  await context.helpers.waitNTicks(150)

  const colliderToRaycast = customAddEntity.addEntity()
  Transform.create(colliderToRaycast, {
    parent: engine.CameraEntity,
    position: Vector3.create(0, 0, 0),
    scale: Vector3.create(0.1, 0.1, 0.1)
  })
  MeshCollider.create(colliderToRaycast, {
    collisionMask: ColliderLayer.CL_CUSTOM4,
    mesh: {
      $case: 'cylinder',
      cylinder: {
        radiusBottom: 0.7,
        radiusTop: 0.7
      }
    }
  })
  await waitForMeshColliderApplied(context)

  const billboardEntity = customAddEntity.addEntity()
  Transform.create(billboardEntity, { position: sceneCenter })
  Billboard.create(billboardEntity, { billboardMode: BillboardMode.BM_ALL })
  MeshRenderer.create(billboardEntity, {
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  // TODO: This tick waits that billboard creating was finished
  await context.helpers.waitNTicks(1)

  Raycast.create(billboardEntity, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Backward()
    },
    maxDistance: 30,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: ColliderLayer.CL_CUSTOM4
  })

  // TODO: These ticks wait that raycast creating was finished
  await context.helpers.waitNTicks(3)

  const rayResult = RaycastResult.get(billboardEntity)

  CameraModeArea.deleteFrom(cameraModeAreaE)
  await context.helpers.waitNTicks(3)

  assertEquals(
    rayResult.hits.length,
    1,
    'raycast from entity should hit the collider'
  )
})

// #2507: with `targetEntity` a billboard faces that entity instead of the camera, follows it, and
// freezes where it was when the target is deleted (it used to snap toward the scene origin).
function targetBox(position: Vector3) {
  const target = customAddEntity.addEntity()
  Transform.create(target, { position, scale: Vector3.create(1, 2, 1) })
  MeshCollider.create(target, { collisionMask: ColliderLayer.CL_CUSTOM5, mesh: { $case: 'box', box: { uvs: [] } } })
  return target
}

/** Where the billboard faces: a ray along its local Backward, and the targets it hits. */
async function facing(context: TestFunctionContext, billboard: ReturnType<typeof customAddEntity.addEntity>) {
  Raycast.createOrReplace(billboard, {
    direction: { $case: 'localDirection', localDirection: Vector3.Backward() },
    maxDistance: 30,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: ColliderLayer.CL_CUSTOM5
  })
  await context.helpers.waitNTicks(3)
  const result = RaycastResult.get(billboard)
  return { direction: result.direction ?? Vector3.Zero(), hits: result.hits.map((hit) => hit.entityId) }
}

test('billboard: a targetEntity billboard faces its target, follows it and freezes when it is removed', async function (context) {
  customAddEntity.clean()
  await assertMovePlayerTo(context, Vector3.create(8, 0, 2), sceneCenter)
  const target = targetBox(Vector3.create(12, 1, 8))
  await waitForMeshColliderApplied(context)
  const billboard = customAddEntity.addEntity()
  Transform.create(billboard, { position: Vector3.create(4, 1, 8) })
  Billboard.create(billboard, { billboardMode: BillboardMode.BM_ALL, targetEntity: target })
  await context.helpers.waitNTicks(3)

  const first = await facing(context, billboard)
  assert(first.hits.includes(target), `the billboard does not face its target: hits ${JSON.stringify(first.hits)}`)

  Transform.getMutable(target).position = Vector3.create(4, 1, 13)
  await waitForMeshColliderApplied(context)
  await context.helpers.waitNTicks(3)
  const moved = await facing(context, billboard)
  assert(moved.hits.includes(target), `the billboard did not follow its target: hits ${JSON.stringify(moved.hits)}`)

  engine.removeEntity(target)
  await context.helpers.waitNTicks(10)
  const frozen = await facing(context, billboard)
  const turned = Vector3.distance(Vector3.normalize(frozen.direction), Vector3.normalize(moved.direction))
  assert(turned < 0.05, `after its target was removed the billboard turned from ${JSON.stringify(moved.direction)} to ${JSON.stringify(frozen.direction)}`)
  customAddEntity.clean()
})
