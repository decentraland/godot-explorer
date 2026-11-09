import { type TestFunctionContext } from '../testing/types'
import { simulateGesture, simulateInput } from '~system/Testing'

// Godot's ~system/Testing extension, live only under --scene-test: it presses the same actions as
// the on-screen joystick and buttons.
declare module '~system/Testing' {
  export function simulateInput(body: {
    action: PlayerAction
    pressed: boolean
    strength?: number
    frames?: number
  }): Promise<object>
}

export type PlayerAction = 'ia_forward' | 'ia_backward' | 'ia_left' | 'ia_right' | 'ia_jump' | 'ia_sprint' | 'ia_walk'

/**
 * Presses an action until `release`. `strength` (0..1) is how far the joystick is pushed; the
 * movement actions ignore anything under their 0.5 deadzone, like a real stick.
 */
export async function press(action: PlayerAction, strength = 1): Promise<void> {
  await simulateInput({ action, pressed: true, strength })
}

export async function release(action: PlayerAction): Promise<void> {
  await simulateInput({ action, pressed: false })
}

/**
 * Holds an action for exactly `frames` frames, counted by the engine so the same hold moves the
 * player the same distance every run, then waits until it has been released.
 */
export async function hold(context: TestFunctionContext, action: PlayerAction, frames: number, strength = 1): Promise<void> {
  await simulateInput({ action, pressed: true, strength, frames })
  await context.helpers.waitNTicks(frames + 2)
}

/** One press of one frame, like a tap on the button. */
export async function tap(context: TestFunctionContext, action: PlayerAction): Promise<void> {
  await hold(context, action, 1)
}

// Godot's ~system/Testing extension for touch gestures, played through the real mobile controls.
declare module '~system/Testing' {
  export function simulateGesture(body: {
    gesture: 'joystick' | 'look' | 'pinch'
    direction?: { x: number; y: number }
    delta?: { x: number; y: number }
    spread?: number
    strength?: number
    frames?: number
  }): Promise<object>
}

/**
 * Pushes the on-screen joystick toward `direction` (screen axes: `{ x: 0, y: -1 }` is forward) by
 * `strength` (0..1) and holds it for `frames` frames, then lets go.
 */
export async function joystick(
  context: TestFunctionContext,
  direction: { x: number; y: number },
  strength: number,
  frames: number
): Promise<void> {
  await simulateGesture({ gesture: 'joystick', direction, strength, frames })
  await context.helpers.waitNTicks(frames + 4)
}

/** Drags one finger across the camera area by `delta` pixels over `frames` frames. */
export async function look(context: TestFunctionContext, delta: { x: number; y: number }, frames: number): Promise<void> {
  await simulateGesture({ gesture: 'look', delta, frames })
  await context.helpers.waitNTicks(frames + 4)
}

/** Spreads two fingers apart by `spread` pixels over `frames` frames (negative pinches them). */
export async function pinch(context: TestFunctionContext, spread: number, frames: number): Promise<void> {
  await simulateGesture({ gesture: 'pinch', spread, frames })
  await context.helpers.waitNTicks(frames + 4)
}
