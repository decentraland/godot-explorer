import { renderTest, skipTest, test } from 'testing-library/src/testing'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { getScreenCanvasInfo } from 'testing-library/src/utils/ui/ui-utils'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'
import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'

import { Color4, Vector3 } from '@dcl/sdk/math'
import type { JSX } from '@dcl/sdk/react-ecs'
import ReactEcs, { UiEntity } from '@dcl/sdk/react-ecs'
import { renderedUi, sameColor, settled } from 'testing-library/src/utils/engine'
import { Entity, UiBackground, UiTransform, engine } from '@dcl/sdk/ecs'
import { MainCanvas, UiItem } from 'testing-library/src/utils/ui/item'

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
        uvs={[0, 0, 0, 0.5, 0.5, 0.5, 0.5, 0]}
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

type Ui = Awaited<ReturnType<typeof renderedUi>>

function withBackground(match: (background: ReturnType<typeof UiBackground.get>, entity: Entity) => boolean): Entity {
  const found = Array.from(engine.getEntitiesWith(UiBackground), ([entity]) => entity as Entity).find((entity) =>
    match(UiBackground.get(entity), entity)
  )
  assert(found !== undefined, 'no matching UI entity')
  return found!
}

// React-ECS orders siblings through `rightOf`: the first child is right of entity 0.
function uiChildren(parent: Entity): Entity[] {
  const children = Array.from(engine.getEntitiesWith(UiTransform), ([entity, transform]) => [entity as Entity, transform] as const)
    .filter(([, transform]) => transform.parent === parent)
  const ordered: Entity[] = []
  let previous = 0
  for (let next = children.find(([, t]) => t.rightOf === previous); next !== undefined; next = children.find(([, t]) => t.rightOf === previous)) {
    ordered.push(next[0])
    previous = next[0]
  }
  return ordered
}

async function render(context: TestFunctionContext, element: () => JSX.Element): Promise<void> {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(element)
  await context.helpers.waitNTicks(10)
}

const isColor = (color: Color4) => (background: ReturnType<typeof UiBackground.get>) =>
  background.color !== undefined && sameColor([background.color.r, background.color.g, background.color.b, background.color.a], [color.r, color.g, color.b, color.a])

type Frame = { x: number; y: number; width: number; height: number }

const frameOf = (ui: Ui): Frame => ({ x: ui.globalPosition[0], y: ui.globalPosition[1], width: ui.size[0], height: ui.size[1] })

/**
 * `ui`'s rect against `expected` [x, y, width, height], given as fractions of `horizontal` (x,
 * width) and `vertical` (y, height): the layout without the viewport's size. Within 1.5 px, as
 * the layout rounds to whole pixels.
 */
function expectRect(name: string, ui: Ui, expected: number[], horizontal: Frame, vertical: Frame = horizontal): void {
  const want = [
    horizontal.x + expected[0] * horizontal.width,
    vertical.y + expected[1] * vertical.height,
    expected[2] * horizontal.width,
    expected[3] * vertical.height
  ]
  const actual = [ui.globalPosition[0], ui.globalPosition[1], ui.size[0], ui.size[1]]
  assert(
    actual.every((value, i) => Math.abs(value - want[i]) <= 1.5),
    `${name}: ${JSON.stringify(actual)} px != ${JSON.stringify(want.map((v) => Math.round(v * 10) / 10))} px`
  )
}

test('ui-background: green fills its 512 px box', async function (context) {
  await render(context, TestElementGreen)
  const ui = await renderedUi(withBackground(isColor(Color4.Green())))
  assertEquals(ui.size, [512, 512], 'size')
  assertEquals(ui.background?.texture, 'res://assets/white_pixel.png', 'texture')
})

test('ui-background: red fills its 512 px box', async function (context) {
  await render(context, TestElementRed)
  const ui = await renderedUi(withBackground(isColor(Color4.Red())))
  assertEquals(ui.size, [512, 512], 'size')
})

test('ui-background: a stretched texture loads', async function (context) {
  await render(context, TestElementRocks)
  const rocks = withBackground((background) => background.texture !== undefined)
  const ui = await settled(context, () => renderedUi(rocks), (u) => u.background?.texture === 'ImageTexture', 300)
  assertEquals(ui.background?.texture, 'ImageTexture', 'texture')
  assertEquals(ui.background?.stretch, [0, 0], 'stretch')
  assertEquals(ui.background?.margins, [0, 0, 0, 0], 'margins')
})

