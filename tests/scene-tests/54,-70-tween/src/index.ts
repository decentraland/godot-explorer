import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import { showCoin } from './qa'
import './tests/tween/index.test'

export function main(): void {
  createBlackRoom()
  showCoin()
  showQaSteps('Tween', [
    'Watch the round coin in the room (turn the camera if needed) for a few turns: it spins smoothly in one direction.',
    'Each time it completes a turn, it keeps going forward: no backward jerk at the end of the turn (a short pause is fine).'
  ])
}
