import type { IEngine, PointerEventsSystem } from '@dcl/ecs'

import { createReconciler } from '@dcl/react-ecs/dist/reconciler'
import { engine, pointerEventsSystem } from '@dcl/sdk/ecs'
import { type UiComponent } from '@dcl/sdk/react-ecs'

export function createReactBasedUiSystem(
  engine: IEngine,
  pointerSystem: PointerEventsSystem
): any {
  // This any is temporal, I need be more specific about this function type
  let renderer: ReturnType<typeof createReconciler> | undefined =
    createReconciler(engine, pointerSystem)
  let uiComponent: UiComponent | undefined

  function ReactBasedUiSystem(): void {
    if (renderer != null && uiComponent != null) {
      renderer.update(uiComponent())
    }
  }

  engine.addSystem(ReactBasedUiSystem, 100e3, '@dcl/react-ecs')

  return {
    destroy(): void {
      if (renderer === undefined) return

      for (const entity of renderer.getEntities()) {
        engine.removeEntity(entity)
      }
      renderer = undefined
    },
    setUiRenderer(ui: UiComponent): void {
      if (renderer === undefined) {
        renderer = createReconciler(engine, pointerSystem)
      }

      uiComponent = ui
    }
  }
}

export const CustomReactEcsRenderer = createReactBasedUiSystem(
  engine,
  pointerEventsSystem
)
