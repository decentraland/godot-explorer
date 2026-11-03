import { createBlackRoom } from 'testing-library/src/utils/black-room'
import { showQaSteps } from 'testing-library/src/qa/panel'
import './tests/nft-shape/index.test'

export function main(): void {
  createBlackRoom()
  showQaSteps('NFT Shape', [])
}
