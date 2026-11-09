import { skipTest, test } from 'testing-library/src/testing'
import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'

import { Color4, Vector3 } from '@dcl/sdk/math'
import ReactEcs, { Dropdown, Label, UiEntity } from '@dcl/sdk/react-ecs'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { Material, UiDropdown, engine } from '@dcl/sdk/ecs'
import { MainCanvas } from 'testing-library/src/utils/ui/item'
import { renderedDropdown, settled } from 'testing-library/src/utils/engine'
import { assert, assertEquals } from 'testing-library/src/testing/assert'
import { TestFunctionContext } from 'testing-library/src/testing/types'

let clicked: boolean = false

function ChangeColor(index: number): void {
  console.log('dropdown clicked')
  let color: Color4 = Color4.Black()
  for (const [entity] of engine.getEntitiesWith(Material)) {
    switch (index) {
      case 0:
        color = Color4.Black()
        clicked = false
        break
      case 1:
        color = Color4.Red()
        clicked = true
        break
      case 2:
        color = Color4.Blue()
        clicked = true
        break
      case 3:
        color = Color4.Green()
        clicked = true
        break
    }
    Material.setPbrMaterial(entity, { albedoColor: color })
  }
}

function TestElementDropdown(): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiBackground={{
        color: Color4.Red()
      }}
      uiTransform={{
        position: { left: '0', top: '0' },
        padding: '15px',
        width: '100%',
        height: 'auto',
        alignContent: 'center',
        alignItems: 'center',
        flexDirection: 'column',
        alignSelf: 'center'
      }}
    >
      <Label
        value="Select a color:"
        fontSize={18}
        color={Color4.White()}
        uiTransform={{
          width: '100%',
          height: 'auto'
        }}
      />
      <Dropdown
        fontSize={18}
        color={Color4.White()}
        options={[`Black`, `Red`, `Blue`, `Green`]}
        onChange={ChangeColor}
        uiTransform={{
          width: '100px',
          height: '60'
        }}
      />
    </UiEntity>
  )
}

// What the scene sends; `selectedIndex: undefined` leaves the field unset.
let sent: { options: string[]; acceptEmpty?: boolean; emptyLabel?: string; selectedIndex?: number } = { options: [] }

function SentDropdown(): ReactEcs.JSX.Element {
  return (
    <MainCanvas>
      <Dropdown uiTransform={{ width: 300, height: 40 }} fontSize={18} {...sent} />
    </MainCanvas>
  )
}

type Shown = NonNullable<Awaited<ReturnType<typeof renderedDropdown>>>

async function show(
  context: TestFunctionContext,
  props: typeof sent,
  done: (shown: Shown) => boolean
): Promise<Shown> {
  sent = props
  CustomReactEcsRenderer.setUiRenderer(SentDropdown)
  await context.helpers.waitNTicks(5)
  const [entity] = Array.from(engine.getEntitiesWith(UiDropdown), ([entity]) => entity)
  assert(entity !== undefined, 'no UiDropdown entity')
  const shown = await settled(context, () => renderedDropdown(entity), (shown) => shown !== null && done(shown))
  assert(shown !== null, 'the engine has no dropdown')
  return shown!
}

const letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J']

test('ui-dropdown: fewer options replace the old ones', async function (context) {
  CustomReactEcsRenderer.destroy()
  await show(context, { options: letters }, (shown) => shown.options.length === 10)
  const shown = await show(context, { options: ['X', 'Y', 'Z'] }, (shown) => shown.options.length <= 3)
  assertEquals(shown.options.join(','), 'X,Y,Z', 'options')
  assertEquals(shown.selected, 0, 'selected')
  assertEquals(shown.text, 'X', 'text')
  CustomReactEcsRenderer.destroy()
})

test('ui-dropdown: a selectedIndex past fewer options selects nothing', async function (context) {
  CustomReactEcsRenderer.destroy()
  await show(context, { options: letters, selectedIndex: 5 }, (shown) => shown.selected === 5)
  const shown = await show(
    context,
    { options: ['X', 'Y', 'Z'], selectedIndex: 5, emptyLabel: 'None' },
    (shown) => shown.options.length <= 3
  )
  assertEquals(shown.selected, -1, 'selected')
  assertEquals(shown.text, 'None', 'text')
  CustomReactEcsRenderer.destroy()
})

