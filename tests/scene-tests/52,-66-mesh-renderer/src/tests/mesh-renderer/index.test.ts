import { Entity, MeshRenderer, PBMeshRenderer, Transform } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { renderedMesh, renderedNode, settled } from 'testing-library/src/utils/engine'
import { test } from 'testing-library/src/testing'
import { assertEquals } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'

// Sizes are in the entity's own space, before its Transform scale.
async function expectMesh(
  context: TestFunctionContext,
  mesh: PBMeshRenderer['mesh'],
  kind: string,
  size: [number, number, number]
): Promise<Entity> {
  customAddEntity.clean()
  const entity = customAddEntity.addEntity()
  Transform.create(entity, { position: Vector3.create(8, 2, 8), scale: Vector3.create(2, 2, 2) })
  MeshRenderer.create(entity, { mesh })
  const rendered = await settled(context, () => renderedMesh(entity), (m) => m?.kind === kind)
  assertEquals(rendered, { kind, visible: true }, `${mesh?.$case} mesh`)
  assertEquals((await renderedNode(entity)).bounds?.size, size, `${mesh?.$case} size`)
  return entity
}

test('mesh-renderer: box', async function (context) {
  await expectMesh(context, { $case: 'box', box: { uvs: [] } }, 'ArrayMesh', [1, 1, 1])
})

test('mesh-renderer: sphere', async function (context) {
  await expectMesh(context, { $case: 'sphere', sphere: {} }, 'SphereMesh', [1, 1, 1])
})

test('mesh-renderer: cylinder with a narrower top', async function (context) {
  await expectMesh(context, { $case: 'cylinder', cylinder: { radiusBottom: 1, radiusTop: 0.5 } }, 'ArrayMesh', [2, 1, 2])
})

test('mesh-renderer: plane', async function (context) {
  await expectMesh(context, { $case: 'plane', plane: { uvs: [] } }, 'ArrayMesh', [1, 1, 0])
})

test('mesh-renderer: changing the shape replaces the mesh, removing it clears it', async function (context) {
  const entity = await expectMesh(context, { $case: 'box', box: { uvs: [] } }, 'ArrayMesh', [1, 1, 1])
  MeshRenderer.setSphere(entity)
  assertEquals((await settled(context, () => renderedMesh(entity), (m) => m?.kind === 'SphereMesh'))?.kind, 'SphereMesh')
  MeshRenderer.deleteFrom(entity)
  assertEquals(await settled(context, () => renderedMesh(entity), (m) => m === null), null, 'mesh after removal')
  customAddEntity.clean()
})
