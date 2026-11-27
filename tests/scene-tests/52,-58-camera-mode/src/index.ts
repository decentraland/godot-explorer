import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'

import './tests/camera-mode/index.test'
import './tests/camera-mode/virtual-camera-fov.test'

export function main(): void {
  createBlackRoom()
  showQaSteps('CameraMode', [])
}
