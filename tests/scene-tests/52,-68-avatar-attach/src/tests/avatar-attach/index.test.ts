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

// #1730: every bone anchor lands on its bone. Bone positions depend on the body and the idle pose,
// so these are the body's structure, not exact heights: the spine runs up from the hips to the
// head, each limb runs down from the shoulder or hip, and left bones are on the left (-x).
const BONES: Array<[string, AvatarAnchorPointType]> = [
  ['hip', AvatarAnchorPointType.AAPT_HIP],
  ['spine', AvatarAnchorPointType.AAPT_SPINE],
  ['neck', AvatarAnchorPointType.AAPT_NECK],
  ['head', AvatarAnchorPointType.AAPT_HEAD],
  ['left shoulder', AvatarAnchorPointType.AAPT_LEFT_SHOULDER],
  ['left arm', AvatarAnchorPointType.AAPT_LEFT_ARM],
  ['left forearm', AvatarAnchorPointType.AAPT_LEFT_FOREARM],
  ['left hand index', AvatarAnchorPointType.AAPT_LEFT_HAND_INDEX],
  ['left up leg', AvatarAnchorPointType.AAPT_LEFT_UP_LEG],
  ['left leg', AvatarAnchorPointType.AAPT_LEFT_LEG],
  ['left foot', AvatarAnchorPointType.AAPT_LEFT_FOOT],
  ['left toe base', AvatarAnchorPointType.AAPT_LEFT_TOE_BASE],
  ['right shoulder', AvatarAnchorPointType.AAPT_RIGHT_SHOULDER],
  ['right arm', AvatarAnchorPointType.AAPT_RIGHT_ARM],
  ['right forearm', AvatarAnchorPointType.AAPT_RIGHT_FOREARM],
  ['right hand index', AvatarAnchorPointType.AAPT_RIGHT_HAND_INDEX],
  ['right up leg', AvatarAnchorPointType.AAPT_RIGHT_UP_LEG],
  ['right leg', AvatarAnchorPointType.AAPT_RIGHT_LEG],
  ['right foot', AvatarAnchorPointType.AAPT_RIGHT_FOOT],
  ['right toe base', AvatarAnchorPointType.AAPT_RIGHT_TOE_BASE]
]

test('avatar-attach: each bone anchor lands on its bone', async function (context) {
  customAddEntity.clean()
  await assertMovePlayerTo(context, Vector3.create(8, 0, 8), Vector3.create(8, 1, 16))
  await context.helpers.waitNTicks(50)
  const entities = BONES.map(([, anchor]) => attach(anchor))
  const at: Record<string, Vector3> = {}
  for (const [index, [name]] of BONES.entries()) at[name] = await offsetFromPlayer(context, entities[index])
  const where = (names: string[]) => names.map((name) => `${name} ${describe(at[name])}`).join(', ')
  const risesThrough = (names: string[]) => names.every((name, i) => i === 0 || at[name].y > at[names[i - 1]].y)

  assert(risesThrough(['hip', 'spine', 'neck', 'head']), `the spine does not rise: ${where(['hip', 'spine', 'neck', 'head'])}`)
  for (const side of ['left', 'right']) {
    const arm = [`${side} hand index`, `${side} forearm`, `${side} arm`, `${side} shoulder`]
    const leg = [`${side} toe base`, `${side} leg`, `${side} up leg`]
    assert(risesThrough(arm), `the ${side} arm does not run down from the shoulder: ${where(arm)}`)
    assert(risesThrough(leg), `the ${side} leg does not run down from the hip: ${where(leg)}`)
    const feet = [`${side} foot`, `${side} toe base`]
    assert(feet.every((name) => at[name].y < 0.3), `the ${side} foot is off the ground: ${where(feet)}`)
    const limbs = [`${side} shoulder`, `${side} arm`, `${side} forearm`, `${side} up leg`, `${side} leg`, `${side} foot`]
    const onItsSide = limbs.every((name) => (side === 'left' ? at[name].x < -0.03 : at[name].x > 0.03))
    assert(onItsSide, `the ${side} limbs are not on the ${side}: ${where(limbs)}`)
  }
  customAddEntity.clean()
})
