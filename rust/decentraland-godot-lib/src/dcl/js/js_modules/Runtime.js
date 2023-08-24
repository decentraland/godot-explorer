module.exports.getRealm = async function (body) { return {} }
module.exports.getWorldTime = async function (body) { return {} }

// sync implementation
module.exports.readFile = async function (body) {
    // body.fileName

    op_crdt_send_to_renderer(body.fileName);

    const response = {
        content: new Uint8Array(),
        hash: "string"
    }
    return response
}
module.exports.getSceneInformation = async function (body) { return {} }