import { Transform, engine, type Entity } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { query, sceneId } from 'testing-library/src/utils/engine'
import { areaNpcs, npc } from '../../avatar-shape-motion'

type NpcNode = { visible: boolean; walk: boolean; jog: boolean; run: boolean }

/** The NPC's avatar node as the engine has it, or null before it exists. */
async function npcNode(entity: Entity): Promise<NpcNode | null> {
  const reply = await query('entity', {
    scene_id: await sceneId(),
    entity_id: entity,
    filters: {
      include_parents: false,
      include_children: false,
      collect_nodes: { AvatarShape: ['visible', 'walk', 'jog', 'run'] }
    }
  })
  const node = reply?.godot?.AvatarShape
  if (node === undefined || node === null) return null
  return {
    visible: node.visible === true,
    walk: node.walk === true,
    jog: node.jog === true,
    run: node.run === true
  }
}

function gait(node: NpcNode | null): string {
  if (node === null) return 'no node'
  if (node.run) return 'run'
  if (node.jog) return 'jog'
  return node.walk ? 'walk' : 'idle'
}

// Wall clock, not ticks: a headless run spends ticks far faster than real time, and the engine
// learns an NPC's scene ~0.1 s after it spawns.
async function waitSeconds(
  context: TestFunctionContext,
  seconds: number
): Promise<void> {
  const deadline = Date.now() + seconds * 1000
  while (Date.now() < deadline) await context.helpers.waitNTicks(1)
}

async function untilNode(
  context: TestFunctionContext,
  entity: Entity,
  seconds: number
): Promise<NpcNode | null> {
  const deadline = Date.now() + seconds * 1000
  while (Date.now() < deadline) {
    const node = await npcNode(entity)
    if (node !== null) return node
    await context.helpers.waitNTicks(1)
  }
  return null
}

test('avatar shape motion: excludeIds keeps a plain-id NPC visible in a hide area (#3338)', async function (context) {
  assert(
    (await untilNode(context, areaNpcs.visible, 30)) !== null,
    'npc-1 never got an avatar node'
  )
  await waitSeconds(context, 3)
  assert(
    (await npcNode(areaNpcs.visible))?.visible === true,
    'npc-1 is in excludeIds but is hidden'
  )
  for (const entity of areaNpcs.hidden) {
    const node = await npcNode(entity)
    assert(
      node !== null && !node.visible,
      `NPC ${entity} is not in excludeIds but is visible`
    )
  }
})

test('avatar shape motion: a scene-moved NPC walks, runs and stops by its speed (#3340)', async function (context) {
  let speed = 0
  let x = 4
  let direction = 1
  const probe = npc('gait-probe', Vector3.create(x, 0, 30))
  function drive(dt: number): void {
    x += direction * speed * dt
    if (x >= 28) direction = -1
    if (x <= 4) direction = 1
    Transform.getMutable(probe).position.x = x
  }
  engine.addSystem(drive)
  assert(
    (await untilNode(context, probe, 30)) !== null,
    'the probe never got an avatar node'
  )

  const expectGait = async (
    metersPerSecond: number,
    scale: number,
    expected: string
  ): Promise<void> => {
    speed = metersPerSecond
    Transform.getMutable(probe).scale = Vector3.create(scale, scale, scale)
    await waitSeconds(context, 1.5)
    const actual = gait(await npcNode(probe))
    assert(
      actual === expected,
      `${metersPerSecond} m/s at ${scale}x played ${actual}, expected ${expected}`
    )
  }
  await expectGait(10, 1, 'run')
  await expectGait(3, 1, 'walk')
  // 10 m/s on a 3x body is 3.3 m/s for its size.
  await expectGait(10, 3, 'walk')
  // 3 m/s on a 6x body is 0.5 m/s for its size, at the idle floor: still visibly moving, so a walk.
  await expectGait(3, 6, 'walk')
  await expectGait(0, 1, 'idle')

  engine.removeSystem(drive)
  engine.removeEntity(probe)
})
