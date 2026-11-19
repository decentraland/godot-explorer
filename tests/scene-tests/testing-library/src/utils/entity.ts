import type { Entity } from '@dcl/sdk/ecs'
import { engine } from '@dcl/sdk/ecs'

export function lazyCreateEntity(): {
  get: () => Entity
} {
  let myEntity = engine.RootEntity

  function addSystem(): void {
    myEntity = engine.addEntity()
    engine.removeSystem(addSystem)
  }

  engine.addSystem(addSystem)

  return {
    get() {
      return myEntity
    }
  }
}

function createAddEntityFunction(): {
  addEntity: () => Entity
  clean: () => void
  isEmpty: () => boolean
  entities: () => Entity[]
} {
  let arr: Entity[] = []

  return {
    addEntity() {
      const newEntity = engine.addEntity()
      arr.push(newEntity)
      return newEntity
    },
    clean() {
      for (const entity of arr) {
        engine.removeEntity(entity)
      }
      arr = []
    },
    isEmpty() {
      return arr.length === 0
    },
    entities() {
      return arr
    }
  }
}

export const customAddEntity = createAddEntityFunction()
