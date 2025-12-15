import { UiCanvasInformation, engine } from '@dcl/sdk/ecs'
import { type Vector2 } from '~system/RestrictedActions'

export function getScreenCanvasInfo(): Vector2 {
  const canvasInfo = UiCanvasInformation.get(engine.RootEntity)
  return { x: canvasInfo.width, y: canvasInfo.height }
}
