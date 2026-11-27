// `~system/SceneCredits` (protocol scene_credits.proto). Only a scene's authoritative server can pay Credits, so in
// the explorer grantCredits always resolves GCE_NOT_AVAILABLE. The module exists so a scene that imports it still loads.

// The protocol enum as a runtime value, shaped like the generated ts-proto enum.
function makeEnum(entries) {
    const e = {};
    for (const [name, value] of entries) {
        e[name] = value;
        e[value] = name;
    }
    return e;
}

module.exports.GrantCreditsError = makeEnum([
    ['GCE_UNSPECIFIED', 0],
    ['GCE_NOT_AVAILABLE', 1],
    ['GCE_INVALID_REQUEST', 2],
    ['GCE_RATE_LIMITED', 3],
    ['GCE_UNAVAILABLE', 4],
    ['GCE_NOT_AUTHORIZED', 5],
    ['GCE_RECIPIENT_NOT_CONNECTED', 6],
    ['GCE_PROGRAM_PAUSED', 7],
    ['GCE_OUTSIDE_SCHEDULE', 8],
    ['GCE_BUDGET_EXHAUSTED', 9],
    ['GCE_PROGRAM_CAP_REACHED', 10],
    ['GCE_PLAYER_CAP_REACHED', 11],
    ['GCE_REQUIREMENT_NOT_MET', 12],
    ['GCE_RECIPIENT_NOT_ELIGIBLE', 13],
    ['UNRECOGNIZED', -1],
]);

module.exports.grantCredits = async function (body) {
    return { ok: false, error: module.exports.GrantCreditsError.GCE_NOT_AVAILABLE };
};
