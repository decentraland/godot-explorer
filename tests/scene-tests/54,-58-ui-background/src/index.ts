import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import { NineSliceRepro } from './qa'
import './tests/ui-background/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps(
    'UI-Background',
    [
      'The wide panel shows exactly four red arrows, one by each corner, stretched along the edges: no repeated arrows or corner notches along the top, bottom or sides.',
      'The dark blue centre is one smooth area, with no seams or repeated pattern.'
    ],
    NineSliceRepro
  )
}
