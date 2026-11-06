import { Color4 } from '@dcl/sdk/math'
import ReactEcs, { Label, UiEntity } from '@dcl/sdk/react-ecs'

// #2060: with slices of 0.15 the red arrows sit in the edge strips, so tiling would repeat them
// along a wide panel; stretched (as in Unity) each corner keeps one.
const slices = { top: 0.15, right: 0.15, bottom: 0.15, left: 0.15 }

export function NineSliceRepro(): ReactEcs.JSX.Element {
  return (
    <UiEntity uiTransform={{ width: '100%', flexDirection: 'column', margin: { top: 16 } }}>
      <UiEntity
        uiTransform={{ width: 480, height: 200 }}
        uiBackground={{
          texture: { src: 'src/assets/images/9slice.png' },
          textureMode: 'nine-slices',
          textureSlices: slices
        }}
      />
      <Label
        value="nine-slices, slices 0.15"
        fontSize={14}
        color={Color4.White()}
        uiTransform={{ margin: { top: 4 } }}
      />
    </UiEntity>
  )
}
