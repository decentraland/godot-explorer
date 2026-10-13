import ReactEcs, { Input, Label, UiEntity } from '@dcl/sdk/react-ecs'
import { Color4 } from '@dcl/sdk/math'

let value = ''
let submitted = ''

const field = (placeholder: string) => (
  <Input
    uiTransform={{ width: 600, height: 80 }}
    placeholder={placeholder}
    fontSize={28}
    onChange={(text) => {
      value = text
    }}
    onSubmit={(text) => {
      submitted = text
    }}
  />
)

// A field at the top, a mirror of what the scene received in the middle, a field at the bottom
// where the soft keyboard overlaps.
export const ui = () => (
  <UiEntity
    uiTransform={{
      width: '100%',
      height: '100%',
      flexDirection: 'column',
      justifyContent: 'space-between',
      alignItems: 'center',
      padding: 40
    }}
  >
    {field('top input')}
    <Label
      value={`value: ${value}\nsubmitted: ${submitted}`}
      fontSize={32}
      color={Color4.White()}
    />
    {field('bottom input (keyboard overlaps)')}
  </UiEntity>
)
