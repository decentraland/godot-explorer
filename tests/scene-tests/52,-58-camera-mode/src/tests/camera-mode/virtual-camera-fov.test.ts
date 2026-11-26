import { Entity, MainCamera, Transform, VirtualCamera, engine } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { test } from 'testing-library/src/testing'
import { assertEquals } from 'testing-library/src/testing/assert'
import { query } from 'testing-library/src/utils/engine'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertMovePlayerTo } from 'testing-library/src/utils/helpers'
import { TestFunctionContext } from 'testing-library/src/testing/types'

async function cameraFov(): Promise<number> {
  return (await query('state')).camera_fov
}

// Waits until the drawn camera's fov reaches `fov`; returns every value seen on the way.
async function fovBecomes(context: TestFunctionContext, fov: number, maxTicks = 180): Promise<number[]> {
  const seen: number[] = []
  for (let tick = 0; tick < maxTicks; tick++) {
    const now = await cameraFov()
    seen.push(now)
    if (Math.abs(now - fov) < 0.01) break
    await context.helpers.waitNTicks(1)
  }
  return seen
}

function useVirtualCamera(fov: number | undefined, time = 0.5): Entity {
  const camera = customAddEntity.addEntity()
  Transform.create(camera, { position: Vector3.create(8, 3, 2) })
  VirtualCamera.create(camera, {
    fov,
    defaultTransition: { transitionMode: VirtualCamera.Transition.Time(time) }
  })
  MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: camera })
  return camera
}

function releaseVirtualCamera(): void {
  MainCamera.createOrReplace(engine.CameraEntity, { virtualCameraEntity: undefined })
}

test('virtual-camera: fov is applied, eased in the transition and restored on release', async function (context) {
  customAddEntity.clean()
  await assertMovePlayerTo(context, Vector3.create(8, 0, 8), Vector3.create(8, 1, 16))
  const playerFov = await cameraFov()

  const camera = useVirtualCamera(14)
  const zoomIn = await fovBecomes(context, 14)
  assertEquals(Math.round(zoomIn[zoomIn.length - 1]), 14, 'the virtual camera reaches fov 14')
  const eased = zoomIn.some((fov) => fov < playerFov - 1 && fov > 15)
  assertEquals(eased, true, `the fov is interpolated during the transition (${zoomIn.join(', ')})`)

  VirtualCamera.getMutable(camera).fov = 90
  const changed = await fovBecomes(context, 90)
  assertEquals(Math.round(changed[changed.length - 1]), 90, 'a fov change on the active camera applies')

  releaseVirtualCamera()
  const released = await fovBecomes(context, playerFov)
  assertEquals(Math.round(released[released.length - 1]), Math.round(playerFov), 'release restores the player fov')
})

test('virtual-camera: no fov means 60', async function (context) {
  customAddEntity.clean()
  useVirtualCamera(undefined, 0.1)
  const seen = await fovBecomes(context, 60)
  assertEquals(Math.round(seen[seen.length - 1]), 60, 'a virtual camera without fov uses 60')
  // Delete the camera's entity only once the view is back on the player camera.
  releaseVirtualCamera()
  await context.helpers.waitNTicks(60)
  customAddEntity.clean()
})
