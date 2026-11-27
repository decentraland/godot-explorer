import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import { LocaleRepro } from './qa'
import './tests/locale/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps(
    'Locale',
    [
      'Set the phone to Español (Argentina) and the app to English (Settings → Language), then open this scene: "app" shows en and "device" shows es-AR.',
      'Pick Português (Brasil) in Settings → Language, close Settings and tap "Re-read": "app" shows pt-BR, "device" still shows es-AR.'
    ],
    LocaleRepro
  )
}
