import { MOCK_BOB, MOCK_CLARA, MOCK_OWN_ADDRESS } from '../core/mock/worldSim';
import type { ChatProfile } from './types';

const profile = (
  address: string,
  name: string,
  tag: string,
  color: string,
  own = false,
): ChatProfile => ({
  address,
  name,
  tag,
  claimed: tag === '',
  color,
  own,
});

/** The people the core mock's world chats as, for previews and tests. */
export const MOCK_PROFILES: ChatProfile[] = [
  profile(MOCK_OWN_ADDRESS, 'Mateo', '', '#FF4BED', true),
  profile(MOCK_CLARA, 'Clara', '#1f0a', '#FF7439'),
  profile(MOCK_BOB, 'Bob', '#c3d2', '#73D3D3'),
  profile('0x5a1000000000000000000000000000000003a77e', 'Sally', '', '#34CE76'),
];
