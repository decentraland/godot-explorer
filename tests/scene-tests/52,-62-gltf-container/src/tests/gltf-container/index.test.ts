import { Entity, GltfContainer, GltfContainerLoadingState, LoadingState, Transform } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { renderedModel, renderedNode } from 'testing-library/src/utils/engine'
import { test } from 'testing-library/src/testing'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'

async function load(context: TestFunctionContext, src: string, scale = 1): Promise<Entity> {
  customAddEntity.clean()
  const entity = customAddEntity.addEntity()
  Transform.create(entity, { position: Vector3.create(8, 0.25, 8), scale: Vector3.create(scale, scale, scale) })
  GltfContainer.create(entity, { src })
  assert(
    await context.helpers.waitTicksUntil(() => {
      const state = GltfContainerLoadingState.getOrNull(entity)?.currentState
      return state !== undefined && state !== LoadingState.LOADING
    }, 10000),
    `timeout loading ${src}`
  )
  return entity
}

// The model's bounds in the entity's own space: its meshes were added, at their size.
async function expectLoaded(entity: Entity, src: string, size: [number, number, number]): Promise<void> {
  assertEquals(GltfContainerLoadingState.get(entity).currentState, LoadingState.FINISHED, 'SDK loading state')
  assertEquals(await renderedModel(entity), { src, state: LoadingState.FINISHED, visible: true }, 'engine model')
  const bounds = (await renderedNode(entity)).bounds
  assertEquals(bounds?.size, size, `${src} size`)
}

test('gltf-container: avocado model loads', async function (context) {
  const avocado = await load(context, 'src/assets/models/avocado.glb')
  await expectLoaded(avocado, 'src/assets/models/avocado.glb', [1.28, 1.89, 0.83])
})

test('gltf-container: H.E.V Mark IV model loads', async function (context) {
  const hev = await load(context, 'src/assets/models/hevmarkiv.glb', 0.03)
  await expectLoaded(hev, 'src/assets/models/hevmarkiv.glb', [29.1, 68.89, 15.32])
})

test('gltf-container: a missing model reports NOT_FOUND and draws nothing', async function (context) {
  const missing = await load(context, 'src/assets/models/missing.glb')
  assertEquals(GltfContainerLoadingState.get(missing).currentState, LoadingState.NOT_FOUND, 'SDK loading state')
  assertEquals((await renderedNode(missing)).bounds, null, 'bounds')
  customAddEntity.clean()
})
