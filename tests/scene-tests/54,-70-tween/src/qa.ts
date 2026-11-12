import { Material, MeshRenderer, Transform, engine } from '@dcl/sdk/ecs'
import { Color4, Vector3 } from '@dcl/sdk/math'
import { isQaMode } from 'testing-library/src/qa/panel'
import { spin, tilt } from './spin'

/** In QA mode, the issue's coin in front of the spawn point. */
export function showCoin(): void {
  if (!isQaMode) return
  const coin = engine.addEntity()
  Transform.create(coin, { position: Vector3.create(8, 2, 12), scale: Vector3.create(1.5, 0.1, 1.5), rotation: tilt })
  MeshRenderer.setCylinder(coin)
  Material.setPbrMaterial(coin, { albedoColor: Color4.create(1, 0.8, 0.2, 1) })
  spin(coin)
}
