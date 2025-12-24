import {
  Transform,
  engine,
  type Entity,
  type TransformTypeWithOptionals
} from '@dcl/sdk/ecs'
import type { Vector3 } from '@dcl/sdk/math'
import { movePlayerTo } from '~system/RestrictedActions'
import { type TestFunctionContext } from '../testing/types'
import { customAddEntity } from './entity'

export async function assertMovePlayerTo(
  ctx: TestFunctionContext,
  newRelativePosition: Vector3,
  cameraTarget: Vector3
): Promise<void> {
  await movePlayerTo({
    newRelativePosition,
    cameraTarget
  })
  await ctx.helpers.waitNTicks(1)
}

export function createChainedEntities(
  transforms: Array<Omit<TransformTypeWithOptionals, 'parent'>>,
  parent: Entity = engine.RootEntity
): Entity {
  return transforms.reduce((parent, transform) => {
    const entity = customAddEntity.addEntity()
    Transform.create(entity, { ...transform, parent })
    return entity
  }, parent)
}

export function withInterval(
  seconds: number,
  callback: () => void
): (dt: number) => void {
  let t = 0
  return function (dt: number) {
    t += dt
    if (t > seconds) {
      t = 0
      callback()
    }
  }
}
