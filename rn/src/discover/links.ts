export type TextPart = { text: string; url?: string };

const URL_PATTERN = /(https?:\/\/[^\s]+)/g;

/** Splits text so every http(s) URL is its own tappable part. */
export function splitLinks(text: string): TextPart[] {
  return text
    .split(URL_PATTERN)
    .filter(part => part !== '')
    .map(part =>
      /^https?:\/\//.test(part) ? { text: part, url: part } : { text: part },
    );
}