test('ui-dropdown: acceptEmpty with no selection shows the emptyLabel', async function (context) {
  CustomReactEcsRenderer.destroy()
  const shown = await show(
    context,
    { options: ['One', 'Two', 'Three'], acceptEmpty: true, emptyLabel: 'Pick one' },
    (shown) => shown.options.length === 3 && shown.selected === -1
  )
  assertEquals(shown.selected, -1, 'selected')
  assertEquals(shown.text, 'Pick one', 'text')
  CustomReactEcsRenderer.destroy()
})

test('ui-dropdown: clearing selectedIndex clears the selection', async function (context) {
  CustomReactEcsRenderer.destroy()
  const options = ['One', 'Two', 'Three']
  const picked = await show(
    context,
    { options, acceptEmpty: true, emptyLabel: 'Pick one', selectedIndex: 1 },
    (shown) => shown.selected === 1
  )
  assertEquals(picked.text, 'Two', 'text with selectedIndex 1')
  const cleared = await show(
    context,
    { options, acceptEmpty: true, emptyLabel: 'Pick one', selectedIndex: undefined },
    (shown) => shown.selected === -1
  )
  assertEquals(cleared.selected, -1, 'selected after clearing')
  assertEquals(cleared.text, 'Pick one', 'text after clearing')
  CustomReactEcsRenderer.destroy()
})

test('ui-dropdown: without acceptEmpty the first option is selected', async function (context) {
  CustomReactEcsRenderer.destroy()
  const shown = await show(context, { options: ['One', 'Two', 'Three'] }, (shown) => shown.options.length === 3)
  assertEquals(shown.selected, 0, 'selected')
  assertEquals(shown.text, 'One', 'text')
  CustomReactEcsRenderer.destroy()
})

// Skipped: never run on Godot; 3 of 4 tests skipped upstream.

skipTest('ui-dropdown: this test only check the visual style of dropdown', async function (context) {
  customAddEntity.clean()
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(TestElementDropdown)

  await context.helpers.waitNTicks(5)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_dropdown.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8)
  )
})

skipTest(
  'ui-dropdown: should change background color to red',
  async function (context) {
    customAddEntity.clean()
    clicked = false
    CustomReactEcsRenderer.destroy()
    CustomReactEcsRenderer.setUiRenderer(TestElementDropdown)
    await context.helpers.waitTicksUntil(() => {
      if (clicked) {
        return true
      } else {
        return false
      }
    })

    await context.helpers.waitNTicks(5)

    await assertSnapshot(
      'screenshot/$explorer_snapshot_ui_dropdown_red.png',
      Vector3.create(8, 1, 10),
      Vector3.create(8, 1, 8)
    )

    ChangeColor(0)
  }
)

skipTest(
  'ui-dropdown: should change background color to blue',
  async function (context) {
    clicked = false
    CustomReactEcsRenderer.destroy()
    CustomReactEcsRenderer.setUiRenderer(TestElementDropdown)
    await context.helpers.waitTicksUntil(() => {
      if (clicked) {
        return true
      } else {
        return false
      }
    })

    await context.helpers.waitNTicks(5)

    await assertSnapshot(
      'screenshot/$explorer_snapshot_ui_dropdown_blue.png',
      Vector3.create(8, 1, 10),
      Vector3.create(8, 1, 8)
    )

    ChangeColor(0)
  }
)

skipTest(
  'ui-dropdown: should change background color to green',
  async function (context) {
    clicked = false
    CustomReactEcsRenderer.destroy()
    CustomReactEcsRenderer.setUiRenderer(TestElementDropdown)
    await context.helpers.waitTicksUntil(() => {
      if (clicked) {
        return true
      } else {
        return false
      }
    })

    await context.helpers.waitNTicks(5)

    await assertSnapshot(
      'screenshot/$explorer_snapshot_ui_dropdown_green.png',
      Vector3.create(8, 1, 10),
      Vector3.create(8, 1, 8)
    )

    ChangeColor(0)
  }
)
