import {
  Transform,
  engine,
  type DeepReadonlyObject,
  type Entity,
  type TransformType
} from '@dcl/sdk/ecs'
import { Matrix } from '@dcl/sdk/math'

export function getGlobalTransform(entity: Entity): TransformType {
  const destTransform = Transform.schema.create()
  const transform = Transform.getOrNull(entity)
  // If the entity has no a transform, the Identity is resolved
  if (transform === null) return destTransform

  const matrixs: Matrix.ReadonlyMatrix[] = []
  if (transform !== null) {
    matrixs.push(
      Matrix.compose(transform.scale, transform.rotation, transform.position)
    )
  }

  let currentTransform: DeepReadonlyObject<TransformType> | null = transform
  while (
    currentTransform.parent !== null &&
    currentTransform.parent !== undefined &&
    currentTransform.parent !== engine.RootEntity
  ) {
    currentTransform = Transform.getOrNull(currentTransform.parent)
    if (currentTransform === null) break

    const currentMatrix = Matrix.compose(
      currentTransform.scale,
      currentTransform.rotation,
      currentTransform.position
    )
    matrixs.push(currentMatrix)
  }

  let globalMatrix = Matrix.Identity()
  for (const matrix of matrixs.reverse()) {
    globalMatrix = Matrix.multiply(matrix, globalMatrix)
  }

  Matrix.decompose(
    globalMatrix,
    destTransform.scale,
    destTransform.rotation,
    destTransform.position
  )
  return destTransform
}

// export function* dumpTree(entity: Entity, depth: number = 0): Generator {
//   yield '   '.repeat(Math.max(depth - 1, 0)) +
//     (depth ? '└──' : '') +
//     entity.toString(16).toUpperCase()
//   for (const child of getChildrenOf(entity)) {
//     yield* dumpTree(child, depth + 1)
//   }
// }

// function* getChildrenOf(parentEntity: Entity) {
//   for (const [childEntity, value] of engine.getEntitiesWith(Transform)) {
//     if (value.parent === parentEntity) yield childEntity
//   }
// }
