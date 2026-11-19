import { test } from 'testing-library/src/testing'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { query, setSetting } from 'testing-library/src/utils/engine'

test('app commands: a state query reports this parcel and realm', async function (context) {
  await context.helpers.waitNTicks(10)
  const state = await query('state')
  assertEquals(state.parcel, [52, -74], 'the player stands on this scene')
  assert(String(state.realm).endsWith('/scene-explorer-tests'), `realm is ${state.realm}`)
})

test('app commands: an avatar query finds the local player', async function () {
  const avatar = await query('avatar', { by: 'local' })
  assert(avatar.identity?.is_local === true, `expected the local avatar, got ${JSON.stringify(avatar.identity)}`)
  assert(avatar.godot?.present === true, 'the local avatar node is in the tree')
})

test('app commands: a query that changes state is refused', async function () {
  let refused = ''
  try {
    await query('teleport', { x: 0, y: 0 })
  } catch (error) {
    refused = String(error)
  }
  assert(refused.includes('not a read-only query'), `teleport was not refused: '${refused}'`)
})

test('app commands: a graphic profile change goes through the settings', async function () {
  const changed = await setSetting('graphic_profile', 0)
  assert(changed > 0, `setting the graphic profile changed ${changed} settings`)
})
