import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import { RadiusRepro } from './qa'
import './tests/ui-background/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps(
    'UI-Background',
    [
      'The blue "radius 64", green "50%" and white "ring" shapes are circles, not squares or rounded squares.',
      'The red "50% wide" shape is a pill: straight top and bottom, fully round ends.',
      'The yellow "resized" shape is a circle. Tap "Shrink": it gets smaller and stays a circle. Tap "Grow": it grows back, still a circle.'
    ],
    RadiusRepro
  )
}
