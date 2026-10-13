import { applyMention, matchProfiles, mentionQuery } from '../src/hud/mentions';
import {
  MAX_ROWS,
  ROW_LIFETIME_MS,
  addRow,
  expireRows,
  nextExpiry,
} from '../src/hud/notifications';
import { chatFeedReducer, initialChatFeed } from '../src/hud/chatFeed';
import { hudReducer, initialHud } from '../src/hud/hudState';
import {
  COACH_STEPS,
  createMemoryStorage,
  loadCoachSeen,
  saveCoachSeen,
} from '../src/hud/coachMarks';
import {
  HUD_MARGIN_FLOOR,
  hudInsets,
  isNativeTouchZone,
} from '../src/hud/layout';
import { u } from '../src/ds';

const line = (n: number) => ({
  id: `l${n}`,
  address: '0xa',
  message: `m${n}`,
  timestamp: n,
});

describe('mentions', () => {
  it('reads the query after an @ at the start or after a space', () => {
    expect(mentionQuery('hi @cla', 7)).toBe('cla');
    expect(mentionQuery('@', 1)).toBe('');
    expect(mentionQuery('mail@x', 6)).toBeNull();
    expect(mentionQuery('hi @cla bob', 11)).toBeNull();
    expect(mentionQuery('hello', 5)).toBeNull();
  });

  it('matches names case-insensitively, sorted, capped at three', () => {
    const profiles = ['Zed', 'Bob', 'bobby', 'Rob#1f0a', 'Cobb', ''].map(
      name => ({ name }),
    );
    expect(matchProfiles('ob', profiles).map(p => p.name)).toEqual([
      'Bob',
      'bobby',
      'Cobb',
    ]);
    expect(matchProfiles('', profiles)).toHaveLength(3);
  });

  it('inserts the mention and moves the caret after it', () => {
    expect(applyMention('hi @cl and more', 6, 'Clara#1f0a')).toEqual({
      text: 'hi @Clara#1f0a  and more',
      caret: 15,
    });
    expect(applyMention('plain', 5, 'x')).toEqual({ text: 'plain', caret: 5 });
  });
});

describe('notifications', () => {
  it('keeps at most ten rows, dropping the oldest', () => {
    let rows = addRow([], line(0), 0);
    for (let i = 1; i <= MAX_ROWS; i += 1) {
      rows = addRow(rows, line(i), i);
    }
    expect(rows).toHaveLength(MAX_ROWS);
    expect(rows[0].line.id).toBe('l1');
  });

  it('expires each row 6.5 s after it arrived', () => {
    const rows = [addRow([], line(1), 0)][0];
    const two = addRow(rows, line(2), 3000);
    expect(expireRows(two, ROW_LIFETIME_MS - 1)).toHaveLength(2);
    expect(expireRows(two, ROW_LIFETIME_MS)).toHaveLength(1);
    expect(expireRows(two, 3000 + ROW_LIFETIME_MS)).toHaveLength(0);
    expect(nextExpiry(two)).toBe(ROW_LIFETIME_MS);
    expect(nextExpiry([])).toBeNull();
  });
});

describe('chatFeedReducer', () => {
  it('appends lines, counts unread while closed and resets on read', () => {
    let s = chatFeedReducer(initialChatFeed, {
      type: 'line',
      line: line(1),
      chatOpen: false,
    });
    s = chatFeedReducer(s, { type: 'line', line: line(2), chatOpen: false });
    expect(s.unread).toBe(2);
    s = chatFeedReducer(s, { type: 'line', line: line(3), chatOpen: true });
    expect(s.unread).toBe(2);
    expect(chatFeedReducer(s, { type: 'read' }).unread).toBe(0);
    expect(chatFeedReducer(s, { type: 'unread', count: 7 }).unread).toBe(7);
  });

  it('caps the history', () => {
    let s = initialChatFeed;
    for (let i = 0; i < 250; i += 1) {
      s = chatFeedReducer(s, { type: 'line', line: line(i), chatOpen: true });
    }
    expect(s.lines).toHaveLength(200);
    expect(s.lines[199].id).toBe('l249');
  });
});

describe('hudReducer', () => {
  it('moves chat closed -> open -> writing -> open -> closed', () => {
    let s = hudReducer(initialHud, { type: 'chat', mode: 'open' });
    expect(s.chat).toBe('open');
    s = hudReducer(s, { type: 'chat', mode: 'writing' });
    expect(s.chat).toBe('writing');
    s = hudReducer(s, { type: 'chat', mode: 'closed' });
    expect(s.chat).toBe('closed');
  });

  it('hide closes everything and show restores the HUD', () => {
    const open = { ...initialHud, chat: 'open' as const };
    const hidden = hudReducer(open, { type: 'hidden', hidden: true });
    expect(hidden).toMatchObject({ hidden: true, chat: 'closed' });
    expect(hudReducer(hidden, { type: 'hidden', hidden: false }).hidden).toBe(
      false,
    );
  });

  it('an open menu closes chat', () => {
    const s = hudReducer(
      { ...initialHud, chat: 'writing' },
      { type: 'menu', open: true },
    );
    expect(s).toMatchObject({ menu: true, chat: 'closed' });
  });
});

describe('coach marks', () => {
  it('has the two tutorial steps in order', () => {
    expect(COACH_STEPS).toEqual(['controls', 'camera']);
  });

  it('persists the seen flag through the storage interface', async () => {
    const storage = createMemoryStorage();
    expect(await loadCoachSeen(storage)).toBe(false);
    await saveCoachSeen(storage);
    expect(await loadCoachSeen(storage)).toBe(true);
  });

  it('treats a failing storage as not seen', async () => {
    const broken = {
      getItem: () => Promise.reject(new Error('x')),
      setItem: () => Promise.reject(new Error('x')),
    };
    expect(await loadCoachSeen(broken)).toBe(false);
    await expect(saveCoachSeen(broken)).resolves.toBeUndefined();
  });
});

describe('layout', () => {
  const zero = { top: 0, left: 0, right: 0, bottom: 0 };
  it('floors the device insets at the Godot margins', () => {
    expect(hudInsets(zero)).toEqual({
      left: u(HUD_MARGIN_FLOOR.left / 2),
      top: u(HUD_MARGIN_FLOOR.top / 2),
      right: u(HUD_MARGIN_FLOOR.right / 2),
      bottom: u(HUD_MARGIN_FLOOR.bottom / 2),
    });
    expect(hudInsets({ ...zero, left: 999 }).left).toBe(999);
  });

  it('flags the gamepad and joystick zones as native', () => {
    expect(isNativeTouchZone(1410, 591)).toBe(true);
    expect(isNativeTouchZone(247, 527)).toBe(true);
    expect(isNativeTouchZone(150, 70)).toBe(false);
    expect(isNativeTouchZone(1500, 100)).toBe(false);
  });
});
