import {
  AvatarModifierArea,
  AvatarModifierType,
  AvatarShape,
  Billboard,
  BillboardMode,
  TextShape,
  Transform,
  engine,
  type Entity
} from '@dcl/sdk/ecs'
import { Color4, Vector3 } from '@dcl/sdk/math'
import { movePlayerTo } from '~system/RestrictedActions'

const WEARABLES = [
  'urn:decentraland:off-chain:base-avatars:green_hoodie',
  'urn:decentraland:off-chain:base-avatars:brown_pants',
  'urn:decentraland:off-chain:base-avatars:sneakers',
  'urn:decentraland:off-chain:base-avatars:casual_hair_01',
  'urn:decentraland:off-chain:base-avatars:eyes_00',
  'urn:decentraland:off-chain:base-avatars:eyebrows_00',
  'urn:decentraland:off-chain:base-avatars:mouth_00'
]

// The lanes' speed cycle, with what each scale should play (the walk/jog/run cutoffs are 4 and 6.5
// m/s, applied to the speed divided by the body's scale).
const PHASES = [
  { speed: 10, seconds: 10, sign: '10 m/s\n1x RUN   3x WALK   6x WALK' },
  { speed: 3, seconds: 10, sign: '3 m/s\n1x WALK   3x WALK   6x WALK' },
  { speed: 0, seconds: 6, sign: 'STOPPED\nall IDLE' }
]
const LANE_X = { min: 3, max: 29 }

/** An NPC with no name tag: a label floats above it instead, which the area never hides. */
export function npc(
  id: string,
  position: Vector3,
  scale = 1,
  expressionTriggerId?: string
): Entity {
  const entity = engine.addEntity()
  Transform.create(entity, {
    position,
    scale: Vector3.create(scale, scale, scale)
  })
  AvatarShape.create(entity, {
    id,
    name: '',
    wearables: WEARABLES,
    emotes: [],
    expressionTriggerId
  })
  return entity
}

function label(
  text: string,
  position: Vector3,
  fontSize = 3,
  parent?: Entity
): Entity {
  const entity = engine.addEntity()
  Transform.create(entity, { position, parent })
  TextShape.create(entity, {
    text,
    fontSize,
    textColor: Color4.White(),
    outlineWidth: 0.2,
    outlineColor: Color4.Black()
  })
  Billboard.create(entity, { billboardMode: BillboardMode.BM_Y })
  return entity
}

/** The NPCs inside the AMT_HIDE_AVATARS area, keyed by what they must do. */
export const areaNpcs: { visible: Entity; hidden: Entity[] } = {
  visible: engine.RootEntity,
  hidden: []
}

export function startAvatarShapeMotion(): void {
  // #3338: four NPCs inside a hide area; only the plain id in excludeIds stays visible.
  const area = engine.addEntity()
  Transform.create(area, { position: Vector3.create(16, 1.5, 7) })
  AvatarModifierArea.create(area, {
    area: Vector3.create(12, 4, 3),
    modifiers: [AvatarModifierType.AMT_HIDE_AVATARS],
    excludeIds: ['npc-1']
  })
  const inArea: Array<[string, number, string]> = [
    ['', 10.5, 'no id\nHIDDEN'],
    ['npc-1', 13, 'npc-1\nVISIBLE'],
    ['npc-2', 19, 'npc-2\nHIDDEN'],
    ['0x1111111111111111111111111111111111111111', 21.5, '0x1111…\nHIDDEN']
  ]
  for (const [id, x, text] of inArea) {
    const entity = npc(id, Vector3.create(x, 0, 7))
    if (id === 'npc-1') areaNpcs.visible = entity
    else areaNpcs.hidden.push(entity)
    label(text, Vector3.create(x, 2.6, 7), 2)
  }

  // #3340: three NPCs of different sizes moved together by this scene, on a speed cycle.
  const lanes = [
    { scale: 1, z: 14 },
    { scale: 3, z: 19 },
    { scale: 6, z: 26 }
  ].map(({ scale, z }) => {
    label(`${scale}x`, Vector3.create(1.5, scale * 2 + 0.5, z), 3)
    return {
      entity: npc(`lane-${scale}x`, Vector3.create(LANE_X.min, 0, z), scale),
      z
    }
  })
  const sign = label(PHASES[0].sign, Vector3.create(16, 13, 31), 6)

  // The dancer: its emote wins over the motion, as on desktop.
  const dancer = npc('dancer', Vector3.create(8, 0, 10.5), 1, 'dance')
  label('dancer\nkeeps DANCING', Vector3.create(0, 2.6, 0), 2, dancer)

  // A jump lands on the parcel, not the spawn point: frame the view once the player is inside.
  engine.addSystem(function frame() {
    const player = Transform.getOrNull(engine.PlayerEntity)?.position
    if (
      player === undefined ||
      player.x < 0 ||
      player.x > 32 ||
      player.z < 0 ||
      player.z > 32
    )
      return
    engine.removeSystem(frame)
    void movePlayerTo({
      newRelativePosition: Vector3.create(16, 0, 1),
      cameraTarget: Vector3.create(16, 1.5, 16)
    })
  })

  let phase = 0
  let phaseTime = 0
  let x = LANE_X.min
  let direction = 1
  let dancerX = 8
  let dancerDirection = 1
  engine.addSystem((dt: number) => {
    phaseTime += dt
    if (phaseTime >= PHASES[phase].seconds) {
      phaseTime = 0
      phase = (phase + 1) % PHASES.length
      TextShape.getMutable(sign).text = PHASES[phase].sign
    }
    x += direction * PHASES[phase].speed * dt
    if (x >= LANE_X.max) direction = -1
    if (x <= LANE_X.min) direction = 1
    x = Math.min(Math.max(x, LANE_X.min), LANE_X.max)
    for (const lane of lanes)
      Transform.getMutable(lane.entity).position = Vector3.create(x, 0, lane.z)

    dancerX += dancerDirection * 2 * dt
    if (dancerX >= 24) dancerDirection = -1
    if (dancerX <= 8) dancerDirection = 1
    Transform.getMutable(dancer).position.x = dancerX
  })
}
