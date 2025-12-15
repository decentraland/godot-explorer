import {
  Raycast,
  RaycastQueryType,
  RaycastResult,
  Transform,
  type Entity,
  engine
} from '@dcl/sdk/ecs'
import { Quaternion, Vector3 } from '@dcl/sdk/math'
import { getGlobalTransform } from 'testing-library/src/utils/transformed'
import { test } from 'testing-library/src/testing'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { type TestFunctionContext } from 'testing-library/src/testing/types'

let customWaitAndAssertTransformPositionTimestamp = 0
async function waitAndAssertTransformPosition(
  context: TestFunctionContext,
  entity: Entity,
  assertPrefix: string,
  doubleCheckPosition?: Vector3 // TODO: remove when getGlobalTransformPosition is ready
): Promise<void> {
  customWaitAndAssertTransformPositionTimestamp += 1
  Raycast.createOrReplace(entity, {
    timestamp: customWaitAndAssertTransformPositionTimestamp,
    originOffset: Vector3.Zero(),
    direction: {
      $case: 'localDirection',
      localDirection: Vector3.Forward()
    },
    maxDistance: 0.1,
    queryType: RaycastQueryType.RQT_QUERY_ALL,
    continuous: false,
    collisionMask: 0xffffffff
  })

  await context.helpers.waitNTicks(2)

  assert(
    RaycastResult.getOrNull(entity) !== null,
    `${assertPrefix}raycast should have arrived`
  )

  const globalPosition = getGlobalTransform(entity).position
  const calculatedPosition = RaycastResult.get(entity).globalOrigin
  assertEquals(
    calculatedPosition,
    globalPosition,
    `${assertPrefix}global origin from raycast is different to computed transform position S`
  )

  if (doubleCheckPosition !== null) {
    assertEquals(
      calculatedPosition,
      doubleCheckPosition,
      `${assertPrefix}global origin from raycast is different to double check position S`
    )
  }
}

test('transform: should transform test-mechanism works well', async function (context) {
  await context.helpers.waitTicksUntil(() => {
    if (Transform.has(engine.PlayerEntity)) {
      return true
    } else {
      return false
    }
  }, 6e5)

  assert(
    customAddEntity.isEmpty(),
    'custom add entity should be empty in the second test'
  )

  const testEntityA = customAddEntity.addEntity()
  Transform.create(testEntityA, {
    position: Vector3.create(3.3, 4.4, 5.5)
  })
  await waitAndAssertTransformPosition(
    context,
    testEntityA,
    '(test entity A1)',
    Vector3.create(3.3, 4.4, 5.5)
  )

  Transform.getMutable(testEntityA).position = Vector3.add(
    Vector3.One(),
    Vector3.create(3.3, 4.4, 5.5)
  )
  await waitAndAssertTransformPosition(
    context,
    testEntityA,
    '(test entity A2)',
    Vector3.create(4.3, 5.4, 6.5)
  )

  const testEntityB = customAddEntity.addEntity()
  Transform.getMutable(testEntityA).position = Vector3.One()
  Transform.create(testEntityB, {
    position: Vector3.create(1.3, 1.4, 1.5),
    parent: testEntityA
  })
  await waitAndAssertTransformPosition(
    context,
    testEntityB,
    '(test entity B with A as parent)',
    Vector3.create(2.3, 2.4, 2.5)
  )

  let testEntityC = customAddEntity.addEntity()
  Transform.create(testEntityC, {
    position: Vector3.create(5, 5, 5),
    rotation: Quaternion.fromAngleAxis(90, Vector3.Up())
  })
  const testEntityD = customAddEntity.addEntity()
  Transform.create(testEntityD, {
    position: Vector3.Forward(),
    parent: testEntityC
  })
  await waitAndAssertTransformPosition(
    context,
    testEntityD,
    '(test entity D with C rotated as parent)',
    Vector3.create(6, 5, 5)
  )

  engine.removeEntity(testEntityC)
  await waitAndAssertTransformPosition(
    context,
    testEntityD,
    '(test entity D removing its parent)',
    Vector3.create(0, 0, 1)
  )

  testEntityC = customAddEntity.addEntity()
  Transform.create(testEntityC, {
    position: Vector3.create(5, 5, 5),
    scale: Vector3.create(2, 2, 2)
  })
  Transform.createOrReplace(testEntityD, {
    position: Vector3.Forward(),
    parent: testEntityC
  })

  await waitAndAssertTransformPosition(
    context,
    testEntityD,
    '(test entity D with C scaled as parent)',
    Vector3.create(5, 5, 7)
  )

  engine.removeEntity(testEntityC)
  await waitAndAssertTransformPosition(
    context,
    testEntityD,
    '(test entity D removing its parent)',
    Vector3.create(0, 0, 1)
  )
})

test(`transform: should transform with a parent add parent's translation`, async function (context) {
  customAddEntity.clean()
  assert(
    customAddEntity.isEmpty(),
    'custom add entity should be empty in the second test '
  )
  const testEntityA = customAddEntity.addEntity()
  Transform.create(testEntityA, {
    position: Vector3.One()
  })
  const testEntityB = customAddEntity.addEntity()
  Transform.create(testEntityB, {
    position: Vector3.One(),
    parent: testEntityA
  })
  await waitAndAssertTransformPosition(
    context,
    testEntityB,
    '(test entity B ) ',
    Vector3.create(2, 2, 2)
  )
})

test(`transform: should transform with a parent add parent's translation with scale/rotation`, async function (context) {
  const [eA, eB, eC, eD, eE, eF] = Array.from({ length: 6 }, () =>
    customAddEntity.addEntity()
  )

  Transform.create(eA, {
    position: Vector3.One()
  })
  Transform.create(eB, {
    position: Vector3.One(),
    scale: Vector3.scale(Vector3.One(), 0.5),
    parent: eA
  })
  Transform.create(eC, {
    position: Vector3.One(),
    scale: Vector3.scale(Vector3.One(), 0.5),
    parent: eB
  })

  await waitAndAssertTransformPosition(
    context,
    eC,
    '(test entity C) ',
    Vector3.create(2.5, 2.5, 2.5)
  )

  Transform.createOrReplace(eC, {
    position: Vector3.One(),
    scale: Vector3.scale(Vector3.One(), 0.5),
    rotation: Quaternion.fromEulerDegrees(0, 90, 0),
    parent: eB
  })
  Transform.create(eD, {
    position: Vector3.Forward(),
    parent: eC
  })

  await waitAndAssertTransformPosition(
    context,
    eD,
    '(test entity D) ',
    Vector3.create(2.75, 2.5, 2.5)
  )

  Transform.create(eE, {
    position: Vector3.Forward(),
    rotation: Quaternion.fromEulerDegrees(0, 90, 0),
    parent: eD
  })

  await waitAndAssertTransformPosition(
    context,
    eE,
    '(test entity E) ',
    Vector3.create(3.0, 2.5, 2.5)
  )

  Transform.create(eF, {
    position: Vector3.Forward(),
    parent: eE
  })

  await waitAndAssertTransformPosition(
    context,
    eF,
    '(test entity F) ',
    Vector3.create(3.0, 2.5, 2.25)
  )

  engine.removeEntity(eE)

  await waitAndAssertTransformPosition(
    context,
    eF,
    '(test entity F) ',
    Vector3.create(0, 0, 1)
  )
})
