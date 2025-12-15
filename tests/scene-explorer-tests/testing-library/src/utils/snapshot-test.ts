import { type Vector3 } from '@dcl/sdk/math'
import { getExplorerInformation } from '~system/Runtime'
import {
  takeAndCompareScreenshot,
  type Vector2,
  type TakeAndCompareScreenshotRequest_ComparisonMethodGreyPixelDiff,
  type TakeAndCompareScreenshotResponse
} from '~system/Testing'

let explorerAgent = 'unknown'

type SnapshotComparisonMethod = {
  method: 'grey-diff'
  threshold: number
}

function getMethodRequest(
  method: SnapshotComparisonMethod
): TakeAndCompareScreenshotRequest_ComparisonMethodGreyPixelDiff {
  switch (method.method) {
    case 'grey-diff':
      return { greyPixelDiff: {} }
    default:
      throw new Error('method not reached')
  }
}

function assertMethodResult(
  method: SnapshotComparisonMethod,
  result: TakeAndCompareScreenshotResponse
): void {
  switch (method.method) {
    case 'grey-diff':
      if (result.greyPixelDiff === undefined) {
        throw new Error(
          `method grey-diff was specified but greyPixelDiff result is undefined`
        )
      }
      if (result.greyPixelDiff.similarity < method.threshold) {
        throw new Error(
          `method grey-diff was specified and greyPixelDiff similarity (${result.greyPixelDiff.similarity}) is lower than threshold ${method.threshold}`
        )
      }
      break
    default:
      throw new Error('method not reached')
  }
}

export const DEFAULT_COMPARISON_METHOD: SnapshotComparisonMethod = {
  method: 'grey-diff',
  threshold: 0.9995
}
export const DEFAULT_SCREENSHOT_SIZE: Vector2 = { x: 512, y: 512 }
export const FAIL_IF_SNAPSHOT_NOT_FOUND = true

/**
 *
 * @param name it resolves the source path as lowercase without spaces
 * @param cameraPosition
 * @param cameraTarget
 *
 * e.g.
 *  - godot explorer and assertSnapshot('Mesh Renderer with box set, scale 1,1,1', Vector3.create(1, 1, 1), Vector3.create(1, 1, 2))
 *  - the path `${sceneFolderCwd}/screenshot/godot_snapshot_mesh_renderer_with_box_set_scale_1_1_1.png` will be used
 *  - by default it uses grey-diff method with threshold 0.9995
 */
export async function assertSnapshot(
  srcStoredSnapshot: string,
  cameraPosition: Vector3,
  cameraTarget: Vector3,
  screenshotSize: Vector2 = DEFAULT_SCREENSHOT_SIZE,
  method: SnapshotComparisonMethod = DEFAULT_COMPARISON_METHOD
): Promise<void> {
  if (explorerAgent === 'unknown') {
    const info = await getExplorerInformation({})
    explorerAgent = info.agent
  }

  const finalSrcStoredSnapshot = srcStoredSnapshot
    .replace('$explorer', explorerAgent)
    .toLocaleLowerCase()
  const result = await takeAndCompareScreenshot({
    snapshotMode: 0 as any,
    srcStoredSnapshot: finalSrcStoredSnapshot,
    cameraPosition,
    cameraTarget,
    screenshotSize,
    ...getMethodRequest(method)
  })

  if (Object.keys(result).length === 0) {
    throw new Error(
      `Snapshot result is empty, maybe the explorer is not running in testing mode`
    )
  }

  if (!result.storedSnapshotFound) {
    if (FAIL_IF_SNAPSHOT_NOT_FOUND) {
      throw new Error(
        `Snapshot not found, please copy the snapshot from the explorer-screenshot folder to the path "${finalSrcStoredSnapshot}"`
      )
    } else {
      console.log(
        `Snapshot not found, please copy the snapshot from the explorer-screenshot folder to the path "${finalSrcStoredSnapshot}"`
      )
      return
    }
  }

  assertMethodResult(method, result)
}
