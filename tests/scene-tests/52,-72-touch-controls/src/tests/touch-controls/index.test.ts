import { CameraMode, CameraType, Transform, engine } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { test } from 'testing-library/src/testing'
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

test('touch controls: spreading two fingers goes first person, pinching goes back', async function (context) {
  await restart(context)
  assert(cameraMode() === CameraType.CT_THIRD_PERSON, `the test starts in third person, not ${cameraMode()}`)
  await pinch(context, 200, 20)
  assert(cameraMode() === CameraType.CT_FIRST_PERSON, `after spreading the camera is ${cameraMode()}, expected first person`)
  await pinch(context, -200, 20)
  assert(cameraMode() === CameraType.CT_THIRD_PERSON, `after pinching the camera is ${cameraMode()}, expected third person`)
})
