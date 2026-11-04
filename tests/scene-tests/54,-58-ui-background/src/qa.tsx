import { Color4 } from '@dcl/sdk/math'
import ReactEcs, { Label, UiEntity } from '@dcl/sdk/react-ecs'

// #3124: radii over half the box, percentages, and a box that changes size.
const SIZE = 80
let small = false

const half = { topLeft: '50%', topRight: '50%', bottomRight: '50%', bottomLeft: '50%' } as any

function Shape(props: { label: string; shape: ReactEcs.JSX.Element }): ReactEcs.JSX.Element {
  return (
    <UiEntity uiTransform={{ flexDirection: 'column', alignItems: 'center', margin: { right: 12 } }}>
      {props.shape}
      <Label value={props.label} fontSize={14} color={Color4.White()} uiTransform={{ margin: { top: 4 } }} />
    </UiEntity>
  )
}

function Button(props: { label: string; onPress: () => void }): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiTransform={{ height: 40, margin: { right: 8 }, padding: { left: 12, right: 12 }, alignItems: 'center' }}
      uiBackground={{ color: Color4.Gray() }}
      onMouseDown={props.onPress}
    >
      <Label value={props.label} fontSize={16} color={Color4.White()} />
    </UiEntity>
  )
}

export function RadiusRepro(): ReactEcs.JSX.Element {
  const resized = small ? SIZE / 2 : SIZE
  return (
    <UiEntity uiTransform={{ width: '100%', flexDirection: 'column', margin: { top: 16 } }}>
      <UiEntity uiTransform={{ flexDirection: 'row', alignItems: 'flex-end' }}>
        <Shape
          label="radius 64"
          shape={
            <UiEntity
              uiTransform={{ width: SIZE, height: SIZE, borderRadius: 64 }}
              uiBackground={{ color: Color4.Blue() }}
            />
          }
        />
        <Shape
          label="50%"
          shape={
            <UiEntity
              uiTransform={{ width: SIZE, height: SIZE, borderRadius: half }}
              uiBackground={{ color: Color4.Green() }}
            />
          }
        />
        <Shape
          label="ring"
          shape={
            <UiEntity
              uiTransform={{ width: SIZE, height: SIZE, borderRadius: 64, borderWidth: 4, borderColor: Color4.White() }}
            />
          }
        />
        <Shape
          label="50% wide"
          shape={
            <UiEntity
              uiTransform={{ width: SIZE * 2, height: SIZE, borderRadius: half }}
              uiBackground={{ color: Color4.Red() }}
            />
          }
        />
      </UiEntity>
      <UiEntity uiTransform={{ flexDirection: 'row', alignItems: 'center', margin: { top: 16 } }}>
        <Shape
          label="resized"
          shape={
            <UiEntity
              uiTransform={{ width: resized, height: resized, borderRadius: 40 }}
              uiBackground={{ color: Color4.Yellow() }}
            />
          }
        />
        <Button
          label="Shrink"
          onPress={() => {
            small = true
          }}
        />
        <Button
          label="Grow"
          onPress={() => {
            small = false
          }}
        />
      </UiEntity>
    </UiEntity>
  )
}
