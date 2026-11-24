// `~system/SceneRewards` (protocol scene_rewards.proto). Only a scene's authoritative server can pay a reward, so in
// the explorer grantReward always resolves GRE_NOT_AVAILABLE. The module exists so a scene that imports it still loads.

// The protocol enum as a runtime value, shaped like the generated ts-proto enum.
function makeEnum(entries) {
    const e = {};
    for (const [name, value] of entries) {
        e[name] = value;
        e[value] = name;
    }
    return e;
}

module.exports.GrantRewardError = makeEnum([
    ['GRE_UNSPECIFIED', 0],
    ['GRE_NOT_AVAILABLE', 1],
    ['GRE_INVALID_REQUEST', 2],
    ['GRE_RATE_LIMITED', 3],
    ['GRE_UNAVAILABLE', 4],
    ['GRE_NOT_AUTHORIZED', 5],
    ['GRE_RECIPIENT_NOT_CONNECTED', 6],
    ['GRE_PROGRAM_PAUSED', 7],
    ['GRE_OUTSIDE_SCHEDULE', 8],
    ['GRE_BUDGET_EXHAUSTED', 9],
    ['GRE_PROGRAM_CAP_REACHED', 10],
    ['GRE_PLAYER_CAP_REACHED', 11],
    ['GRE_REQUIREMENT_NOT_MET', 12],
    ['GRE_RECIPIENT_NOT_ELIGIBLE', 13],
    ['UNRECOGNIZED', -1],
]);

module.exports.grantReward = async function (body) {
    return { ok: false, error: module.exports.GrantRewardError.GRE_NOT_AVAILABLE };
};
