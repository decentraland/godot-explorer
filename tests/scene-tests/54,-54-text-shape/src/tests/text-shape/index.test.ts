import {
  EngineInfo,
  Font,
  TextAlignMode,
  TextShape,
  Transform,
  engine
} from '@dcl/sdk/ecs'
import { Color4, Vector3 } from '@dcl/sdk/math'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { test } from 'testing-library/src/testing'

test('text-shape: default text - if exist a reference snapshot should match with it', async function (context) {
  await context.helpers.waitTicksUntil(() => {
    const tickNumber = EngineInfo.getOrNull(engine.RootEntity)?.tickNumber ?? 0
    if (tickNumber > 100) {
      return true
    } else {
      return false
    }
  })
  customAddEntity.clean()
  const textEntity = customAddEntity.addEntity()
  Transform.create(textEntity, {
    position: Vector3.create(8, 1, 8)
  })
  TextShape.createOrReplace(textEntity, { text: 'Default text ' })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_text_shape_1.png',
    Vector3.create(8, 1, 7),
    Vector3.create(8, 1, 8)
  )
})

test('text-shape: colorized - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const textEntity = customAddEntity.addEntity()
  Transform.create(textEntity, {
    position: Vector3.create(8, 1, 8)
  })
  TextShape.createOrReplace(textEntity, {
    text: 'Red text',
    fontSize: 3,
    textColor: Color4.Red()
  })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_text_shape_2.png',
    Vector3.create(8, 1, 7),
    Vector3.create(8, 1, 8)
  )
})

test('text-shape: outlined - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const textEntity = customAddEntity.addEntity()
  Transform.create(textEntity, {
    position: Vector3.create(8, 1, 8)
  })
  /* The text outline works correctly, but the default value of the property is lost (each iteration is affected by the value set here)  */
  TextShape.createOrReplace(textEntity, {
    text: 'Text with\nred outline',
    fontSize: 2,
    outlineColor: Color4.Red(),
    outlineWidth: 0.1
  })
  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_text_shape_3.png',
    Vector3.create(8, 1, 7),
    Vector3.create(8, 1, 8)
  )
})

test('text-shape: changed font - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const textEntity = customAddEntity.addEntity()
  Transform.create(textEntity, {
    position: Vector3.create(8, 1, 8)
  })
  /* Text font don't change  */
  TextShape.createOrReplace(textEntity, {
    text: 'Monospace',
    fontSize: 2,
    font: Font.F_MONOSPACE
  })

  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_text_shape_4.png',
    Vector3.create(8, 1, 7),
    Vector3.create(8, 1, 8)
  )
})

test('text-shape: align 1 - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const textEntity = customAddEntity.addEntity()
  Transform.create(textEntity, {
    position: Vector3.create(8, 1, 8)
  })
  /* Text alignment has inverse behavior  */
  TextShape.createOrReplace(textEntity, {
    text: 'Bottom Center',
    fontSize: 2,
    textAlign: TextAlignMode.TAM_BOTTOM_CENTER
  })
  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_text_shape_5.png',
    Vector3.create(8, 1, 7),
    Vector3.create(8, 1, 8)
  )
})

test('text-shape: align 2 - if exist a reference snapshot should match with it', async function (context) {
  customAddEntity.clean()
  const textEntity = customAddEntity.addEntity()
  Transform.create(textEntity, {
    position: Vector3.create(8, 1, 8)
  })
  /* Text alignment has inverse behavior  */
  TextShape.createOrReplace(textEntity, {
    text: 'Middle Right',
    fontSize: 2,
    textAlign: TextAlignMode.TAM_MIDDLE_RIGHT
  })
  await context.helpers.waitNTicks(1)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_text_shape_6.png',
    Vector3.create(8, 1, 7),
    Vector3.create(8, 1, 8)
  )
})
