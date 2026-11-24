import { showQaSteps } from 'testing-library/src/qa/panel'
import { startAvatarShapeMotion } from './avatar-shape-motion'
import './tests/avatar-shape-motion/index.test'

export function main(): void {
  startAvatarShapeMotion()
  showQaSteps('Avatar Shape Motion', [
    'Stand at the spawn point, facing north: the big sign at the far end is straight ahead.',
    'Row in front of you: only "npc-1 VISIBLE" has a body. Under "no id", "npc-2" and "0x1111…" there is nobody.',
    'Lanes behind it: while the sign reads 10 m/s, the 1x NPC runs and the 3x and 6x NPCs walk. Legs step, nobody slides.',
    'While the sign reads 3 m/s all three walk, the 6x one included. While it reads STOPPED all three stand idle.',
    'The dancer moves side to side and keeps dancing the whole time.'
  ])
}
