import type { CameraType, Entity } from '@dcl/sdk/ecs'
import {
  CameraModeArea,
  Material,
  MeshRenderer,
  TextShape,
  Transform
} from '@dcl/sdk/ecs'
import type { Color4 } from '@dcl/sdk/math'
import { Quaternion, Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from './entity'

export function createAreaMode(
  position: Vector3,
  rotationDegrees: number,
  areaRotationDegrees: number,
  text: string,
  subname: string,
  mode: CameraType,
  floorColor: Color4,
  areaColor: Color4,
  areaScale: Vector3
): Record<string, Entity> {
  const obj: Record<string, Entity> = {}
  // create center to manipulate entities
  obj['center' + subname] = customAddEntity.addEntity()
  Transform.create(obj['center' + subname], {
    position,
    rotation: Quaternion.fromAngleAxis(rotationDegrees, Vector3.Up())
  })

  obj['child' + subname] = customAddEntity.addEntity()
  Transform.create(obj['child' + subname], {
    parent: obj['center' + subname],
    rotation: Quaternion.fromAngleAxis(-90, Vector3.Left())
  })

  obj['floor' + subname] = customAddEntity.addEntity()
  MeshRenderer.setPlane(obj['floor' + subname])
  Material.setPbrMaterial(obj['floor' + subname], { albedoColor: floorColor })
  Transform.create(obj['floor' + subname], {
    parent: obj['child' + subname],
    scale: Vector3.create(6, 2, 1)
  })

  obj['text' + subname] = customAddEntity.addEntity()
  Transform.create(obj['text' + subname], {
    parent: obj['child' + subname],
    position: Vector3.create(0, 0, -0.01)
  })
  TextShape.create(obj['text' + subname], { text, fontSize: 8 })

  // These entities declare and show the real area mode
  obj['cameraMode' + subname] = customAddEntity.addEntity()
  Transform.create(obj['cameraMode' + subname], {
    parent: obj['center' + subname],
    position: Vector3.create(0, 2, 0),
    rotation: Quaternion.fromAngleAxis(areaRotationDegrees, Vector3.Up()),
    scale: areaScale
  })
  CameraModeArea.create(obj['cameraMode' + subname], {
    area: Vector3.create(6, 4, 2),
    mode
  })

  obj['area' + subname] = customAddEntity.addEntity()
  Transform.create(obj['area' + subname], {
    parent: obj['cameraMode' + subname],
    scale: Vector3.create(6, 4, 2)
  })
  MeshRenderer.setBox(obj['area' + subname])
  Material.setPbrMaterial(obj['area' + subname], { albedoColor: areaColor })

  return obj
}