// Rows grow 2:1:1:5 inside the blue box; every row but the growing-sibling ones (rows 2 and 3,
// see the skipped test) is checked here. Child x and width are fractions of the box's width,
// child y and height fractions of their row.
const JUSTIFIED: Array<[string, number[]]> = [
  ['center', [0.35, 0.45, 0.55]],
  ['flex-end', [0.7, 0.8, 0.9]],
  ['flex-start', [0, 0.1, 0.2]],
  ['space-around', [0.7 / 6, 0.45, 1 - 0.7 / 6 - 0.1]],
  ['space-between', [0, 0.45, 0.9]],
  ['space-evenly', [0.175, 0.45, 0.725]],
  ['center, align center', [0.35, 0.45, 0.55]],
  ['center, align flex-end', [0.35, 0.45, 0.55]],
  ['center, align stretch', [0.35, 0.45, 0.55]]
]
const HEIGHTS = [0.4, 0.8, 0.6]

async function flexbox(context: TestFunctionContext): Promise<{ box: Ui; rows: Entity[] }> {
  await render(context, FlexBoxTest)
  const blue = withBackground(isColor(Color4.Blue()))
  return { box: await renderedUi(blue), rows: uiChildren(blue) }
}

test('ui-background: flexbox rows, percentages, justify and align', async function (context) {
  const { box, rows } = await flexbox(context)
  assertEquals(rows.length, 4, 'rows')
  const grow = [[0, 2], [2, 1], [3, 1], [4, 5]]
  for (const [index, row] of rows.entries()) {
    expectRect(`row ${index + 1}`, await renderedUi(row), [0, grow[index][0] / 9, 1, grow[index][1] / 9], frameOf(box))
  }
  const row1 = frameOf(await renderedUi(rows[0]))
  for (const [index, child] of uiChildren(rows[0]).entries()) {
    expectRect(`row 1 box ${index + 1}`, await renderedUi(child), [index * 0.25, 0, 0.25, 1], row1)
  }
  const justified = uiChildren(uiChildren(rows[3])[0])
  assertEquals(justified.length, JUSTIFIED.length, 'justified rows')
  for (const [index, row] of justified.entries()) {
    const [name, xs] = JUSTIFIED[index]
    const rowFrame = frameOf(await renderedUi(row))
    for (const [child, entity] of uiChildren(row).entries()) {
      const height = HEIGHTS[child]
      const y = name.endsWith('align center') ? (1 - height) / 2 : name.endsWith('align flex-end') ? 1 - height : 0
      expectRect(`${name} box ${child + 1}`, await renderedUi(entity), [xs[child], y, 0.1, height], frameOf(box), rowFrame)
    }
  }
})

// Skipped until the engine treats a NaN flexGrow as 0: the SDK sends `flexGrow: undefined` as NaN,
// and one NaN sibling leaves the growing box 0 px wide. Issue to be filed.
skipTest('ui-background: flexbox boxes grow next to their siblings', async function (context) {
  const { box, rows } = await flexbox(context)
  const expected = [
    [[0, 0.25], [0.25, 0.5], [0.75, 0.25]],
    [[0, 0.05], [0.05, 0.19], [0.24, 0.76]]
  ]
  for (const [row, boxes] of expected.entries()) {
    const rowFrame = frameOf(await renderedUi(rows[row + 1]))
    for (const [index, child] of uiChildren(rows[row + 1]).entries()) {
      expectRect(`row ${row + 2} box ${index + 1}`, await renderedUi(child), [boxes[index][0], 0, boxes[index][1], 1], frameOf(box), rowFrame)
    }
  }
})

// The same bug, reduced: a sibling with `flexGrow: undefined`.
skipTest('ui-background: flexGrow next to a sibling whose flexGrow is undefined', async function (context) {
  await render(context, () => (
    <UiEntity uiTransform={{ width: 800, height: 100, flexDirection: 'row' }}>
      <UiEntity uiTransform={{ width: 200, flexGrow: undefined }} uiBackground={{ color: Color4.Red() }} />
      <UiEntity uiTransform={{ flexGrow: 1 }} uiBackground={{ color: Color4.Purple() }} />
    </UiEntity>
  ))
  assertEquals((await renderedUi(withBackground(isColor(Color4.Purple())))).size, [600, 100], 'growing box')
})

