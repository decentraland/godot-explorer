import { Color4 } from '@dcl/sdk/math'
import ReactEcs, { Label, UiEntity } from '@dcl/sdk/react-ecs'
import { readLocales } from './locale'

let shown = { locale: 'reading…', systemLocale: 'reading…' }

function read(): void {
  readLocales()
    .then((value) => {
      shown = value
    })
    .catch((error) => {
      shown = { locale: String(error), systemLocale: String(error) }
    })
}
read()

export function LocaleRepro(): ReactEcs.JSX.Element {
  return (
    <UiEntity
      uiTransform={{
        width: '100%',
        flexDirection: 'column',
        margin: { top: 16 }
      }}
    >
      <Label
        value={`app: ${shown.locale}`}
        fontSize={18}
        color={Color4.White()}
      />
      <Label
        value={`device: ${shown.systemLocale}`}
        fontSize={18}
        color={Color4.White()}
      />
      <UiEntity
        uiTransform={{
          height: 40,
          width: 120,
          margin: { top: 8 },
          alignItems: 'center',
          justifyContent: 'center'
        }}
        uiBackground={{ color: Color4.Gray() }}
        onMouseDown={read}
      >
        <Label value="Re-read" fontSize={16} color={Color4.White()} />
      </UiEntity>
    </UiEntity>
  )
}
