export type ChatMode = 'closed' | 'open' | 'writing';

export type HudUiState = {
  chat: ChatMode;
  /** UI hidden: only the restore button remains. */
  hidden: boolean;
  /** The in-world menu drawer is open; the HUD group steps aside. */
  menu: boolean;
};

export const initialHud: HudUiState = {
  chat: 'closed',
  hidden: false,
  menu: false,
};

export type HudAction =
  | { type: 'chat'; mode: ChatMode }
  | { type: 'hidden'; hidden: boolean }
  | { type: 'menu'; open: boolean };

export function hudReducer(state: HudUiState, action: HudAction): HudUiState {
  switch (action.type) {
    case 'chat':
      return { ...state, chat: action.mode };
    case 'hidden':
      return action.hidden
        ? { ...state, hidden: true, chat: 'closed' }
        : { ...state, hidden: false };
    case 'menu':
      return action.open
        ? { ...state, menu: true, chat: 'closed' }
        : { ...state, menu: false };
  }
}
