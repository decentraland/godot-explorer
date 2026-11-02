import Clipboard from '@react-native-clipboard/clipboard';

/** Puts `text` on the system clipboard. */
export function copyText(text: string): void {
  Clipboard.setString(text);
}
