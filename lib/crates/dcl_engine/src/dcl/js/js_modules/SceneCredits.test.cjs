// Unit tests for the explorer's `~system/SceneCredits` module (SceneCredits.js), which never pays Credits.
// No npm dependencies — Node's built-in test runner only.
//
// Run with:  node --test lib/crates/dcl_engine/src/dcl/js/js_modules/SceneCredits.test.cjs

const test = require("node:test");
const assert = require("node:assert/strict");

const { GrantCreditsError, grantCredits } = require("./SceneCredits.js");

// scene_credits.proto's GrantCreditsError, plus ts-proto's UNRECOGNIZED.
const PROTOCOL_ENUM = [
    ["GCE_UNSPECIFIED", 0],
    ["GCE_NOT_AVAILABLE", 1],
    ["GCE_INVALID_REQUEST", 2],
    ["GCE_RATE_LIMITED", 3],
    ["GCE_UNAVAILABLE", 4],
    ["GCE_NOT_AUTHORIZED", 5],
    ["GCE_RECIPIENT_NOT_CONNECTED", 6],
    ["GCE_PROGRAM_PAUSED", 7],
    ["GCE_OUTSIDE_SCHEDULE", 8],
    ["GCE_BUDGET_EXHAUSTED", 9],
    ["GCE_PROGRAM_CAP_REACHED", 10],
    ["GCE_PLAYER_CAP_REACHED", 11],
    ["GCE_REQUIREMENT_NOT_MET", 12],
    ["GCE_RECIPIENT_NOT_ELIGIBLE", 13],
    ["UNRECOGNIZED", -1],
];

test("grantCredits resolves GCE_NOT_AVAILABLE whatever the request", async () => {
    const requests = [
        {
            programId: "00000000-0000-0000-0000-000000000000",
            recipient: "0x" + "0".repeat(40),
            outcomeId: "match-1",
        },
        {},
        undefined,
    ];
    for (const request of requests) {
        assert.deepEqual(await grantCredits(request), { ok: false, error: 1 });
    }
});

test("GrantCreditsError has the protocol's names and numbers, with the reverse mapping", () => {
    for (const [name, value] of PROTOCOL_ENUM) {
        assert.equal(GrantCreditsError[name], value);
        assert.equal(GrantCreditsError[value], name);
    }
    assert.equal(Object.keys(GrantCreditsError).length, PROTOCOL_ENUM.length * 2);
});
