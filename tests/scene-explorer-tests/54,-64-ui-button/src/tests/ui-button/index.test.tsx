import { skipTest, test } from 'testing-library/src/testing'
import { CustomReactEcsRenderer } from 'testing-library/src/utils/ui'

import { Color4, Vector3 } from '@dcl/sdk/math'
import ReactEcs, { Button } from '@dcl/sdk/react-ecs'
import { assertSnapshot } from 'testing-library/src/utils/snapshot-test'
import { MainCanvas } from 'testing-library/src/utils/ui/item'
import { getScreenCanvasInfo } from 'testing-library/src/utils/ui/ui-utils'
import { Material, engine } from '@dcl/sdk/ecs'

let clicked: boolean = false

function ChangeColor(color: Color4): void {
  for (const [entity] of engine.getEntitiesWith(Material)) {
    Material.setPbrMaterial(entity, { albedoColor: color })
  }
}

function TestElementButton(): ReactEcs.JSX.Element {
  return (
    <MainCanvas>
      <Button
        value="Click change backgound to blue"
        variant="primary"
        uiTransform={{
          width: 200,
          height: 50,
          margin: { left: 156, top: 231 }
        }}
        onMouseDown={() => {
          clicked = true
          ChangeColor(Color4.Blue())
        }}
      />
    </MainCanvas>
  )
}

skipTest('ui-button: should change color to blue', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(TestElementButton)
  await context.helpers.waitTicksUntil(() => {
    if (clicked) {
      return true
    } else {
      return false
    }
  })

  await context.helpers.waitNTicks(5)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_button.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )

  ChangeColor(Color4.Black())
})

test('ui-button: this test only check the visual style of button', async function (context) {
  CustomReactEcsRenderer.destroy()
  CustomReactEcsRenderer.setUiRenderer(TestElementButton)

  await context.helpers.waitNTicks(5)

  await assertSnapshot(
    'screenshot/$explorer_snapshot_ui_button.png',
    Vector3.create(8, 1, 10),
    Vector3.create(8, 1, 8),
    getScreenCanvasInfo()
  )
})
