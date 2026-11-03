import { testCommand } from '~system/Testing'

// Godot's ~system/Testing extension, live only under --scene-test: what the React Native shell can
// ask of the app, and the debug hub's read-only view of the engine.
declare module '~system/Testing' {
  export function testCommand(body: { kind: 'control' | 'setting' | 'query'; payload: unknown }): Promise<any>
}

/**
 * Sends a shell command to the core as the React Native app does, e.g.
 * `{ type: 'set_color', target: 'eyes', color: '#ff0000' }` or `{ type: 'emote', urn: 'wave' }`.
 * Avatar edits reach the engine as the avatar to render, like a Backpack edit.
 */
export async function shell(control: { type: string; [field: string]: unknown }): Promise<void> {
  await testCommand({ kind: 'control', payload: control })
}

/**
 * Changes a setting the way the settings screen does (not saved), e.g. `graphic_profile`; resolves to
 * how many settings changed with it (a graphic profile drives three more).
 */
export async function setSetting(key: string, value: boolean | number | string): Promise<number> {
  return await testCommand({ kind: 'setting', payload: { key, value } })
}

/**
 * A read-only debug-hub query, answered as the hub answers it: `state`, `scenes`, `scene`,
 * `entity`, `ui_scene`, `ui_entity`, `avatars`, `avatar` or `app_ui`, with the hub's `filters`
 * (`collect_nodes`, `class_filter`, `name_contains`, ...). See the mobile-dev-debug-tool skill.
 */
export async function query(cmd: string, args: Record<string, unknown> = {}): Promise<any> {
  return await testCommand({ kind: 'query', payload: { cmd, args } })
}
