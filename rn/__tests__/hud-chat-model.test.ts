import { parseBbcode } from '../src/hud/bbcode';
import {
  escapeUrlForLink,
  isSafeUrl,
  nameColor,
  rowKind,
  splitName,
  tokenize,
  truncateReduced,
} from '../src/hud/chatModel';

describe('parseBbcode', () => {
  it('turns color and bold tags into styled runs', () => {
    expect(
      parseBbcode('[color=#ccc]🟢 Teleported to [b]0,0[/b][/color]'),
    ).toEqual([
      { text: '🟢 Teleported to ', color: '#ccc' },
      { text: '0,0', color: '#ccc', bold: true },
    ]);
  });

  it('keeps plain text and drops unknown or unbalanced tags', () => {
    expect(parseBbcode('hello [i]x[/i] [b]y')).toEqual([
      { text: 'hello x ' },
      { text: 'y', bold: true },
    ]);
  });
});

describe('splitName', () => {
  it('splits unclaimed names into nickname and tag', () => {
    expect(splitName('Clara#1f0a')).toEqual({
      nickname: 'Clara',
      tag: '#1f0a',
      claimed: false,
    });
  });
  it('treats a name without tag as claimed', () => {
    expect(splitName('Clara')).toEqual({
      nickname: 'Clara',
      tag: '',
      claimed: true,
    });
  });
});

describe('nameColor', () => {
  it('is deterministic and a hex colour', () => {
    expect(nameColor('Clara#1f0a')).toBe(nameColor('Clara#1f0a'));
    expect(nameColor('x')).toMatch(/^#[0-9a-f]{6}$/i);
  });
});

describe('truncateReduced', () => {
  it('cuts at 80 chars with an ellipsis', () => {
    const text = 'a'.repeat(100);
    expect(truncateReduced(text)).toBe(`${'a'.repeat(80)}...`);
    expect(truncateReduced('short')).toBe('short');
  });
});

describe('isSafeUrl', () => {
  it('accepts plain https and rejects dangerous ones', () => {
    expect(isSafeUrl('https://decentraland.org/play')).toBe(true);
    expect(isSafeUrl('http://decentraland.org')).toBe(false);
    expect(isSafeUrl('https://x.org/javascript:alert(1)')).toBe(false);
    expect(isSafeUrl('https://x.org/<b>')).toBe(false);
    expect(isSafeUrl('https://x.org//evil')).toBe(false);
    expect(isSafeUrl(`https://x.org/${'a'.repeat(2100)}`)).toBe(false);
  });
  it('escapes brackets for the link target', () => {
    expect(escapeUrlForLink('https://x.org/[a]')).toBe('https://x.org/%5Ba%5D');
  });
});

describe('tokenize', () => {
  const ctx = { names: ['Clara#1f0a', 'Bob'], myName: 'Bob' };

  it('finds urls, bare domains and trims trailing punctuation', () => {
    const out = tokenize(
      'see https://decentraland.org/play, or amazon.com.',
      ctx,
    );
    expect(
      out.filter(s => s.kind === 'url').map(s => [s.text, s.value]),
    ).toEqual([
      ['https://decentraland.org/play', 'https://decentraland.org/play'],
      ['amazon.com', 'https://amazon.com'],
    ]);
    expect(out.map(s => s.text).join('')).toBe(
      'see https://decentraland.org/play, or amazon.com.',
    );
  });

  it('finds coordinates inside the world only', () => {
    const out = tokenize('meet at -12,45 or 999,1', ctx);
    expect(out.filter(s => s.kind === 'coord').map(s => s.value)).toEqual([
      '-12,45',
    ]);
  });

  it('finds world names before urls', () => {
    const out = tokenize('go to spacerunner.dcl.eth now', ctx);
    expect(out.find(s => s.kind === 'world')?.value).toBe(
      'spacerunner.dcl.eth',
    );
    expect(out.some(s => s.kind === 'url')).toBe(false);
  });

  it('marks known mentions clickable and mentions of me', () => {
    const out = tokenize('hi @Clara#1f0a and @Bob and @ghost', ctx);
    const mentions = out.filter(s => s.kind === 'mention');
    expect(mentions.map(s => [s.text, s.value, s.mine ?? false])).toEqual([
      ['@Clara#1f0a', 'Clara#1f0a', false],
      ['@Bob', 'Bob', true],
    ]);
  });

  it('matches names followed by punctuation, with spaces, or in the wrong case', () => {
    const named = {
      names: ['Kenz0', 'Lara Fire', 'Lara', 'Bob#a1b2', 'Anaïs'],
      myName: 'Me',
    };
    const found = (text: string) =>
      tokenize(text, named)
        .filter(s => s.kind === 'mention')
        .map(s => [s.text, s.value]);
    expect(found('hey @Kenz0!')).toEqual([['@Kenz0', 'Kenz0']]);
    expect(found('@kenz0 hi')).toEqual([['@Kenz0', 'Kenz0']]);
    expect(found('@Lara Fire, come')).toEqual([['@Lara Fire', 'Lara Fire']]);
    expect(found('@Lara come')).toEqual([['@Lara', 'Lara']]);
    expect(found('@Bob hi')).toEqual([['@Bob#a1b2', 'Bob#a1b2']]);
    expect(found('@Anaïs!')).toEqual([['@Anaïs', 'Anaïs']]);
    expect(found('@Kenz0x and mail@Kenz0')).toEqual([]);
  });

  it('needs the tag when two nearby profiles share a name', () => {
    const twins = { names: ['Bob#a1b2', 'Bob#c3d4'] };
    const found = (text: string) =>
      tokenize(text, twins)
        .filter(s => s.kind === 'mention')
        .map(s => s.value);
    expect(found('@Bob hi')).toEqual([]);
    expect(found('@Bob#c3d4 hi')).toEqual(['Bob#c3d4']);
  });

  it('does not link unsafe urls', () => {
    const out = tokenize('x www.bad.com/<script>', ctx);
    expect(out.every(s => s.kind === 'text')).toBe(true);
  });

  it('returns one text segment for plain messages', () => {
    expect(tokenize('hello world', ctx)).toEqual([
      { kind: 'text', text: 'hello world' },
    ]);
  });
});

describe('rowKind', () => {
  const base = { id: '1', message: 'x', timestamp: 0 };
  it('classifies system, own and other lines', () => {
    expect(rowKind({ ...base, address: 'system' }, undefined)).toBe('system');
    expect(rowKind({ ...base, address: '0xa' }, { own: true } as never)).toBe(
      'own',
    );
    expect(rowKind({ ...base, address: '0xb' }, undefined)).toBe('other');
  });
});
