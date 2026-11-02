import type { ChatLine } from './types';

export const MAX_HISTORY = 200;

export type ChatFeed = { lines: ChatLine[]; unread: number };

export const initialChatFeed: ChatFeed = { lines: [], unread: 0 };

export type ChatFeedAction =
  | { type: 'line'; line: ChatLine; chatOpen: boolean }
  | { type: 'read' }
  | { type: 'unread'; count: number };

export function chatFeedReducer(
  state: ChatFeed,
  action: ChatFeedAction,
): ChatFeed {
  switch (action.type) {
    case 'line':
      return {
        lines: [...state.lines, action.line].slice(-MAX_HISTORY),
        unread: action.chatOpen ? state.unread : state.unread + 1,
      };
    case 'read':
      return state.unread ? { ...state, unread: 0 } : state;
    case 'unread':
      return { ...state, unread: action.count };
  }
}
