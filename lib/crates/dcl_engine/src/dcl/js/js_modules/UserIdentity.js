module.exports.getUserPublicKey = async function (body) {
    const res = await Deno.core.ops.op_get_player_data("");
    return {
        address: res?.userId
    };
}
module.exports.getUserData = async function (body) {
    const res = await Deno.core.ops.op_get_player_data("");
    return {
        data: res
    };
}