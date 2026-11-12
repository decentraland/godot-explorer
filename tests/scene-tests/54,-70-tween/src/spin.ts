import { EasingFunction, Entity, Tween, TweenLoop, TweenSequence } from '@dcl/sdk/ecs'
import { Quaternion } from '@dcl/sdk/math'

// #2320: the coin from the issue, tilted 90° about X so its face points up, spun in 90° steps
// about its local Z, looped with TL_RESTART.
export const tilt = Quaternion.fromEulerDegrees(90, 0, 0)
const step = (degrees: number) => Quaternion.multiply(tilt, Quaternion.fromEulerDegrees(0, 0, degrees))
export const SEGMENT_MS = 500

export function spin(coin: Entity): void {
  const segment = (from: number, to: number) => ({
    mode: Tween.Mode.Rotate({ start: step(from), end: step(to) }),
    duration: SEGMENT_MS,
    easingFunction: EasingFunction.EF_LINEAR
  })
  Tween.createOrReplace(coin, { ...segment(0, 90), playing: true })
  TweenSequence.createOrReplace(coin, {
    sequence: [segment(90, 180), segment(180, 270), segment(270, 360)],
    loop: TweenLoop.TL_RESTART
  })
}
