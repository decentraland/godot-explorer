import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'

// This test should always be first
import './tests/transform/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps('Transform', [])
}
