const env = require('env')

const testingEnabled = env['testing_enable']

function emptyTesting() {
    return {
        logTestResult: async function (body) { return {} },
        plan: async function (body) { return {} },
        setCameraTransform: async function (body) { return {} },
    }
}

function testingModule() {
    function takeAndCompareSnapshot(body) {
        const { srcStoredSnapshot, cameraPosition, cameraTarget, screenshotSize } = body
        const greyPixelDiff = body.greyPixelDiff

        /**
 * the source path in the scene where the screenshot is stored,
 *  the snapshot taken is compared with the stored one
 */
        // srcStoredSnapshot: string;
        // /** the camera position where is set before and while taking the screenshot, relative to base scene */
        // cameraPosition: Vector3 | undefined;
        // /** the camera position where is target to before and while taking the screenshot, relative to base scene */
        // cameraTarget: Vector3 | undefined;
        // /** width x height screenshot size */
        // screenshotSize: Vector2 | undefined;
        // greyPixelDiff?: TakeAndCompareScreenshotRequest_ComparisonMethodGreyPixelDiff | undefined;

        return Deno.core.ops.op_take_and_compare_snapshot(
            srcStoredSnapshot,
            [cameraPosition.x, cameraPosition.y, cameraPosition.z],
            [cameraTarget.x, cameraTarget.y, cameraTarget.z],
            [screenshotSize.x, screenshotSize.y],
            tolerance
        );
    }

    return {
        logTestResult: async function (body) { return {} },
        plan: async function (body) { return {} },
        setCameraTransform: async function (body) { return {} },
        takeAndCompareSnapshot
    }
}

module.exports = testingEnabled ? testingModule() : emptyTesting()