import { test } from 'testing-library/src/testing'
import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'

import { Color4, Vector3 } from '@dcl/sdk/math'
import type { JSX } from '@dcl/sdk/react-ecs'
import ReactEcs, { UiEntity } from '@dcl/sdk/react-ecs'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { MainCanvas, UiItem } from 'testing-library/src/utils/ui/item'
import { getScreenCanvasInfo } from 'testing-library/src/utils/ui/ui-utils'

function TestElementGreen(): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiTransform={{
        width: 512,
        height: 512
      }}
      uiBackground={{
        color: Color4.Green()
      }}
    ></UiEntity>
  )
}

function TestElementRed(): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiTransform={{
        width: 512,
        height: 512
      }}
      uiBackground={{
        color: Color4.Red()
      }}
    ></UiEntity>
  )
}

function TestElementRocks(): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiTransform={{
        width: 512,
        height: 512
      }}
      uiBackground={{
        textureMode: 'stretch',
        texture: {
          src: 'src/assets/images/rock-wall-texture.png',
          wrapMode: 'repeat'
        }
      }}
    ></UiEntity>
  )
}

function FlexBoxTest(): JSX.Element {
  return (
    <MainCanvas>
      <UiItem
        flexDirection={'column'}
        color={Color4.Blue()}
        width="50%"
        height="50%"
        position={{ top: '25%', left: '25%' }}
      >
        <UiItem flexGrow={2}>
          <UiItem color={Color4.Yellow()} width={'25%'} />
          <UiItem color={Color4.White()} width={'25%'} />
          <UiItem color={Color4.Black()} width={'25%'} />
        </UiItem>
        <UiItem flexGrow={1}>
          <UiItem color={Color4.Red()} width={'25%'} />
          <UiItem color={Color4.Purple()} flexGrow={1} />
          <UiItem color={Color4.Green()} width={'25%'} />
        </UiItem>
        <UiItem flexGrow={1}>
          <UiItem color={Color4.Purple()} width={'5%'} />
          <UiItem color={Color4.Green()} flexGrow={1} />
          <UiItem color={Color4.White()} flexGrow={4} />
        </UiItem>
        <UiItem color={Color4.Gray()} flexGrow={5}>
          <UiItem flexGrow={1} flexDirection={'column'}>
            <UiItem
              color={Color4.Yellow()}
              height={'10%'}
              justifyContent="center"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Red()}
              height={'10%'}
              justifyContent="flex-end"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Yellow()}
              height={'10%'}
              justifyContent="flex-start"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Red()}
              height={'10%'}
              justifyContent="space-around"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Yellow()}
              height={'10%'}
              justifyContent="space-between"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Red()}
              height={'10%'}
              justifyContent="space-evenly"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Yellow()}
              height={'10%'}
              justifyContent="center"
              alignItems="center"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Red()}
              height={'10%'}
              justifyContent="center"
              alignItems="flex-end"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
            <UiItem
              color={Color4.Yellow()}
              height={'10%'}
              justifyContent="center"
              alignItems="stretch"
            >
              <UiItem color={Color4.Green()} width={'10%'} height={'40%'} />
              <UiItem color={Color4.Black()} width={'10%'} height={'80%'} />
              <UiItem color={Color4.Purple()} width={'10%'} height={'60%'} />
            </UiItem>
          </UiItem>
        </UiItem>
      </UiItem>
    </MainCanvas>
  )
}

const nineSlicesTextureSource = 'src/assets/images/9slice.png'

