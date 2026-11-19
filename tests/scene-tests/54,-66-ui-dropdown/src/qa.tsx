import { Color4 } from '@dcl/sdk/math'
import ReactEcs, { Dropdown, Label, UiEntity } from '@dcl/sdk/react-ecs'

// #3133: a dropdown whose options shrink, and one with acceptEmpty that the scene can clear.
const letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J']
const choices = ['One', 'Two', 'Three']
let many = true
let picked: number | undefined = undefined

function Button(props: { label: string; onPress: () => void }): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiTransform={{ height: 40, margin: { top: 8 }, padding: { left: 12, right: 12 }, alignItems: 'center' }}
      uiBackground={{ color: Color4.Gray() }}
      onMouseDown={props.onPress}
    >
      <Label value={props.label} fontSize={16} color={Color4.White()} />
    </UiEntity>
  )
}

export function DropdownRepro(): ReactEcs.JSX.Element {
  return (
    <UiEntity uiTransform={{ width: '100%', flexDirection: 'column', margin: { top: 16 } }}>
      <Button
        label={many ? 'Show 3 options' : 'Show 10 options'}
        onPress={() => {
          many = !many
        }}
      />
      <Dropdown
        uiTransform={{ height: 40, margin: { top: 8 } }}
        uiBackground={{ color: Color4.create(0.2, 0.2, 0.2, 1) }}
        fontSize={16}
        color={Color4.White()}
        options={many ? letters : ['X', 'Y', 'Z']}
      />
      <Dropdown
        uiTransform={{ height: 40, margin: { top: 16 } }}
        uiBackground={{ color: Color4.create(0.2, 0.2, 0.2, 1) }}
        fontSize={16}
        color={Color4.White()}
        acceptEmpty
        emptyLabel="Pick one"
        options={choices}
        selectedIndex={picked}
        onChange={(index) => {
          picked = index
        }}
      />
      <Label
        value={`Scene: ${picked === undefined ? 'none' : choices[picked]}`}
        fontSize={16}
        color={Color4.White()}
        textAlign="middle-left"
        uiTransform={{ margin: { top: 8 } }}
      />
      <Button
        label="Clear selection"
        onPress={() => {
          picked = undefined
        }}
      />
    </UiEntity>
  )
}
