import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import './tests/emotes/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps('Emotes', [])
}
