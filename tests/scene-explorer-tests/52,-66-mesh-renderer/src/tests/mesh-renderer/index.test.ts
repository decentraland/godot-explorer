import { MeshRenderer, Transform } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { test } from 'testing-library/src/testing'

test('mesh-renderer clean previous tests', async function (context) {
  customAddEntity.clean()
  await context.helpers.waitNTicks(100)
})

test('mesh-renderer: box - if exist a reference snapshot should match with it', async function (context) {
  const cube = customAddEntity.addEntity()
  Transform.create(cube, {
    position: Vector3.create(8, 2, 8),
    scale: Vector3.create(2, 2, 2)
  })
  MeshRenderer.create(cube, {
    mesh: {
      $case: 'box',
      box: { uvs: [] }
    }
  })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_mesh_renderer_box.png',
    Vector3.create(6, 3.5, 6),
    Vector3.create(8, 1.75, 8)
  )
})

test('mesh-renderer: sphere - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const sphere = customAddEntity.addEntity()
  Transform.create(sphere, {
    position: Vector3.create(8, 2, 8),
    scale: Vector3.create(2, 2, 2)
  })
  MeshRenderer.create(sphere, {
    mesh: {
      $case: 'sphere',
      sphere: { uvs: [] }
    }
  })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_mesh_renderer_sphere.png',
    Vector3.create(7, 3.5, 7),
    Vector3.create(8, 1.75, 8)
  )
})

test('mesh-renderer: cylinder - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const cylinder = customAddEntity.addEntity()
  Transform.create(cylinder, {
    position: Vector3.create(8, 2, 8),
    scale: Vector3.create(2, 2, 2)
  })
  MeshRenderer.create(cylinder, {
    mesh: {
      $case: 'cylinder',
      cylinder: { radiusBottom: 1, radiusTop: 0.5 }
    }
  })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_mesh_renderer_cylinder.png',
    Vector3.create(5, 3.5, 5),
    Vector3.create(8, 2, 8)
  )
})

test('mesh-renderer: plane - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const plane = customAddEntity.addEntity()
  Transform.create(plane, {
    position: Vector3.create(8, 2, 8),
    scale: Vector3.create(2, 2, 2)
  })
  MeshRenderer.create(plane, {
    mesh: {
      $case: 'plane',
      plane: { uvs: [] }
    }
  })

  await context.helpers.waitNTicks(2)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_mesh_renderer_plane.png',
    Vector3.create(6.5, 3.5, 5.5),
    Vector3.create(8, 2, 8)
  )
  customAddEntity.clean()
})
