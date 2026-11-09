import { AvatarEmoteCommand, AvatarMask, engine } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { triggerEmote } from '~system/RestrictedActions'
import { skipTest, test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { query } from 'testing-library/src/utils/engine'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'
import { hold } from 'testing-library/src/utils/input'

// AvatarEmoteCommand.state, as the explorer writes it (avatar_emote_command.proto).
const STARTED = 0
const INTERRUPTED = 2

const states: number[] = []
AvatarEmoteCommand.onChange(engine.PlayerEntity, (value) => {
  if (value?.state !== undefined) states.push(value.state)
})

/** Waits until the local avatar is built: an emote sent before that is parked until it is. */
async function avatarReady(context: TestFunctionContext): Promise<void> {
  for (let tick = 0; tick < 1200; tick++) {
    const reply = await query('avatar', { by: 'local', filters: { collect_nodes: { '.': ['avatar_ready'] } } })
    if (reply?.godot?.['.']?.avatar_ready === true) return
    await context.helpers.waitNTicks(1)
  }
  assert(false, 'the local avatar never got ready')
}

/** Triggers `emote`, walks forward for a second, and returns the emote states seen. */
async function emoteAndWalk(context: TestFunctionContext, emote: string, masked: boolean): Promise<number[]> {
  await assertMovePlayerTo(context, Vector3.create(8, 0, 2), Vector3.create(8, 1, 16))
  await avatarReady(context)
  states.length = 0
  await triggerEmote(masked ? { predefinedEmote: emote, mask: AvatarMask.AM_UPPER_BODY } : { predefinedEmote: emote })
  // Wall clock, not ticks: the emote is downloaded on first use, and a headless run spends
  // ticks far faster than that.
  const deadline = Date.now() + 15000
  while (!states.includes(STARTED) && Date.now() < deadline) await context.helpers.waitNTicks(1)
  await hold(context, 'ia_forward', 60)
  await context.helpers.waitNTicks(10)
  return [...states]
}

// #2621: an upper-body (masked) emote keeps playing while the player walks; a full-body one is
// cancelled by movement.
test('emotes: an upper-body emote keeps playing while the player walks', async function (context) {
  const seen = await emoteAndWalk(context, 'wave', true)
  assert(seen.includes(STARTED), `the emote never started: states ${JSON.stringify(seen)}`)
  assert(!seen.includes(INTERRUPTED), `walking interrupted the upper-body emote: states ${JSON.stringify(seen)}`)
})

// Skipped until #3244 is fixed: only a scene's first emote plays, and this is the second.
skipTest('emotes: walking interrupts a full-body emote', async function (context) {
  const seen = await emoteAndWalk(context, 'clap', false)
  assert(seen.includes(STARTED), `the emote never started: states ${JSON.stringify(seen)}`)
  assert(seen.includes(INTERRUPTED), `walking did not interrupt the full-body emote: states ${JSON.stringify(seen)}`)
})