async function texture(context: TestFunctionContext, description: string): Promise<{ box: Ui; ui: Ui }> {
  const item = backgroundTextureTests.find((test) => test.description === description)!
  await render(context, () => (
    <MainCanvas>
      <UiItem flexDirection={'column'} color={Color4.Blue()} width="50%" height="50%" position={{ top: '25%', left: '25%' }}>
        {item.value}
      </UiItem>
    </MainCanvas>
  ))
  // The blue frame, not the inner box (blue too in 'Stretch with colored').
  const blue = withBackground((background, entity) => isColor(Color4.Blue())(background) && uiChildren(entity).length > 0)
  const inner = uiChildren(blue)[0]
  // The avatar texture comes from the network.
  const ui = await settled(context, () => renderedUi(inner), (u) => u.background?.texture === 'ImageTexture', 900)
  assertEquals(ui.background?.texture, 'ImageTexture', `${description}: texture`)
  return { box: await renderedUi(blue), ui }
}

test('ui-background: texture stretch', async function (context) {
  const { box, ui } = await texture(context, 'Stretch')
  assertEquals(ui.size, box.size, 'size')
  assert(sameColor(ui.background?.color, [1, 1, 1, 1]), `color ${JSON.stringify(ui.background?.color)}`)
  assertEquals([ui.background?.stretch, ui.background?.margins, ui.background?.uvs], [[0, 0], [0, 0, 0, 0], null])
})

test('ui-background: texture stretch with colored', async function (context) {
  const { ui } = await texture(context, 'Stretch with colored')
  assert(sameColor(ui.background?.color, [0, 0, 1, 1]), `color ${JSON.stringify(ui.background?.color)}`)
})

test('ui-background: texture stretch with uvs', async function (context) {
  const { ui } = await texture(context, 'Stretch with uvs')
  assertEquals(ui.background?.uvs, [0, 0, 0, 0.5, 0.5, 0.5, 0.5, 0], 'uvs')
})

// Network: the portrait comes from the profile server, so it stays out of headless runs.
renderTest('ui-background: avatar texture', async function (context) {
  const { ui } = await texture(context, 'Using avatar texture')
  assert(sameColor(ui.background?.color, [1, 1, 1, 0.5]), `color ${JSON.stringify(ui.background?.color)}`)
})

// The 9slice texture is 256 px: default slices are a third (85 px), the custom ones 115/256.
test('ui-background: nine-slices with default slices', async function (context) {
  const { ui } = await texture(context, 'NineSlices with default textureSlices')
  assertEquals([ui.background?.margins, ui.background?.stretch], [[85, 85, 85, 85], [2, 2]])
})

test('ui-background: nine-slices with custom slices', async function (context) {
  const { ui } = await texture(context, 'NineSlices with right values')
  assertEquals([ui.background?.margins, ui.background?.stretch], [[115, 115, 115, 115], [2, 2]])
})

test('ui-background: smaller nine-slices keep their margins', async function (context) {
  const { box, ui } = await texture(context, 'Smaller NineSlices with right values')
  assertEquals(ui.background?.margins, [115, 115, 115, 115], 'margins')
  assert(ui.size.every((side, i) => Math.abs(side - box.size[i] / 2) <= 1), `size ${JSON.stringify(ui.size)} in ${JSON.stringify(box.size)}`)
  CustomReactEcsRenderer.destroy()
})

// The settings are checked above; these show the texture is sampled that way.
async function picture(name: string): Promise<void> {
  await assertSnapshot(
    `screenshot/$explorer_snapshot_ui_background_${name}.png`,
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )
}

renderTest('ui-background: stretched texture drawn', async function (context) {
  await render(context, TestElementRocks)
  const rocks = withBackground((background) => background.texture !== undefined)
  await settled(context, () => renderedUi(rocks), (u) => u.background?.texture === 'ImageTexture', 300)
  await picture('all_screen_rocks')
})

for (const [description, name] of [
  ['Stretch with uvs', 'stretch_with_uvs'],
  ['NineSlices with right values', 'nineslices_with_right_values'],
  ['Using avatar texture', 'using_avatar_texture']
]) {
  renderTest(`ui-background: ${description.toLowerCase()} drawn`, async function (context) {
    await texture(context, description)
    await picture(name)
    CustomReactEcsRenderer.destroy()
  })
}
