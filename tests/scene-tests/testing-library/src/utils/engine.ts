import { Entity } from '@dcl/sdk/ecs'
import { Vector3 } from '@dcl/sdk/math'
import { getSceneInformation } from '~system/Runtime'
import { testCommand } from '~system/Testing'
import { TestFunctionContext } from '../testing/types'

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

// What the engine made of an entity, read through `query`. Tests use these instead of a screenshot
// when the state shows what is tested; an engine rename then breaks one helper, not every test.

export type Rgba = [number, number, number, number]
export type Bounds = { position: [number, number, number]; size: [number, number, number] }

let ownScene: Promise<{ id: number; base: [number, number] }> | undefined

function engineScene(): Promise<{ id: number; base: [number, number] }> {
  ownScene ??= (async () => {
    const info = await getSceneInformation({})
    const base = (JSON.parse(info.metadataJson).scene as { base: string }).base
    const scenes: Array<{ scene_id: number; base_parcel: [number, number]; is_global: boolean }> =
      await query('scenes')
    const own = scenes.find((scene) => !scene.is_global && scene.base_parcel.join(',') === base)
    if (own === undefined) throw new Error(`no engine scene at ${base}`)
    return { id: own.scene_id, base: own.base_parcel }
  })()
  return ownScene
}

/** This scene's id in the engine (the debug hub's `scene_id`). */
export async function sceneId(): Promise<number> {
  return (await engineScene()).id
}

async function entityNode(entity: Entity, collect: Record<string, string[]>, ui = false): Promise<any> {
  const reply = await query(ui ? 'ui_entity' : 'entity', {
    scene_id: await sceneId(),
    entity_id: entity,
    filters: { include_parents: false, include_children: false, collect_nodes: collect }
  })
  return reply.godot
}

// Resources come back as "(path):<Class#id>".
function resourceClass(value: unknown): string | null {
  return typeof value === 'string' ? (/<(\w+)#/.exec(value)?.[1] ?? null) : null
}

function resourcePath(value: unknown): string | null {
  return typeof value === 'string' ? (/^\(([^)]*)\)/.exec(value)?.[1] ?? null) || null : null
}

// A texture loaded from the scene's content has no path: its class (`ImageTexture`) stands in.
function textureName(value: unknown): string | null {
  return resourcePath(value) ?? resourceClass(value)
}

const round = (values: number[]) => values.map((value) => Math.round(value * 100) / 100 + 0) as [number, number, number]

/**
 * The entity's node: whether it exists and is visible, where it is in scene coordinates (as a
 * `Transform` position, wherever the engine moved it: an `AvatarAttach` anchor, a parent), and the
 * bounds of everything it draws (glTF meshes included) in its own space, so before its own scale.
 * Bounds are rounded to 0.01: a sphere made of faces is 0.999 wide.
 */
export async function renderedNode(
  entity: Entity
): Promise<{ present: boolean; visible: boolean; position: Vector3 | null; bounds: Bounds | null }> {
  const node = await entityNode(entity, {})
  const base = (await engineScene()).base
  const global: [number, number, number] | undefined = node?.global_transform?.position
  const aabb = node?.aabb
  return {
    present: node?.present === true,
    visible: node?.visible === true,
    // Godot's z runs north to south.
    position:
      global === undefined
        ? null
        : Vector3.create(global[0] - base[0] * 16, global[1], -global[2] - base[1] * 16),
    bounds: aabb === undefined ? null : { position: round(aabb.position), size: round(aabb.size) }
  }
}

/** The `MeshRenderer` mesh: its Godot class (`SphereMesh`, or `ArrayMesh` for a box, cylinder or plane) and visibility. */
export async function renderedMesh(entity: Entity): Promise<{ kind: string; visible: boolean } | null> {
  const mesh = (await entityNode(entity, { MeshRenderer: ['mesh', 'visible'] }))?.MeshRenderer
  if (mesh === undefined) return null
  return { kind: resourceClass(mesh.mesh) ?? 'none', visible: mesh.visible === true }
}

/** The `GltfContainer` node: the model it loads, its loading state (SDK `LoadingState`) and visibility. */
export async function renderedModel(entity: Entity): Promise<{ src: string; state: number; visible: boolean } | null> {
  const model = (await entityNode(entity, { GltfContainer: ['dcl_gltf_src', 'dcl_gltf_loading_state', 'visible'] }))
    ?.GltfContainer
  if (model === undefined) return null
  return { src: model.dcl_gltf_src, state: model.dcl_gltf_loading_state, visible: model.visible === true }
}

