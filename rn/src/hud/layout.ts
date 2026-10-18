import { u } from '../ds';

export type Insets = {
  top: number;
  left: number;
  right: number;
  bottom: number;
};

/** Figma frame the HUD was drawn on; sizes below are in these px (dp = px / 2 via `u`). */
export const HUD_FRAME = { width: 1600, height: 720 } as const;

/** `hud_margins.tres` landscape floor, in 720-high viewport px, on top of the device insets. */
export const HUD_MARGIN_FLOOR: Insets = {
  left: 108,
  top: 32,
  right: 108,
  bottom: 45,
};

/** Right/bottom corner where the native gamepad lives: RN keeps x >= minX and y >= minY clear. */
export const GAMEPAD_ZONE = { minX: 1180, minY: 380 } as const;

/** Native floating-joystick area (left ~40%, below the chat column); only the emote button sits inside. */
export const JOYSTICK_ZONE = { x: 0, y: 380, width: 640, height: 340 } as const;

/** Resting joystick base, which the controls overlay and the tutorial point at. */
export const JOYSTICK_BASE = {
  x: 161,
  y: 442,
  width: 172,
  height: 170,
} as const;

export const HUD_SIZES = {
  menu: 82,
  chat: 66,
  emote: 80,
  hideUi: 64,
  groupGap: 32,
  listGap: 14,
  chatPanelWidth: 420,
  notificationsWidth: 337,
} as const;

/** Figma px of the HUD frame -> dp from the screen's top-left, on the same floor as the buttons. */
export function hudPoint(x: number, y: number, insets: Insets) {
  return {
    x: insets.left + u((x - HUD_MARGIN_FLOOR.left) / 2),
    y: insets.top + u((y - HUD_MARGIN_FLOOR.top) / 2),
  };
}

/** Effective HUD insets in dp: the larger of the device inset and the Godot margin floor. */
export function hudInsets(device: Insets): Insets {
  return {
    left: Math.max(device.left, u(HUD_MARGIN_FLOOR.left / 2)),
    top: Math.max(device.top, u(HUD_MARGIN_FLOOR.top / 2)),
    right: Math.max(device.right, u(HUD_MARGIN_FLOOR.right / 2)),
    bottom: Math.max(device.bottom, u(HUD_MARGIN_FLOOR.bottom / 2)),
  };
}

/** True when a point in 720-high landscape px falls where RN must let touches through to Godot. */
export function isNativeTouchZone(x: number, y: number): boolean {
  const gamepad = x >= GAMEPAD_ZONE.minX && y >= GAMEPAD_ZONE.minY;
  const joystick =
    x >= JOYSTICK_ZONE.x &&
    x < JOYSTICK_ZONE.x + JOYSTICK_ZONE.width &&
    y >= JOYSTICK_ZONE.y &&
    y < JOYSTICK_ZONE.y + JOYSTICK_ZONE.height;
  return gamepad || joystick;
}
