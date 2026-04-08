/**
 * Schema for exported scene state at a specific tick.
 * This file defines the types for the JSON export from the Scene Log Viewer.
 */

/** A single component entry in the exported state */
export interface ExportedComponent {
  /** Entity ID (combined number and version as u32) */
  entity_id: number;
  /** Component ID (matches Decentraland SDK component IDs) */
  component_id: number;
  /** Raw binary payload encoded as hex string */
  bin_payload: string;
}

/** Exported state at a specific tick */
export interface ExportedState {
  /** The tick number at which this state was captured */
  tick: number;
  /** Export timestamp in milliseconds since epoch */
  exported_at: number;
  /** Total number of entities in the state */
  entity_count: number;
  /** Total number of components in the state */
  component_count: number;
  /** All components in the state (LWW components appear once, GOS components may have multiple entries) */
  components: ExportedComponent[];
}

/**
 * Component name to ID mapping (from Decentraland SDK).
 * Use this to convert component names back to IDs.
 */
export const COMPONENT_NAME_TO_ID: Record<string, number> = {
  "Transform": 1,
  "MeshRenderer": 1018,
  "MeshCollider": 1019,
  "Material": 1017,
  "AudioSource": 1020,
  "AudioStream": 1021,
  "TextShape": 1030,
  "NftShape": 1040,
  "GltfContainer": 1041,
  "Animator": 1042,
  "VideoPlayer": 1043,
  "VideoEvent": 1044,
  "EngineInfo": 1048,
  "GltfContainerLoadingState": 1049,
  "UiTransform": 1050,
  "UiText": 1052,
  "UiBackground": 1053,
  "UiCanvasInformation": 1054,
  "TriggerArea": 1060,
  "TriggerAreaResult": 1061,
  "PointerEvents": 1062,
  "PointerEventsResult": 1063,
  "Raycast": 1067,
  "RaycastResult": 1068,
  "AvatarModifierArea": 1070,
  "CameraModeArea": 1071,
  "CameraMode": 1072,
  "AvatarAttach": 1073,
  "PointerLock": 1074,
  "MainCamera": 1075,
  "VirtualCamera": 1076,
  "InputModifier": 1078,
  "LightSource": 1079,
  "AvatarShape": 1080,
  "VisibilityComponent": 1081,
  "AvatarBase": 1087,
  "AvatarEmoteCommand": 1088,
  "PlayerIdentityData": 1089,
  "Billboard": 1090,
  "AvatarEquippedData": 1091,
  "UiInput": 1093,
  "UiDropdown": 1094,
  "UiInputResult": 1095,
  "UiDropdownResult": 1096,
  "MapPin": 1097,
  "GltfNodeModifiers": 1099,
  "Tween": 1102,
  "TweenState": 1103,
  "TweenSequence": 1104,
  "AudioEvent": 1105,
  "RealmInfo": 1106,
  "GltfNode": 1200,
  "GltfNodeState": 1201,
  "UiScrollResult": 1202,
  "UiCanvas": 1203,
  "GlobalLight": 1206,
  "TextureCamera": 1207,
  "CameraLayers": 1208,
  "PrimaryPointerInfo": 1209,
  "SkyboxTime": 1210,
  "CameraLayer": 1211,
};

/**
 * Get component ID from name.
 * Returns 0 if the component name is not found.
 */
export function getComponentId(name: string): number {
  return COMPONENT_NAME_TO_ID[name] ?? 0;
}
