import {
  EngineInfo,
  MeshCollider,
  Raycast,
  RaycastQueryType,
  RaycastResult,
  Transform,
  engine
} from '@dcl/sdk/ecs'
import { Quaternion, Vector3 } from '@dcl/sdk/math'

import {
  assertComponentValue,
  assertEquals
} from 'testing-library/src/testing/assert'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { test } from 'testing-library/src/testing'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { createChainedEntities } from 'testing-library/src/utils/helpers'
import { waitForMeshColliderApplied } from 'testing-library/src/utils/godot'

function getNextTickNumber(): number {
  return (EngineInfo.getOrNull(engine.RootEntity)?.tickNumber ?? -1) + 1
}

async function waitForRaycastResult(
  context: TestFunctionContext
): Promise<void> {
  // TODO: (GODOT) review this in godot, in some cases there is no synchronization between send_to_reenderer and recv_from_renderer
  await context.helpers.waitNTicks(2)
}

test('raycast: raycast should hits only entities with collisionMask 1', async function (context) {
  customAddEntity.clean()
  const [eA, eB, eC] = Array.from({ length: 3 }, () =>
    customAddEntity.addEntity()
  )
  // An entity A with Raycast hits B and no C (because it hits all but C is in another layer)
  Transform.create(eA, { position: Vector3.One() })

  Transform.create(eB, { position: Vector3.create(4, 1, 1) })
  MeshCollider.create(eB, {
    collisionMask: 1, // 0b0000 0000 0000 0001
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  Transform.create(eC, { position: Vector3.create(6, 1, 1) })
  MeshCollider.create(eC, {
    collisionMask: 2, // 0b0000 0000 0000 0010
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  await waitForMeshColliderApplied(context)

  Raycast.create(eA, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Right()
    },
    maxDistance: 15,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: 1 // 0b0000 0000 0000 0001
  })
  await waitForRaycastResult(context)

  let rayResult = RaycastResult.get(eA)
  assertEquals(
    rayResult.hits.length,
    1,
    'raycast should hits only the colliderMesh with CM 1.'
  )

  MeshCollider.createOrReplace(eC, {
    collisionMask: 1,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  await waitForMeshColliderApplied(context)

  Raycast.createOrReplace(eA, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Right()
    },
    maxDistance: 15,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: 1
  })
  await waitForRaycastResult(context)

  rayResult = RaycastResult.get(eA)
  assertEquals(
    rayResult.hits.length,
    2,
    'raycast should hits all the colliderMeshs with CM 1.'
  )
})

test('raycast: raycast should hits only the first one entity with collisionMask 1', async function (context) {
  customAddEntity.clean()
  const [eA, eB, eC] = Array.from({ length: 3 }, () =>
    customAddEntity.addEntity()
  )
  Transform.create(eA, { position: Vector3.One() })

  Transform.create(eB, { position: Vector3.create(4, 1, 1) })
  MeshCollider.create(eB, {
    collisionMask: 1,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  Transform.create(eC, { position: Vector3.create(6, 1, 1) })
  MeshCollider.create(eC, {
    collisionMask: 1,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  await waitForMeshColliderApplied(context)

  Raycast.create(eA, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Right()
    },
    maxDistance: 15,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    continuous: false,
    collisionMask: 1
  })
  await waitForRaycastResult(context)
  const rayResult = RaycastResult.get(eA)

  assertEquals(
    rayResult.hits.length,
    1,
    'raycast should hits only the first entity.'
  )
})

test('raycast: raycast should not hits any entity because has been rotated', async function (context) {
  customAddEntity.clean()
  const [eA, eB, eC] = Array.from({ length: 3 }, () =>
    customAddEntity.addEntity()
  )
  Transform.create(eA, {
    position: Vector3.One(),
    rotation: Quaternion.fromEulerDegrees(0, -90, 0)
  })

  Transform.create(eB, { position: Vector3.create(4, 1, 1) })
  MeshCollider.create(eB, {
    collisionMask: 1,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  Transform.create(eC, { position: Vector3.create(6, 1, 1) })
  MeshCollider.create(eC, {
    collisionMask: 1,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })
  await waitForMeshColliderApplied(context)

  Raycast.create(eA, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Right()
    },
    maxDistance: 15,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: 1
  })
  console.log(
    `ticknumber=${EngineInfo.getOrNull(engine.RootEntity)?.tickNumber}`
  )

  await waitForRaycastResult(context)

  console.log(
    `ticknumber=${EngineInfo.getOrNull(engine.RootEntity)?.tickNumber}`
  )
  const rayResult = RaycastResult.get(eA)
  assertEquals(
    rayResult.hits.length,
    0,
    'raycast has been rotated, it should hits any entities.'
  )
})

test('raycast: rotated raycast should hits entities with any collisionMask', async function (context) {
  customAddEntity.clean()
  const [eA, eB, eC] = Array.from({ length: 3 }, () =>
    customAddEntity.addEntity()
  )
  Transform.create(eA, {
    position: Vector3.One(),
    rotation: Quaternion.fromEulerDegrees(0, -90, 0)
  })

  Transform.create(eB, { position: Vector3.create(1, 1, 4) })
  MeshCollider.create(eB, {
    collisionMask: 1,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  Transform.create(eC, { position: Vector3.create(1, 1, 6) })
  MeshCollider.create(eC, {
    collisionMask: 2,
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  await waitForMeshColliderApplied(context)

  Raycast.create(eA, {
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Right()
    },
    maxDistance: 15,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: 0xffffffff
  })
  await waitForRaycastResult(context)

  const rayResult = RaycastResult.get(eA)
  assertEquals(
    rayResult.hits.length,
    2,
    'raycast hits should be 2 when CollisionMask is 0xffffffff. '
  )
})

test('raycast: raycasting from an entity to global origin yields correct direction', async function (context) {
  const globalTarget = Vector3.create(0, 10, 0)
  const globalOrigin = Vector3.One()

  // 1. Create an entity with a transform component and a raycast component
  const entity = customAddEntity.addEntity()

  Transform.create(entity, { position: globalOrigin })
  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'globalTarget', globalTarget },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 3
  })
  // 2. Wait for the next frame to let the RaycastSystem to process the raycast
  const resultTickNumber = getNextTickNumber()

  await waitForRaycastResult(context)

  // 3. Validate that the RaycastResult component of the entity has the correct direction
  assertComponentValue(entity, RaycastResult, {
    direction: Vector3.normalize(Vector3.subtract(globalTarget, globalOrigin)),
    globalOrigin,
    hits: [],
    timestamp: 3,
    tickNumber: resultTickNumber
  })
})

test('raycast: raycasting from an entity to local direction origin yields correct direction without transform ', async function (context) {
  // create a new entity with a transform and a raycast component
  const globalOrigin = Vector3.create(0, 10, 0)
  const localDirection = Vector3.Down()

  const entity = customAddEntity.addEntity()

  Transform.create(entity, { position: globalOrigin })
  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'localDirection', localDirection },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 4
  })

  const resultTickNumber = getNextTickNumber()
  await waitForRaycastResult(context)

  // check that the raycast result component was added to the entity
  assertComponentValue(entity, RaycastResult, {
    direction: Vector3.normalize(Vector3.Down()),
    globalOrigin,
    hits: [],
    timestamp: 4,
    tickNumber: resultTickNumber
  })
})

test('raycast: raycasting from an entity to another entity works like globalTarget ', async function (context) {
  // create a new entity with a transform and a raycast component
  const globalOrigin = Vector3.create(0, 10, 0)
  const targetEntityGlobalOrigin = Vector3.create(0, 10, 10)

  const entity = customAddEntity.addEntity()
  const targetEntity = customAddEntity.addEntity()

  Transform.create(entity, { position: globalOrigin })
  Transform.create(targetEntity, { position: targetEntityGlobalOrigin })

  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'targetEntity', targetEntity },
    continuous: false,
    maxDistance: 100,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 5
  })
  // Wait for the next frame to let the RaycastSystem to process the raycast
  const resultTickNumber = getNextTickNumber()
  await waitForRaycastResult(context)

  // check that the raycast result component was added to the entity
  assertComponentValue(entity, RaycastResult, {
    direction: Vector3.normalize(
      Vector3.subtract(targetEntityGlobalOrigin, globalOrigin)
    ),
    globalOrigin,
    hits: [],
    timestamp: 5,
    tickNumber: resultTickNumber
  })
})

