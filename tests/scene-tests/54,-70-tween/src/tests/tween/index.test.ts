import { MeshRenderer, Transform, engine } from '@dcl/sdk/ecs'
import { Quaternion, Vector3 } from '@dcl/sdk/math'
import { test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'
import { SEGMENT_MS, spin, tilt } from '../../spin'

/** The coin's spin about its local Z, in degrees 0..360, from the rotation the engine reports. */
function spinAngle(rotation: Quaternion): number {
  const local = Quaternion.multiply(Quaternion.create(-tilt.x, -tilt.y, -tilt.z, tilt.w), rotation)
  const sign = local.w < 0 ? -1 : 1
  const degrees = (2 * Math.atan2(sign * local.z, sign * local.w) * 180) / Math.PI
  return (degrees + 360) % 360
}

// #2320: on mobile the coin reached ~350° and then snapped backward before the next turn, because
// the last segment (270° -> 360°) was interpolated the long way around.
test('tween: a TL_RESTART sequence of 90° rotations always turns forward', async function (context) {
  const coin = engine.addEntity()
  Transform.create(coin, { position: Vector3.create(8, 1.5, 8), rotation: tilt })
  MeshRenderer.setCylinder(coin)
  spin(coin)
  try {
    // Tweens run on the wall clock, which the fixed-step test run outpaces: sample every tick until
    // the coin has turned a turn and a quarter (every seam, including 270° -> 360° -> 0°), five
    // segments, allowing three times that.
    const deadline = Date.now() + 3 * 5 * SEGMENT_MS
    let previous = spinAngle(Transform.get(coin).rotation)
    let turned = 0
    for (let tick = 0; Date.now() < deadline && turned < 450; tick++) {
      await context.helpers.waitNTicks(1)
      const angle = spinAngle(Transform.get(coin).rotation)
      // Forward is +; crossing 360° -> 0° is forward too.
      const delta = ((angle - previous + 540) % 360) - 180
      assert(delta >= 0, `the coin turned back ${(-delta).toFixed(1)}° at ${previous.toFixed(1)}° (tick ${tick})`)
      turned += delta
      previous = angle
    }
    assert(turned >= 450, `the coin turned ${turned.toFixed(0)}°, expected a turn and a quarter (450°)`)
  } finally {
    engine.removeEntity(coin)
  }
})
