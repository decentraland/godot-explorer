import { InputModifier, Transform, engine } from '@dcl/sdk/ecs'
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