test('raycast: raycasting from an entity to local direction origin yields correct direction with last entity rotated', async function (context) {
  // create a new entity with a transform and a raycast component
  const globalOrigin = Vector3.create(0, 10, 0)
  const globalDirection = Vector3.Down()

  const entity = customAddEntity.addEntity()

  Transform.create(entity, {
    position: globalOrigin,
    rotation: Quaternion.fromEulerDegrees(45, 45, 45)
  })
  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'globalDirection', globalDirection },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 6
  })

  // Wait for the next frame to let the RaycastSystem to process the raycast
  const resultTickNumber = getNextTickNumber()
  await waitForRaycastResult(context)

  // check that the raycast result component was added to the entity
  assertComponentValue(entity, RaycastResult, {
    direction: Vector3.normalize(Vector3.Down()),
    globalOrigin,
    hits: [],
    timestamp: 6,
    tickNumber: resultTickNumber
  })
})

test('raycast: raycasting from a translated origin works', async function (context) {
  // this is the paremeter of the globalTarget
  const globalTarget = Vector3.create(0, 10, 0)

  // 1. Create an entity that is located in a transformed origin
  const entity = createChainedEntities([
    { position: Vector3.create(10, 0, 10) },
    { position: Vector3.create(10, 0, 10) }
  ])

  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'globalTarget', globalTarget },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 3
  })
  // Wait for the next frame to let the RaycastSystem to process the raycast
  const resultTickNumber = getNextTickNumber()
  await waitForRaycastResult(context)

  const globalOrigin = Vector3.create(20, 0, 20)

  // 3. Validate that the RaycastResult component of the entity has the correct direction
  assertComponentValue(entity, RaycastResult, {
    direction: Vector3.normalize(Vector3.subtract(globalTarget, globalOrigin)),
    globalOrigin,
    hits: [],
    timestamp: 3,
    tickNumber: resultTickNumber
  })
})

