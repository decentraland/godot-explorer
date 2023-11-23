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
        const { id, cameraPosition, cameraTarget, snapshotFrameSize, tolerance } = body

        return Deno.core.ops.op_take_and_compare_snapshot(
            id,
            [cameraPosition.x, cameraPosition.y, cameraPosition.z],
            [cameraTarget.x, cameraTarget.y, cameraTarget.z],
            [snapshotFrameSize.x, snapshotFrameSize.y],
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