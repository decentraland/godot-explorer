export const MAX_SUGGESTIONS = 3;

/** The text typed after an `@` that starts the input or follows a space, or null when not mentioning. */
export function mentionQuery(text: string, caret: number): string | null {
  const before = text.slice(0, caret);
  const at = before.lastIndexOf('@');
  if (at === -1 || (at > 0 && before[at - 1] !== ' ')) {
    return null;
  }
  // Names can contain spaces: the query ends when it outgrows every nearby name.
  return before.slice(at + 1);
}

export function matchProfiles<T extends { name: string }>(
  query: string,
  profiles: T[],
): T[] {
  const needle = query.toLowerCase();
  return profiles
    .filter(p => p.name !== '' && p.name.toLowerCase().includes(needle))
    .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
    .slice(0, MAX_SUGGESTIONS);
}

export function applyMention(
  text: string,
  caret: number,
  fullName: string,
): { text: string; caret: number } {
  const at = text.slice(0, caret).lastIndexOf('@');
  if (at === -1) {
    return { text, caret };
  }
  const mention = `@${fullName} `;
  return {
    text: text.slice(0, at) + mention + text.slice(caret),
    caret: at + mention.length,
  };
}
