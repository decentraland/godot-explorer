import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import { DropdownRepro } from './qa'
import './tests/ui-dropdown/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps(
    'UI-Dropdown',
    [
      'Open the first dropdown: it lists A to J.',
      'Tap "Show 3 options", then open the first dropdown: it lists only X, Y and Z.',
      'Tap "Show 10 options": the first dropdown lists A to J again.',
      'Before picking anything, the second dropdown reads "Pick one".',
      'Pick "Two" in the second dropdown: the line under it reads "Scene: Two".',
      'Tap "Clear selection": the second dropdown reads "Pick one" again, and the line "Scene: none".'
    ],
    DropdownRepro
  )
}
