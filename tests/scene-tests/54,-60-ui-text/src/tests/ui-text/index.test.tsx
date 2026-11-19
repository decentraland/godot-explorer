import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'
import { test } from 'testing-library/src/testing'
import ReactEcs, { Label } from '@dcl/sdk/react-ecs'
import { Color4 } from '@dcl/sdk/math'
import { Entity, UiBackground, UiText, engine } from '@dcl/sdk/ecs'
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
