import { CameraMode, CameraType, Entity, MainCamera, Transform, VirtualCamera, engine } from '@dcl/sdk/ecs'
import { movePlayerTo } from '~system/RestrictedActions'
import { Vector3 } from '@dcl/sdk/math'
import { skipTest, test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { type TestFunctionContext } from 'testing-library/src/testing/types'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'
import { joystick, look, pinch } from 'testing-library/src/utils/input'

const START = Vector3.create(8, 0, 2)
const FACING = Vector3.create(8, 1, 16)

async function restart(context: TestFunctionContext): Promise<void> {
  await assertMovePlayerTo(context, START, FACING)
  await context.helpers.waitNTicks(20)
}

/** The camera's heading in degrees: 0 faces +z, 90 faces +x. */
function cameraHeading(): number {
  const forward = Vector3.rotate(Vector3.Forward(), Transform.get(engine.CameraEntity).rotation)
  return (Math.atan2(forward.x, forward.z) * 180) / Math.PI
}

function cameraMode(): CameraType | undefined {
  return CameraMode.getOrNull(engine.CameraEntity)?.mode
}

test('touch controls: the joystick pushed forward walks the player forward', async function (context) {
  await restart(context)
  const from = Transform.get(engine.PlayerEntity).position
  await joystick(context, { x: 0, y: -1 }, 1, 60)
  await context.helpers.waitNTicks(30)
  const to = Transform.get(engine.PlayerEntity).position
  assert(to.z - from.z > 3, `the player moved ${to.z - from.z} m forward, expected more than 3`)
  assert(Math.abs(to.x - from.x) < 0.5, `the player drifted ${to.x - from.x} m sideways`)
})

test('touch controls: dragging right turns the camera right', async function (context) {
  await restart(context)
  const before = cameraHeading()
  await look(context, { x: 300, y: 0 }, 20)
  const turned = cameraHeading() - before
  assert(turned > 10, `the camera turned ${turned} degrees, expected more than 10 to the right`)
})

// Skipped until the pinch camera switch is fixed: apply_pinch_zoom re-enters the borrowed Player
// (player/body.rs) and the switch never happens.
skipTest('touch controls: spreading two fingers goes first person, pinching goes back', async function (context) {
  await restart(context)
  assert(cameraMode() === CameraType.CT_THIRD_PERSON, `the test starts in third person, not ${cameraMode()}`)
  await pinch(context, 200, 20)
  assert(cameraMode() === CameraType.CT_FIRST_PERSON, `after spreading the camera is ${cameraMode()}, expected first person`)
  await pinch(context, -200, 20)
  assert(cameraMode() === CameraType.CT_THIRD_PERSON, `after pinching the camera is ${cameraMode()}, expected third person`)
})

// #2885: a scene that switches to a VirtualCamera and back (as cs-1.6 does for its zoom and death
// cam) must leave touch look working. Set up like cs-1.6's ensureCamera: one camera at eye height,
// instant transition; released by deleting MainCamera.
function sceneCamera(): Entity {
  const camera = engine.addEntity()
  const eye = Transform.get(engine.CameraEntity).position
  Transform.create(camera, { position: { ...eye } })
  VirtualCamera.create(camera, { defaultTransition: { transitionMode: VirtualCamera.Transition.Time(0) } })
  return camera
}

async function expectLookTurns(context: TestFunctionContext, what: string): Promise<void> {
  const before = cameraHeading()
  await look(context, { x: 300, y: 0 }, 20)
  const turned = cameraHeading() - before
  assert(turned > 10, `${what}: the camera turned ${turned} degrees, expected more than 10 to the right`)
}

test('touch controls: dragging turns the camera after a scene camera is released', async function (context) {
  await restart(context)
  const camera = sceneCamera()
  MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: camera })
  await context.helpers.waitNTicks(30)
  // While the scene camera is on, the scene points the view: a drag must not turn it.
  const held = cameraHeading()
  await look(context, { x: 300, y: 0 }, 20)
  assert(Math.abs(cameraHeading() - held) < 2, `the scene camera turned ${cameraHeading() - held} degrees with a drag`)
  MainCamera.deleteFrom(engine.CameraEntity)
  await context.helpers.waitNTicks(30)
  await expectLookTurns(context, 'after the release')
  engine.removeEntity(camera)
})

test('touch controls: dragging turns the camera after a release during a teleport', async function (context) {
  await restart(context)
  const camera = sceneCamera()
  MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: camera })
  await context.helpers.waitNTicks(30)
  // The #2885 race: released in the frame of a teleport, set again the next frame, released again.
  void movePlayerTo({ newRelativePosition: START, cameraTarget: FACING })
  MainCamera.deleteFrom(engine.CameraEntity)
  await context.helpers.waitNTicks(1)
  MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: camera })
  await context.helpers.waitNTicks(1)
  MainCamera.deleteFrom(engine.CameraEntity)
  await context.helpers.waitNTicks(30)
  await expectLookTurns(context, 'after the teleport race')
  engine.removeEntity(camera)
})
