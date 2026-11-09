import {
  ColliderLayer,
  Entity,
  Transform,
  TriggerArea,
  TriggerAreaEventType,
  engine,
  triggerAreaEventsSystem
} from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { skipTest, test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'
import { getPlayer, onEnterScene, onLeaveScene } from '@dcl/sdk/players'
import { hold } from 'testing-library/src/utils/input'

// #2382: registered once, at load, so a count above one is a repeated event, not a second handler.
const enters: Array<{ userId: string; hasPosition: boolean }> = []
onEnterScene((player) =>
  enters.push({ userId: player.userId, hasPosition: Transform.getOrNull(player.entity)?.position != null })
)
const leaves: string[] = []
onLeaveScene((userId) => leaves.push(userId))

const OUTSIDE = Vector3.create(8, 0, 1)
const LOOK = Vector3.create(8, 1, 16)

type Seen = { phase: TriggerAreaEventType; layers: number }

/** A box area at `center`, subscribed to `mask`, recording every result it gets. */
function area(center: Vector3, mask: ColliderLayer): { entity: Entity; seen: Seen[] } {
  const entity = engine.addEntity()
  Transform.create(entity, { position: center, scale: Vector3.create(4, 4, 4) })
  TriggerArea.setBox(entity, mask)
  const seen: Seen[] = []
  const record = (result: { trigger?: { layers: number }; eventType: TriggerAreaEventType }) =>
    seen.push({ phase: result.eventType, layers: result.trigger?.layers ?? 0 })
  triggerAreaEventsSystem.onTriggerEnter(entity, record)
  triggerAreaEventsSystem.onTriggerStay(entity, record)
  triggerAreaEventsSystem.onTriggerExit(entity, record)
  return { entity, seen }
}

/** Walks the player in, waits, and out again; returns what the area saw. */
async function walkThrough(context: TestFunctionContext, center: Vector3, mask: ColliderLayer): Promise<Seen[]> {
  await assertMovePlayerTo(context, OUTSIDE, LOOK)
  await context.helpers.waitNTicks(10)
  const { entity, seen } = area(center, mask)
  try {
    await context.helpers.waitNTicks(10)
    await assertMovePlayerTo(context, Vector3.create(center.x, 0, center.z), LOOK)
    await context.helpers.waitNTicks(30)
    await assertMovePlayerTo(context, OUTSIDE, LOOK)
    await context.helpers.waitNTicks(30)
    return seen
  } finally {
    triggerAreaEventsSystem.removeOnTriggerEnter(entity)
    triggerAreaEventsSystem.removeOnTriggerStay(entity)
    triggerAreaEventsSystem.removeOnTriggerExit(entity)
    engine.removeEntity(entity)
  }
}

const phases = (seen: Seen[]) => new Set(seen.map((s) => s.phase))
const PHASE_NAMES: Record<number, string> = {
  [TriggerAreaEventType.TAET_ENTER]: 'ENTER',
  [TriggerAreaEventType.TAET_STAY]: 'STAY',
  [TriggerAreaEventType.TAET_EXIT]: 'EXIT'
}
const describe = (seen: Seen[]) => JSON.stringify(seen.slice(0, 6))

// #2227: an area that subscribes to CL_PLAYER (4) gets the local player's results with the
// main-player bit (8) as well, on enter, stay and exit, so a scene can tell the local player apart
// from remote ones by the bitmask alone.
test('trigger area: the local player carries the main-player layer on enter, stay and exit', async function (context) {
  const seen = await walkThrough(context, Vector3.create(4, 2, 8), ColliderLayer.CL_PLAYER)
  const got = phases(seen)
  for (const phase of [
    TriggerAreaEventType.TAET_ENTER,
    TriggerAreaEventType.TAET_STAY,
    TriggerAreaEventType.TAET_EXIT
  ]) {
    assert(got.has(phase), `no ${PHASE_NAMES[phase]} result: ${describe(seen)}`)
  }
  const missing = seen.filter((s) => (s.layers & ColliderLayer.CL_MAIN_PLAYER) === 0)
  assert(missing.length === 0, `results without the main-player bit: ${describe(missing)}`)
  assert(
    seen.every((s) => (s.layers & ColliderLayer.CL_PLAYER) !== 0),
    `results without the player bit: ${describe(seen)}`
  )
})

test('trigger area: an area subscribed to the main player fires for the local player', async function (context) {
  const seen = await walkThrough(context, Vector3.create(12, 2, 8), ColliderLayer.CL_MAIN_PLAYER)
  const got = phases(seen)
  assert(
    got.has(TriggerAreaEventType.TAET_ENTER) && got.has(TriggerAreaEventType.TAET_EXIT),
    `results: ${describe(seen)}`
  )
  assert(
    seen.every((s) => (s.layers & ColliderLayer.CL_MAIN_PLAYER) !== 0),
    `results without the main-player bit: ${describe(seen)}`
  )
})

// #2382: one join fired onSceneEnter 7-9 times, only the first with a position. The local player
// joined this scene once (and the tests above teleported it around inside): one enter, with a
// position.
test('trigger area: the local player entered the scene once, with a position', async function (context) {
  // getPlayer() can briefly find none of the local player's components.
  let me: string | undefined
  const deadline = Date.now() + 5000
  while (me === undefined && Date.now() < deadline) {
    await context.helpers.waitNTicks(1)
    me = getPlayer()?.userId
  }
  assert(me !== undefined, 'no local player')
  const mine = enters.filter((enter) => enter.userId === me)
  assert(mine.length === 1, `onEnterScene fired ${mine.length} times for the local player`)
  assert(mine[0].hasPosition, 'the enter had no position for the local player')
})

// Skipped until #3246 is fixed: onLeaveScene fires when the local player walks out of the scene,
// and onEnterScene again when it walks back in. Last in this scene: it walks the player outside.
skipTest('trigger area: the local player walking out fires onLeaveScene, walking back in onEnterScene', async function (context) {
  const me = getPlayer()!.userId
  await assertMovePlayerTo(context, Vector3.create(8, 0, 1), Vector3.create(8, 1, 16))
  await context.helpers.waitNTicks(20)
  const entersBefore = enters.filter((enter) => enter.userId === me).length
  await hold(context, 'ia_backward', 60)
  await context.helpers.waitNTicks(20)
  assert(leaves.includes(me), `walking out did not fire onLeaveScene: leaves ${JSON.stringify(leaves)}`)
  await hold(context, 'ia_forward', 90)
  await context.helpers.waitNTicks(20)
  const entersAfter = enters.filter((enter) => enter.userId === me).length
  assert(entersAfter === entersBefore + 1, `walking back in fired onEnterScene ${entersAfter - entersBefore} times, expected once`)
})
