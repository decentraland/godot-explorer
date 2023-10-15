module.exports.movePlayerTo = async function (body) { return {} }
module.exports.teleportTo = async function (body) { return {} }
module.exports.triggerEmote = async function (body) { return {} }
module.exports.changeRealm = function (body) {
    return Deno.core.ops.op_change_realm(body.realm, body.message);
}
module.exports.openExternalUrl = async function (body) { return {} }
module.exports.openNftDialog = async function (body) { return {} }
module.exports.setCommunicationsAdapter = async function (body) { return {} }