const backgroundTextureTests = [
  {
    description: 'Stretch',
    value: (
      <UiItem
        texture={{
          src: nineSlicesTextureSource,
          wrapMode: 'repeat',
          filterMode: 'bi-linear'
        }}
        textureMode="stretch"
        width={'100%'}
        height={'100%'}
      />
    )
  },
  {
    description: 'Stretch with colored',
    value: (
      <UiItem
        texture={{
          src: nineSlicesTextureSource,
          wrapMode: 'repeat',
          filterMode: 'bi-linear'
        }}
        textureMode="stretch"
        width={'100%'}
        height={'100%'}
        color={Color4.Blue()}
      />
    )
  },
  {
    description: 'Stretch with uvs',
    value: (
      <UiItem
        texture={{
          src: nineSlicesTextureSource,
          wrapMode: 'repeat',
          filterMode: 'bi-linear'
        }}
        textureMode="stretch"
        width={'100%'}
        height={'100%'}
        uvs={[0, 0, 0.5, 0.5]}
      />
    )
  },
  {
    description: 'Using avatar texture',
    value: (
      <UiItem
        avatarTexture={{
          userId: '0x83f9192d59b393c8789b55d446e5d4a77075c820'
        }}
        color={Color4.create(1, 1, 1, 0.5)}
        uvs={[0, 0, 1, 1]}
        width={'100%'}
        height={'100%'}
      />
    ),
    // With 10 seconds we ensure the texture is loaded
    delay: 10
  },
  {
    description: 'NineSlices with default textureSlices',
    value: (
      <UiItem
        texture={{
          src: nineSlicesTextureSource,
          wrapMode: 'repeat',
          filterMode: 'bi-linear'
        }}
        textureMode="nine-slices"
        width={'100%'}
        height={'100%'}
      />
    )
  },
  {
    description: 'NineSlices with right values',
    value: (
      <UiItem
        texture={{
          src: nineSlicesTextureSource,
          wrapMode: 'repeat',
          filterMode: 'bi-linear'
        }}
        textureMode="nine-slices"
        textureSlices={{
          top: 115 / 256.0,
          right: 115 / 256.0,
          bottom: 115 / 256.0,
          left: 115 / 256.0
        }}
        width={'100%'}
        height={'100%'}
      />
    )
  },
  {
    description: 'Smaller NineSlices with right values',
    value: (
      <UiItem
        texture={{
          src: nineSlicesTextureSource,
          wrapMode: 'repeat',
          filterMode: 'bi-linear'
        }}
        textureMode="nine-slices"
        textureSlices={{
          top: 115 / 256.0,
          right: 115 / 256.0,
          bottom: 115 / 256.0,
          left: 115 / 256.0
        }}
        width={'50%'}
        height={'50%'}
      />
    )
  }
]

test('ui-brackground: should render the entire screen green', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(TestElementGreen)
  await context.helpers.waitNTicks(10)
  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_background_all_screen_green.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )
})

test('ui-brackground: should render the entire screen red', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(TestElementRed)
  await context.helpers.waitNTicks(10)
  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_background_all_screen_red.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )
})

test('ui-brackground: should render the entire screen rocks', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(TestElementRocks)
  await context.helpers.waitNTicks(10)
  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_background_all_screen_rocks.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )
})

test('ui-brackground: should render different flexbox property with colors', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(FlexBoxTest)
  await context.helpers.waitNTicks(10)
  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_background_test_flexbox.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )
})

backgroundTextureTests.forEach((item) => {
  test(
    'ui-brackground: shoud test texture -> ' + item.description,
    async function (context) {
      const snapshotId = item.description.replace(/ /g, '_').toLocaleLowerCase()
      CustomReactEcsRenderer.destroy()
      CustomReactEcsRenderer.setUiRenderer(() => (
        <MainCanvas>
          <UiItem
            flexDirection={'column'}
            color={Color4.Blue()}
            width="50%"
            height="50%"
            position={{ top: '25%', left: '25%' }}
          >
            {item.value}
          </UiItem>
        </MainCanvas>
      ))
      await context.helpers.waitNTicks(10)

      if (item.delay !== undefined) {
        await context.helpers.waitTicksUntil(() => false, item.delay * 1000)
      }

      await assertSnapshot(
        'screenshot/$explorer_snapshot_ui_background_' + snapshotId + '.png',
        Vector3.create(8, 1, 10),
        Vector3.create(8, 1, 8),
        getScreenCanvasInfo()
      )
    }
  )
})
