import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import { showVideoScreen } from './qa'
import './tests/video-player/index.test'

export function main(): void {
  createBlackRoom()
  showVideoScreen()
  showQaSteps('Video Player', [
    'Look at the wall in front of you: a 4-second clip plays and loops, sharp, with no pink, green or frozen frames.',
    'Android: send the app to the background for 5 seconds and come back. The clip keeps playing within a second.'
  ])
}
