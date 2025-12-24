import {
  Material,
  MaterialTransparencyMode,
  MeshRenderer,
  Transform,
  executeTask
} from '@dcl/sdk/ecs'
import { Color4, Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { test } from 'testing-library/src/testing'
import { getUserData } from '~system/UserIdentity'

test('material: blue emissiveIntensity:100: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    albedoColor: Color4.Blue(),
    emissiveColor: Color4.Blue(),
    emissiveIntensity: 100
  })
  await context.helpers.waitNTicks(5)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_01.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: blue with alpha if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    metallic: 0,
    roughness: 0,
    alphaTest: 0.5,
    albedoColor: Color4.Blue()
  })

  await context.helpers.waitNTicks(2)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_02.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: blue with texture if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    metallic: 0.5,
    roughness: 0.2,
    alphaTest: 1,
    bumpTexture: {
      tex: {
        $case: 'texture',
        texture: {
          src: 'src/src/assets/images/normal_mapping_normal_map.png'
        }
      }
    },
    albedoColor: Color4.Blue()
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_03.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: blue with metallic:0 roghness:1 alphaTest:1: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    metallic: 0,
    roughness: 1,
    alphaTest: 1,
    albedoColor: Color4.Blue()
  })

  await context.helpers.waitNTicks(2)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_04.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: blue with metallic:0.5 roghness:0.5 alphaTest:1: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    metallic: 0.5,
    roughness: 0.5,
    alphaTest: 1,
    albedoColor: Color4.Blue()
  })

  await context.helpers.waitNTicks(2)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_05.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: blue with metallic:0 roghness:0 alphaTest:1: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    metallic: 0,
    roughness: 0,
    alphaTest: 1,
    albedoColor: Color4.Blue()
  })

  await context.helpers.waitNTicks(2)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_06.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: blue with metallic:1 roghness:0 alphaTest:1: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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
  Material.setPbrMaterial(cube, {
    metallic: 1,
    roughness: 0,
    alphaTest: 1,
    albedoColor: Color4.Blue()
  })

  await context.helpers.waitNTicks(2)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_07.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: uv checker if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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

  Material.setPbrMaterial(cube, {
    texture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/uv-checker.png' }
      }
    }
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_08.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: uv checker with transparency mode auto: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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

  Material.setPbrMaterial(cube, {
    alphaTexture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/transparency-texture.png' }
      }
    },
    texture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/uv-checker.png' }
      }
    },
    transparencyMode: MaterialTransparencyMode.MTM_AUTO
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_09.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: uv checker with transparency mode blend: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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

  Material.setPbrMaterial(cube, {
    alphaTexture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/transparency-texture.png' }
      }
    },
    texture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/uv-checker.png' }
      }
    },
    transparencyMode: MaterialTransparencyMode.MTM_ALPHA_TEST_AND_ALPHA_BLEND,
    castShadows: false
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_10.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: transparency mode auto with emissive: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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

  Material.setPbrMaterial(cube, {
    alphaTexture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/transparency-texture.png' }
      }
    },
    transparencyMode: MaterialTransparencyMode.MTM_AUTO,
    emissiveTexture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/emissive-texture.png' }
      }
    },
    emissiveColor: Color4.Yellow(),
    emissiveIntensity: 150
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_11.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: rock wall texture if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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

  Material.setPbrMaterial(cube, {
    texture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/rock-wall-texture.png' }
      }
    }
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_12.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: rock wall texture with bump texture: if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
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

  Material.setPbrMaterial(cube, {
    texture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/rock-wall-texture.png' }
      }
    },
    bumpTexture: {
      tex: {
        $case: 'texture',
        texture: { src: 'src/assets/images/rock-wall-bump.png' }
      }
    }
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_13.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})

test('material: avatar portrait', async function (context) {
  customAddEntity.clean()

  executeTask(async () => {
    const userData = await getUserData({})
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

    if (userData.data !== null) {
      Material.setPbrMaterial(plane, {
        texture: Material.Texture.Avatar({
          userId: '0xc09cc22c8f5cf3fb1edbf0b42da2cff70990908b'
        })
      })
    }
  })

  // TODO: should be able to know when the texture is loaded
  await context.helpers.waitNTicks(100)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_material_14.png',
    Vector3.create(6, 4, 6),
    Vector3.create(8, 1, 8)
  )
})
