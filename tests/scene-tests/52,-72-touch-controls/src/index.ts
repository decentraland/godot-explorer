import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import './tests/touch-controls/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps('Touch Controls', [
    'Drag one finger across the right half of the screen: the camera turns the same way.',
    'Spread two fingers apart on the right half: the camera switches to first person. Pinch them together: back to third person.',
    'Walk with the joystick while dragging the camera with the other hand: both work at once.'
  ])
}