/** The 3D label a `TextShape` draws; `fontSize` is in label pixels, not the SDK's units. */
export async function renderedText(entity: Entity): Promise<{
  text: string
  fontSize: number
  color: Rgba
  outlineWidth: number
  outlineColor: Rgba
  horizontalAlignment: number
  verticalAlignment: number
  font: string | null
  visible: boolean
} | null> {
  const label = (
    await entityNode(entity, {
      'TextShape/Label3D': [
        'text',
        'font_size',
        'modulate',
        'outline_size_float',
        'outline_modulate',
        'horizontal_alignment',
        'vertical_alignment',
        'font',
        'visible'
      ]
    })
  )?.['TextShape/Label3D']
  if (label === undefined) return null
  return {
    text: label.text,
    fontSize: label.font_size,
    color: label.modulate,
    outlineWidth: label.outline_size_float,
    outlineColor: label.outline_modulate,
    horizontalAlignment: label.horizontal_alignment,
    verticalAlignment: label.vertical_alignment,
    font: resourcePath(label.font),
    visible: label.visible === true
  }
}

/**
 * A scene UI entity as laid out: its rect in viewport pixels, and what its `UiBackground` and
 * `UiText` draw (each `null` when the entity has none). A texture is its `res://` path, or
 * `ImageTexture` for one the scene loaded; `stretch` is per axis: 0 stretch, 1 tile,
 * 2 tile to fit (nine-slices); `uvs` are set when a texture is drawn through custom UVs.
 */
export async function renderedUi(entity: Entity): Promise<{
  present: boolean
  visible: boolean
  position: [number, number]
  size: [number, number]
  globalPosition: [number, number]
  background: {
    color: Rgba
    texture: string | null
    margins: [number, number, number, number]
    stretch: [number, number]
    uvs: number[] | null
  } | null
  text: { text: string; color: Rgba; fontSize: number; font: string | null } | null
}> {
  const node = await entityNode(
    entity,
    {
      bkg: [
        'modulate',
        'texture',
        'patch_margin_left',
        'patch_margin_top',
        'patch_margin_right',
        'patch_margin_bottom',
        'axis_stretch_horizontal',
        'axis_stretch_vertical'
      ],
      'bkg/UVChild': ['visible', 'texture', 'material:shader_parameter/uvs_0', 'material:shader_parameter/uvs_1'],
      text: ['text', 'theme_override_colors/font_color', 'theme_override_font_sizes/font_size', 'theme_override_fonts/font']
    },
    true
  )
  const bkg = node?.bkg
  const uvChild = node?.['bkg/UVChild']
  const drawsUvs = uvChild?.visible === true
  const text = node?.text
  return {
    present: node?.present === true,
    visible: node?.visible === true,
    position: node?.position,
    size: node?.size,
    globalPosition: node?.global_position,
    background:
      bkg === undefined
        ? null
        : {
            color: bkg.modulate,
            texture: textureName(drawsUvs ? uvChild.texture : bkg.texture),
            margins: [bkg.patch_margin_left, bkg.patch_margin_top, bkg.patch_margin_right, bkg.patch_margin_bottom],
            stretch: [bkg.axis_stretch_horizontal, bkg.axis_stretch_vertical],
            uvs: drawsUvs
              ? [...uvChild['material:shader_parameter/uvs_0'], ...uvChild['material:shader_parameter/uvs_1']]
              : null
          },
    text:
      text === undefined
        ? null
        : {
            text: text.text,
            color: text['theme_override_colors/font_color'],
            fontSize: text['theme_override_font_sizes/font_size'],
            font: resourcePath(text['theme_override_fonts/font'])
          }
  }
}

/** A scene `UiDropdown` as the engine shows it: its options, the selected one (-1 for none) and the button's text. */
export async function renderedDropdown(
  entity: Entity
): Promise<{ options: string[]; selected: number; text: string } | null> {
  const count: number | undefined = (await entityNode(entity, { dropdown: ['item_count'] }, true))?.dropdown
    ?.item_count
  if (count === undefined) return null
  const items = Array.from({ length: count }, (_, i) => `popup/item_${i}/text`)
  const dropdown = (await entityNode(entity, { dropdown: ['selected', 'text', ...items] }, true))?.dropdown
  if (dropdown === undefined) return null
  return { options: items.map((item) => dropdown[item]), selected: dropdown.selected, text: dropdown.text }
}

/** True when every channel is within `tolerance` (colours come back as floats). */
export function sameColor(actual: Rgba | null | undefined, expected: Rgba, tolerance = 0.01): boolean {
  return actual != null && actual.every((channel, i) => Math.abs(channel - expected[i]) <= tolerance)
}

/**
 * Reads `read` once a tick until `done` accepts the value or `ticks` pass, and returns the last
 * value: the engine applies a component a few ticks after the scene sets it.
 */
export async function settled<T>(
  context: TestFunctionContext,
  read: () => Promise<T>,
  done: (value: T) => boolean,
  ticks = 30
): Promise<T> {
  let value = await read()
  for (let tick = 0; tick < ticks && !done(value); tick++) {
    await context.helpers.waitNTicks(1)
    value = await read()
  }
  return value
}
