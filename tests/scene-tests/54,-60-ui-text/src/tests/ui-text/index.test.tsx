import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'
import { renderTest, skipTest, test } from 'testing-library/src/testing'
import ReactEcs, { Label } from '@dcl/sdk/react-ecs'
import { Color4 } from '@dcl/sdk/math'
import { Entity, UiBackground, UiCanvasInformation, UiText, engine } from '@dcl/sdk/ecs'
import { MainCanvas } from 'testing-library/src/utils/ui/item'
import { Rgba, renderedUi, sameColor, settled } from 'testing-library/src/utils/engine'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'

function TestElementText(): ReactEcs.JSX.Element {
  return (
    <MainCanvas>
      <Label value="This is a label" color={Color4.White()} fontSize={80} font="serif" textAlign="middle-center" />
    </MainCanvas>
  )
}

function TestElementTextBlue(): ReactEcs.JSX.Element {
  return (
    <MainCanvas color={Color4.Red()}>
      <Label value="This is a label" color={Color4.Blue()} fontSize={80} font="serif" textAlign="middle-center" />
    </MainCanvas>
  )
}

function withComponent(component: typeof UiText | typeof UiBackground): Entity[] {
  return Array.from(engine.getEntitiesWith(component), ([entity]) => entity)
}

function withComponentValue<T>(component: { get(entity: Entity): T }, entities: Entity[], match: (value: T) => boolean): Entity {
  const found = entities.find((entity) => match(component.get(entity)))
  assert(found !== undefined, 'no matching UI entity')
  return found!
}

async function render(context: TestFunctionContext, element: () => ReactEcs.JSX.Element) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(element)
  await context.helpers.waitNTicks(10)
  const label = withComponentValue(UiText, withComponent(UiText), (text) => text.value === 'This is a label')
  return await settled(context, () => renderedUi(label), (ui) => ui.text !== null)
}

async function expectLabel(ui: Awaited<ReturnType<typeof renderedUi>>, color: Rgba): Promise<void> {
  assertEquals(ui.text?.text, 'This is a label', 'text')
  assert(sameColor(ui.text?.color, color), `color ${JSON.stringify(ui.text?.color)}`)
  assertEquals(ui.text?.fontSize, 80, 'font size')
  assert(ui.text?.font?.includes('NotoSerif') === true, `font ${ui.text?.font}`)
  assert(ui.visible && ui.size[0] > 0 && ui.size[1] > 0, `label rect ${JSON.stringify(ui.size)}`)
}

test('ui-text: white serif label', async function (context) {
  await expectLabel(await render(context, TestElementText), [1, 1, 1, 1])
})

test('ui-text: blue label on a red canvas', async function (context) {
  await expectLabel(await render(context, TestElementTextBlue), [0, 0, 1, 1])
  const canvas = withComponentValue(UiBackground, withComponent(UiBackground), () => true)
  const background = (await renderedUi(canvas)).background
  assert(sameColor(background?.color, [1, 0, 0, 1]), `canvas ${JSON.stringify(background?.color)}`)
  CustomReactEcsRenderer.destroy()
})

// #2638: the scene reads its canvas and the safe area (`interactableArea`: the screen minus notches
// and system bars) from UiCanvasInformation. Values depend on the device; what must hold is that
// they are there and the canvas is the size the UI is laid out in.
test('ui-text: the canvas information gives the size the UI is laid out in', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(() => <MainCanvas color={Color4.Red()} />)
  await context.helpers.waitNTicks(10)
  const canvas = UiCanvasInformation.getOrNull(engine.RootEntity)
  assert(canvas !== null, 'no UiCanvasInformation on the root entity')
  assert(canvas!.width > 0 && canvas!.height > 0, `canvas ${canvas!.width} x ${canvas!.height}`)
  const root = withComponentValue(UiBackground, withComponent(UiBackground), () => true)
  const ui = await settled(context, () => renderedUi(root), (ui) => ui.size[0] > 0)
  const close = (a: number, b: number) => Math.abs(a - b) <= 2
  assert(close(ui.size[0], canvas!.width) && close(ui.size[1], canvas!.height), `the full-screen UI is ${ui.size[0]} x ${ui.size[1]}, the canvas ${canvas!.width} x ${canvas!.height}`)
  CustomReactEcsRenderer.destroy()
})

// Skipped until #3245 is fixed: before the app measures the safe area, interactableArea insets the
// whole canvas (the windowed run never measures it).
skipTest('ui-text: the canvas information gives a safe area inside the canvas', async function (context) {
  await context.helpers.waitNTicks(10)
  const canvas = UiCanvasInformation.getOrNull(engine.RootEntity)
  assert(canvas !== null, 'no UiCanvasInformation on the root entity')
  const inset = canvas!.interactableArea
  assert(inset !== undefined, 'no interactableArea')
  const sides = [inset!.top, inset!.right, inset!.bottom, inset!.left]
  assert(sides.every((side) => side >= 0), `negative inset ${JSON.stringify(inset)}`)
  assert(inset!.left + inset!.right < canvas!.width && inset!.top + inset!.bottom < canvas!.height, `inset ${JSON.stringify(inset)} leaves no area in ${canvas!.width} x ${canvas!.height}`)
})

// The pixel ratio is the window's size over the canvas's: only a run that draws has a window.
renderTest('ui-text: the canvas information gives a device pixel ratio', async function (context) {
  await context.helpers.waitNTicks(10)
  const canvas = UiCanvasInformation.getOrNull(engine.RootEntity)
  assert(canvas !== null, 'no UiCanvasInformation on the root entity')
  assert(canvas!.devicePixelRatio > 0, `device pixel ratio ${canvas!.devicePixelRatio}`)
})
