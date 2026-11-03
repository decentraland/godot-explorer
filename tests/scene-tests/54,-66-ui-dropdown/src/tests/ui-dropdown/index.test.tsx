import { skipTest, test } from 'testing-library/src/testing'
import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'

import { Color4, Vector3 } from '@dcl/sdk/math'
import ReactEcs, { Dropdown, Label, UiEntity } from '@dcl/sdk/react-ecs'
import { customAddEntity } from 'testing-library/src/utils/entity'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { Material, engine } from '@dcl/sdk/ecs'

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

test('ui-dropdown: this test only check the visual style of dropdown', async function (context) {
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
