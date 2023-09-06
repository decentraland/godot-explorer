module.exports.getRealm = async function (body) { return {} }
module.exports.getWorldTime = async function (body) { return {} }

// sync implementation
module.exports.readFile = async function (body) {
    // body.fileName
    const { hash, content } = await Deno.core.ops.op_read_file(body.fileName)
    const data = new Uint8Array(content)
    const response = {
        content: data,
        hash
    }
    return response
}
module.exports.getSceneInformation = async function (body) { return {} }