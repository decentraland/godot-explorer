import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import './tests/locomotion/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps('Locomotion', [
    'Push the joystick halfway forward: the avatar walks. Push it fully: it jogs.',
    'Hold the run button while pushing the joystick: the avatar runs faster than it jogs.',
    'Tap Jump: the avatar jumps. Tap it again in the air: the second jump goes higher.'
  ])
}
