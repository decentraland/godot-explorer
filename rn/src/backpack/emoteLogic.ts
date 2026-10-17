import { Control } from '../core/native';
import { Emote, emoteIdOf } from './emotes';

export type EmoteStrip = 'unequip' | 'check' | 'equip' | null;

export const slotOfEmote = (slots: string[], id: string): number =>
  slots.findIndex(urn => urn !== '' && emoteIdOf(urn) === id);

/**
 * Strip shown on a grid tile: UNEQUIP for the selected slot's emote (any equipped one while no slot
 * is selected), a check for one worn in another slot, EQUIP for the tapped candidate.
 */
export function emoteStrip(
  id: string,
  slots: string[],
  selected: number | null,
  pending: string | null,
): EmoteStrip {
  if (pending === id) {
    return 'equip';
  }
  const index = slotOfEmote(slots, id);
  if (index === -1) {
    return null;
  }
  return selected === null || index === selected ? 'unequip' : 'check';
}

export type EmoteTap = {
  commands: Control[];
  /** Candidate waiting for a second tap, or null. */
  pending: string | null;
  /** Slot to select afterwards, set when the emote was equipped. */
  equippedSlot: number | null;
};

/** First tap previews and arms EQUIP; the second equips; an UNEQUIP tile empties its slot. */
export function tapEmote(
  emote: Emote,
  slots: string[],
  selected: number | null,
  pending: string | null,
): EmoteTap {
  const strip = emoteStrip(emote.id, slots, selected, pending);
  const index = slotOfEmote(slots, emote.id);
  if (strip === 'unequip') {
    return {
      commands: [new Control.SetEmoteSlot({ slot: index, urn: '' })],
      pending: null,
      equippedSlot: null,
    };
  }
  if (strip === 'equip') {
    const slot = selected ?? Math.max(0, slots.indexOf(''));
    const commands: Control[] = [];
    if (index !== -1 && index !== slot) {
      commands.push(new Control.SetEmoteSlot({ slot: index, urn: '' }));
    }
    commands.push(new Control.SetEmoteSlot({ slot, urn: emote.urn }));
    return { commands, pending: null, equippedSlot: slot };
  }
  return {
    commands: [new Control.PlayPreviewEmote({ urn: emote.urn })],
    pending: emote.id,
    equippedSlot: null,
  };
}

/** Name in the chip above the slot row: the previewed candidate, else the selected slot's emote. */
export function chipName(
  emotes: Emote[],
  slots: string[],
  selected: number | null,
  pending: string | null,
): string | null {
  const id =
    pending ??
    (selected !== null && slots[selected] ? emoteIdOf(slots[selected]) : null);
  return emotes.find(emote => emote.id === id)?.name ?? null;
}
