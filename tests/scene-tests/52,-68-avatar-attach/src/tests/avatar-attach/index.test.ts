import { AvatarAnchorPointType, AvatarAttach, Entity, MeshRenderer, Transform, engine } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { renderedNode } from 'testing-library/src/utils/engine'
import { test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'

function attach(anchorPointId: AvatarAnchorPointType): Entity {
  const entity = customAddEntity.addEntity()
  AvatarAttach.create(entity, { anchorPointId })
  MeshRenderer.setSphere(entity)
  return entity
}

// Where the attached node is, relative to the player's position (their feet).
async function offsetFromPlayer(context: TestFunctionContext, entity: Entity): Promise<Vector3> {
  await context.helpers.waitNTicks(10)
  const position = (await renderedNode(entity)).position
  assert(position !== null, 'the attached entity has no node')
  return Vector3.subtract(position!, Transform.get(engine.PlayerEntity).position)
}

function describe(offset: Vector3): string {
  return `(${offset.x.toFixed(2)}, ${offset.y.toFixed(2)}, ${offset.z.toFixed(2)})`
}

// Facing +z, the avatar's left is -x. Ranges, not exact values: the idle animation moves the hands.
const ANCHORS: Array<[string, AvatarAnchorPointType, (offset: Vector3) => boolean]> = [
  ['position', AvatarAnchorPointType.AAPT_POSITION, (o) => Vector3.length(o) < 0.05],
  ['name tag', AvatarAnchorPointType.AAPT_NAME_TAG, (o) => o.y > 2 && o.y < 3 && Math.hypot(o.x, o.z) < 0.3],
  ['left hand', AvatarAnchorPointType.AAPT_LEFT_HAND, (o) => o.x < -0.15 && o.x > -0.6 && o.y > 0.6 && o.y < 1.3],
  ['right hand', AvatarAnchorPointType.AAPT_RIGHT_HAND, (o) => o.x > 0.15 && o.x < 0.6 && o.y > 0.6 && o.y < 1.3]
]

async function expectAnchors(context: TestFunctionContext, entities: Entity[]): Promise<void> {
  for (const [index, [name, , near]] of ANCHORS.entries()) {
    const offset = await offsetFromPlayer(context, entities[index])
    assert(near(offset), `${name}: ${describe(offset)} from the player`)
  }
}

test('avatar-attach: each anchor puts the entity on the avatar', async function (context) {
  customAddEntity.clean()
  await assertMovePlayerTo(context, Vector3.create(8, 0, 8), Vector3.create(8, 1, 16))
  await context.helpers.waitNTicks(50)
  await expectAnchors(context, ANCHORS.map(([, anchor]) => attach(anchor)))
})

test('avatar-attach: attached entities follow the player', async function (context) {
  const entities = ANCHORS.map(([, anchor]) => attach(anchor))
  await assertMovePlayerTo(context, Vector3.create(4, 0, 4), Vector3.create(4, 1, 12))
  await context.helpers.waitNTicks(50)
  await expectAnchors(context, entities)
  customAddEntity.clean()
})
