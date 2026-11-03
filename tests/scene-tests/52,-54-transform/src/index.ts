import { createBlackRoom } from 'testing-library/src/utils/black-room'

// This test should always be first
import './tests/transform/index.test'

export function main(): void {
  createBlackRoom()
}
