module.exports.getUserPublicKey = async function (body) {
    const res = await Deno.core.ops.op_get_player_data("");
    return {
        address: res?.userId
    };
}
module.exports.getUserData = async function (body) {
    const res = await Deno.core.ops.op_get_player_data("");
    // Ops return objects without a prototype; a scene that turns one into text (`'' + data`)
    // throws, so the scene gets a JSON copy.
    return {
        data: res && JSON.parse(JSON.stringify(res))
    };
}