test('raycast: localDirection raycasting from a translated origin works', async function (context) {
  // 1. Create an entity that is located in a transformed origin
  const entity = createChainedEntities([
    {
      position: Vector3.create(10, 0, 10),
      scale: Vector3.create(0.5, 0.5, 0.5)
    },
    {
      position: Vector3.create(10, 0, 10),
      rotation: Quaternion.fromEulerDegrees(0, 90, 0)
    }
  ])

  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'localDirection', localDirection: Vector3.Forward() },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 3
  })
  // Wait for the next frame to let the RaycastSystem to process the raycast
  const resultTickNumber = getNextTickNumber()
  await waitForRaycastResult(context)

  // this is the global origin of the raycast, result of the translation and scaling of the entity
  const globalOrigin = Vector3.create(15, 0, 15)

  // 3. Validate that the RaycastResult component of the entity has the correct direction
  assertComponentValue(entity, RaycastResult, {
    // the direction is now right because the transform was rotated 90 degrees
    direction: Vector3.Right(),
    globalOrigin,
    hits: [],
    timestamp: 3,
    tickNumber: resultTickNumber
  })
})

test('raycast: localDirection raycasting from a translated origin works, with rotated parent', async function (context) {
  // 1. Create an entity that is located in a transformed origin
  const entity = createChainedEntities([
    {
      position: Vector3.create(10, 0, 10),
      scale: Vector3.create(0.5, 0.5, 0.5)
    },
    {
      position: Vector3.create(10, 0, 10),
      rotation: Quaternion.fromEulerDegrees(0, 90, 0)
    },
    {
      scale: Vector3.create(1, 1, 1)
    }
  ])

  Raycast.create(entity, {
    originOffset: Vector3.Zero(),
    direction: { $case: 'localDirection', localDirection: Vector3.Forward() },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 3
  })
  // Wait for the next frame to let the RaycastSystem to process the raycast
  const resultTickNumber = getNextTickNumber()
  await waitForRaycastResult(context)

  // this is the global origin of the raycast, result of the translation and scaling of the entity
  const globalOrigin = Vector3.create(15, 0, 15)

  // 3. Validate that the RaycastResult component of the entity has the correct direction
  assertComponentValue(entity, RaycastResult, {
    // the direction is now right because the transform was rotated 90 degrees
    direction: Vector3.Right(),
    globalOrigin,
    hits: [],
    timestamp: 3,
    tickNumber: resultTickNumber
  })
})

test('raycast: localDirection raycasting from a translated origin works, with rotated parent and offsetOrigin', async function (context) {
  // 1. Create an entity that is located in a transformed origin
  const entity = createChainedEntities([
    {
      position: Vector3.create(10, 0, 10),
      scale: Vector3.create(0.5, 0.5, 0.5)
    },
    {
      position: Vector3.create(10, 0, 10),
      rotation: Quaternion.fromEulerDegrees(0, 90, 0)
    }
  ])

  Raycast.create(entity, {
    // in this case, the originOffset is in the local space of the entity one unit forward
    originOffset: Vector3.Forward(),
    direction: { $case: 'localDirection', localDirection: Vector3.Forward() },
    continuous: false,
    maxDistance: 10,
    queryType: RaycastQueryType.RQT_HIT_FIRST,
    timestamp: 777
  })
  const resultTickNumber = getNextTickNumber()

  await waitForRaycastResult(context)
  // this is the global origin of the raycast, result of the translation and scaling of the entity
  const globalOrigin = Vector3.create(15, 0, 15)
  const rotatedForwardOrigin = Vector3.add(
    Vector3.create(0.5, 0, 0),
    globalOrigin
  )

  // 3. Validate that the RaycastResult component of the entity has the correct direction
  assertComponentValue(entity, RaycastResult, {
    // the direction is now right because the transform was rotated 90 degrees
    timestamp: 777,
    globalOrigin: rotatedForwardOrigin,
    direction: Vector3.Right(),
    // and the globalOrigin is offsetted by originOffset
    hits: [],
    tickNumber: resultTickNumber
  })
})
