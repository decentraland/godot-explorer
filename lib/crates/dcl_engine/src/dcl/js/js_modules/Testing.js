const env = require('env')

const testingEnabled = env['testing_enable']

function emptyTesting() {
    console.log("Testing disabled")
    return {
        logTestResult: async function (body) { return {} },
        plan: async function (body) { return {} },
        setCameraTransform: async function (body) { return {} },
        takeAndCompareScreenshot: async function (body) { return {} },
        simulateInput: async function (body) { return {} },
        simulateGesture: async function (body) { return {} },
        testCommand: async function (body) { throw new Error("Testing disabled") }
    }
}

function testingModule() {
    console.log("Testing enabled")
    function takeAndCompareScreenshot(body) {
        const { srcStoredSnapshot, cameraPosition, cameraTarget, screenshotSize } = body
        const methods = {
            grey_pixel_diff: body.greyPixelDiff
        }

        return Deno.core.ops.op_take_and_compare_snapshot(
            srcStoredSnapshot,
            [cameraPosition.x, cameraPosition.y, cameraPosition.z],
            [cameraTarget.x, cameraTarget.y, cameraTarget.z],
            [screenshotSize.x, screenshotSize.y],
            methods
        );
    }

    return {
        logTestResult: async function (body) {
            Deno.core.ops.op_log_test_result(body);
            return {}
        },
        plan: async function (body) {
            Deno.core.ops.op_log_test_plan(body);
            return {}
        },
        setCameraTransform: async function (body) { return {} },
        takeAndCompareScreenshot,
        // Godot-only: presses or releases a player action (ia_forward, ia_jump, ...).
        simulateInput: async function (body) {
            const strength = body.strength === undefined ? 1 : body.strength
            Deno.core.ops.op_simulate_input(body.action, body.pressed, strength, body.frames || 0);
            return {}
        },
        // Godot-only: a named touch gesture through the mobile controls.
        simulateGesture: async function (body) {
            const v = body.direction || body.delta || { x: body.spread || 0, y: 0 }
            const strength = body.strength === undefined ? 1 : body.strength
            Deno.core.ops.op_simulate_gesture(body.gesture, v.x, v.y, strength, body.frames || 1);
            return {}
        },
        // Godot-only: a shell command, a setting change or a debug-hub query; resolves to its JSON reply.
        testCommand: async function (body) {
            const reply = await Deno.core.ops.op_test_command(body.kind, JSON.stringify(body.payload || {}));
            return JSON.parse(reply)
        }
    }
}

module.exports = testingEnabled ? testingModule() : emptyTesting()