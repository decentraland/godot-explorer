export type Run = { text: string; color?: string; bold?: boolean };

const TAG = /\[(\/?)([a-z]+)(?:=([^\]]+))?\]/g;

/** Godot sends system lines as BBCode; only `[color]` and `[b]` are honoured, other tags are dropped. */
export function parseBbcode(source: string): Run[] {
  const runs: Run[] = [];
  const colors: string[] = [];
  let bold = 0;
  let last = 0;
  const push = (text: string) => {
    if (!text) {
      return;
    }
    const color = colors[colors.length - 1];
    const previous = runs[runs.length - 1];
    if (previous && previous.color === color && !!previous.bold === bold > 0) {
      previous.text += text;
      return;
    }
    runs.push({
      text,
      ...(color ? { color } : {}),
      ...(bold ? { bold: true } : {}),
    });
  };
  for (const match of source.matchAll(TAG)) {
    push(source.slice(last, match.index));
    last = match.index + match[0].length;
    const [, closing, tag, value] = match;
    if (tag === 'b') {
      bold = Math.max(0, bold + (closing ? -1 : 1));
    } else if (tag === 'color') {
      if (closing) {
        colors.pop();
      } else if (value) {
        colors.push(value);
      }
    }
  }
  push(source.slice(last));
  return runs;
}
