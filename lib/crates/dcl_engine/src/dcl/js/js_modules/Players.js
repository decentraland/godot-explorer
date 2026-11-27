module.exports.getPlayerData = async function (body) {
    const res = await Deno.core.ops.op_get_player_data(body.userId);
    // Ops return objects without a prototype; a scene that turns one into text (`'' + data`)
    // throws, so the scene gets a JSON copy.
    return {
        data: res && JSON.parse(JSON.stringify(res))
    };
}

module.exports.getPlayersInScene = async function (body) {
    const res = await Deno.core.ops.op_get_players_in_scene();
    return {
        players: res.map((address) => ({ userId: address }))
    }
}

module.exports.getConnectedPlayers = async function (body) {
    const res = await Deno.core.ops.op_get_connected_players();
    return {
        players: res.map((address) => ({ userId: address }))
    }
}
