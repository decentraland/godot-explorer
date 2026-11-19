import { Entity, MeshRenderer, Transform, VisibilityComponent } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { renderedMesh, settled } from 'testing-library/src/utils/engine'
import { test } from 'testing-library/src/testing'
import { assertEquals } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'

function cube(visible: boolean | undefined): Entity {
  const entity = customAddEntity.addEntity()
  Transform.create(entity, { position: Vector3.create(8, 1, 8) })
  MeshRenderer.setBox(entity)
  if (visible !== undefined) VisibilityComponent.create(entity, { visible })
  return entity
}

async function meshVisible(context: TestFunctionContext, entity: Entity, expected: boolean) {
  const mesh = await settled(context, () => renderedMesh(entity), (m) => m?.visible === expected)
  assertEquals(mesh?.visible, expected, 'mesh visible')
}

test('visibility: visible true draws the mesh', async function (context) {
  customAddEntity.clean()
  await meshVisible(context, cube(true), true)
})

test('visibility: visible false hides the mesh', async function (context) {
  customAddEntity.clean()
  await meshVisible(context, cube(false), false)
})

test('visibility: no component draws the mesh', async function (context) {
  customAddEntity.clean()
  await meshVisible(context, cube(undefined), true)
})

test('visibility: toggling it hides and shows the mesh again', async function (context) {
  customAddEntity.clean()
  const entity = cube(true)
  await meshVisible(context, entity, true)
  VisibilityComponent.getMutable(entity).visible = false
  await meshVisible(context, entity, false)
  VisibilityComponent.getMutable(entity).visible = true
  await meshVisible(context, entity, true)
  VisibilityComponent.deleteFrom(entity)
  await meshVisible(context, entity, true)
})
