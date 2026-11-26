import {
  Material,
  MeshRenderer,
  Transform,
  VideoPlayer,
  engine
} from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { isQaMode } from 'testing-library/src/qa/panel'

/** In QA mode, the scene's clip looping on the wall in front of the spawn point. */
export function showVideoScreen(): void {
  if (!isQaMode) return
  const screen = engine.addEntity()
  Transform.create(screen, {
    position: Vector3.create(8, 4, 15),
    scale: Vector3.create(8, 4.5, 1)
  })
  MeshRenderer.setPlane(screen)
  VideoPlayer.create(screen, {
    src: 'src/assets/videos/dae-video-1.mp4',
    playing: true,
    loop: true
  })
  Material.setPbrMaterial(screen, {
    texture: Material.Texture.Video({ videoPlayerEntity: screen }),
    roughness: 1,
    specularIntensity: 0,
    metallic: 0
  })
}
