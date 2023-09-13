module.exports.getRealm = async function (body) { return {} }
module.exports.getWorldTime = async function (body) { return {} }

// sync implementation
module.exports.readFile = async function (body) {
    // body.fileName
    const { hash, url } = Deno.core.ops.op_get_file_url(body.fileName)
    const response = await fetch(url)
    const bytes = await response._internal_bytes()
    const data = new Uint8Array(bytes)
    return {
        content: data,
        hash
    }
}
module.exports.getSceneInformation = async function (body) { return {} }