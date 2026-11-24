// Unit tests for the explorer's `~system/SceneRewards` module (SceneRewards.js), which never pays a reward.
// No npm dependencies — Node's built-in test runner only.
//
// Run with:  node --test lib/crates/dcl_engine/src/dcl/js/js_modules/SceneRewards.test.cjs

const test = require("node:test");
const assert = require("node:assert/strict");

const { GrantRewardError, grantReward } = require("./SceneRewards.js");

// scene_rewards.proto's GrantRewardError, plus ts-proto's UNRECOGNIZED.
const PROTOCOL_ENUM = [
    ["GRE_UNSPECIFIED", 0],
    ["GRE_NOT_AVAILABLE", 1],
    ["GRE_INVALID_REQUEST", 2],
    ["GRE_RATE_LIMITED", 3],
    ["GRE_UNAVAILABLE", 4],
    ["GRE_NOT_AUTHORIZED", 5],
    ["GRE_RECIPIENT_NOT_CONNECTED", 6],
    ["GRE_PROGRAM_PAUSED", 7],
    ["GRE_OUTSIDE_SCHEDULE", 8],
    ["GRE_BUDGET_EXHAUSTED", 9],
    ["GRE_PROGRAM_CAP_REACHED", 10],
    ["GRE_PLAYER_CAP_REACHED", 11],
    ["GRE_REQUIREMENT_NOT_MET", 12],
    ["GRE_RECIPIENT_NOT_ELIGIBLE", 13],
    ["UNRECOGNIZED", -1],
];

test("grantReward resolves GRE_NOT_AVAILABLE whatever the request", async () => {
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
        assert.deepEqual(await grantReward(request), { ok: false, error: 1 });
    }
});

test("GrantRewardError has the protocol's names and numbers, with the reverse mapping", () => {
    for (const [name, value] of PROTOCOL_ENUM) {
        assert.equal(GrantRewardError[name], value);
        assert.equal(GrantRewardError[value], name);
    }
    assert.equal(Object.keys(GrantRewardError).length, PROTOCOL_ENUM.length * 2);
});
