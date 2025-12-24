import {
  GltfContainer,
  GltfContainerLoadingState,
  LoadingState,
  Transform
} from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { test } from 'testing-library/src/testing'
import { assert } from 'testing-library/src/testing/assert'

test('gltf-container: avocado model loads correctly', async function (context) {
  customAddEntity.clean()
  const avocado = customAddEntity.addEntity()
  Transform.create(avocado, {
    position: Vector3.create(8, 0.25, 8)
  })
  GltfContainer.create(avocado, {
    src: 'src/assets/models/avocado.glb'
  })

  assert(
    await context.helpers.waitTicksUntil(() => {
      return (
        GltfContainerLoadingState.getOrNull(avocado) !== null &&
        GltfContainerLoadingState.get(avocado).currentState !==
          LoadingState.LOADING
      )
    }, 10000),
    'timeout waiting loading avocado'
  )

  assert(
    GltfContainerLoadingState.get(avocado).currentState ===
      LoadingState.FINISHED
  )

  // TODO: sometimes in godot the gltf is not added immediately it's loaded
  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_gltfcontainer_avocado.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8)
  )
})

test('gltf-container: H.E.V Mark IV model loads correctly', async function (context) {
  customAddEntity.clean()
  const hev = customAddEntity.addEntity()
  Transform.create(hev, {
    position: Vector3.create(8, 0, 8),
    scale: Vector3.create(0.03, 0.03, 0.03)
  })
  GltfContainer.create(hev, {
    src: 'src/assets/models/hevmarkiv.glb'
  })

  assert(
    await context.helpers.waitTicksUntil(() => {
      return (
        GltfContainerLoadingState.getOrNull(hev) !== null &&
        GltfContainerLoadingState.get(hev).currentState !== LoadingState.LOADING
      )
    }, 10000),
    'timeout waiting loading avocado'
  )

  assert(
    GltfContainerLoadingState.get(hev).currentState === LoadingState.FINISHED
  )

  // TODO: sometimes in godot the gltf is not added immediately it's loaded
  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_gltfcontainer_hev.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8)
  )
})
