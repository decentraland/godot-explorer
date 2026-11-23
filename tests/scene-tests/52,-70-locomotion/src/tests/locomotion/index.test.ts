import { InputModifier, Physics, Transform, engine } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'
import { hold, press, release, tap, type PlayerAction } from 'testing-library/src/utils/input'

// Every run starts from the same spot facing +z, so it has the parcel's length ahead of it.
const START = Vector3.create(8, 0, 2)
const FACING = Vector3.create(8, 1, 16)

function playerPosition(): Vector3 {
  return Transform.get(engine.PlayerEntity).position
}

async function restart(context: TestFunctionContext): Promise<void> {
  await assertMovePlayerTo(context, START, FACING)
  await context.helpers.waitNTicks(20)
}

/** Distance covered holding forward at `strength` (plus `modifier`, if any) for `frames` frames. */
async function distanceForward(
  context: TestFunctionContext,
  frames: number,
  strength: number,
  modifier?: PlayerAction
): Promise<number> {
  await restart(context)
  const from = playerPosition()
  if (modifier !== undefined) await press(modifier)
  await hold(context, 'ia_forward', frames, strength)
  if (modifier !== undefined) await release(modifier)
  // read it at rest, so the reading does not depend on which frame the tick lands on
  await context.helpers.waitNTicks(30)
  const to = playerPosition()
  return Math.hypot(to.x - from.x, to.z - from.z)
}

/** Highest point above the start reached while `jumps` runs, watched for `ticks` ticks. */
async function peakHeight(context: TestFunctionContext, jumps: () => Promise<void>, ticks = 90): Promise<number> {
  await restart(context)
  const ground = playerPosition().y
  let peak = 0
  const watcher = (): void => {
    peak = Math.max(peak, playerPosition().y - ground)
  }
  engine.addSystem(watcher)
  try {
    await jumps()
    await context.helpers.waitNTicks(ticks)
  } finally {
    engine.removeSystem(watcher)
  }
  return peak
}

test('locomotion: holding forward moves the player forward', async function (context) {
  await restart(context)
  const from = playerPosition()
  await hold(context, 'ia_forward', 30)
  await context.helpers.waitNTicks(30)
  const to = playerPosition()
  assert(to.z - from.z > 0.5, `the player moved ${to.z - from.z} m forward, expected more than 0.5`)
  assert(Math.abs(to.x - from.x) < 0.5, `the player drifted ${to.x - from.x} m sideways`)
})

// On a phone the joystick's push sets the speed from walk to jog; only the run button runs.
test('locomotion: a light push walks, a full push jogs, run is faster still', async function (context) {
  const walk = await distanceForward(context, 60, 0.6)
  const jog = await distanceForward(context, 60, 1)
  const run = await distanceForward(context, 60, 1, 'ia_sprint')
  assert(walk > 0.5 && walk < jog && jog < run, `walk ${walk} m, jog ${jog} m, run ${run} m: expected each to cover more`)
})

test('locomotion: jump lifts the player', async function (context) {
  const peak = await peakHeight(context, () => tap(context, 'ia_jump'))
  assert(peak > 0.5, `the jump peaked at ${peak} m, expected more than 0.5`)
})

test('locomotion: a double jump goes higher than a single jump', async function (context) {
  const single = await peakHeight(context, () => tap(context, 'ia_jump'))
  const double = await peakHeight(context, async () => {
    await tap(context, 'ia_jump')
    await context.helpers.waitNTicks(12)
    await tap(context, 'ia_jump')
  })
  assert(double > single + 0.3, `double jump peaked at ${double} m, single at ${single} m`)
})

test('locomotion: InputModifier disableJump keeps the player on the ground', async function (context) {
  InputModifier.createOrReplace(engine.PlayerEntity, {
    mode: { $case: 'standard', standard: { disableJump: true } }
  })
  try {
    await context.helpers.waitNTicks(2)
    const peak = await peakHeight(context, () => tap(context, 'ia_jump'), 40)
    assert(peak < 0.2, `with jumps disabled the player still rose ${peak} m`)
  } finally {
    InputModifier.deleteFrom(engine.PlayerEntity)
  }
})

// #1537: one-shot impulses and continuous forces on the player through the SDK's Physics helper.
/** The highest the player gets over `frames` frames, above where it started. */
async function highestRise(context: TestFunctionContext, frames: number): Promise<number> {
  const ground = playerPosition().y
  let highest = 0
  for (let frame = 0; frame < frames; frame++) {
    await context.helpers.waitNTicks(1)
    highest = Math.max(highest, playerPosition().y - ground)
  }
  return highest
}

test('locomotion: an upward impulse lifts the player, who lands again', async function (context) {
  await restart(context)
  Physics.applyImpulseToPlayer(Vector3.create(0, 1, 0), 8)
  const rise = await highestRise(context, 90)
  assert(rise > 1, `the impulse lifted the player ${rise.toFixed(2)} m, expected more than 1`)
  await context.helpers.waitNTicks(60)
  assert(playerPosition().y < START.y + 0.2, `the player is still ${playerPosition().y.toFixed(2)} m up after landing`)
})

test('locomotion: a sideways impulse pushes the player along it', async function (context) {
  await restart(context)
  const from = playerPosition()
  Physics.applyImpulseToPlayer(Vector3.create(1, 0.3, 0), 7)
  await context.helpers.waitNTicks(90)
  const to = playerPosition()
  assert(to.x - from.x > 1, `the impulse pushed the player ${(to.x - from.x).toFixed(2)} m along +x, expected more than 1`)
})

// The force is added against gravity, which is stronger: a lift of 12 makes jumps go higher, it
// does not lift a standing player.
test('locomotion: a continuous upward force makes jumps higher until it is removed', async function (context) {
  const normal = await peakHeight(context, () => tap(context, 'ia_jump'))
  const source = engine.addEntity()
  Physics.applyForceToPlayer(source, Vector3.create(0, 1, 0), 12)
  const lifted = await peakHeight(context, () => tap(context, 'ia_jump'))
  Physics.removeForceFromPlayer(source)
  engine.removeEntity(source)
  const after = await peakHeight(context, () => tap(context, 'ia_jump'))
  assert(lifted > normal + 0.3, `with the force the jump peaked at ${lifted.toFixed(2)} m, without at ${normal.toFixed(2)} m`)
  assert(Math.abs(after - normal) < 0.2, `after removing the force the jump peaked at ${after.toFixed(2)} m, before at ${normal.toFixed(2)} m`)
})
