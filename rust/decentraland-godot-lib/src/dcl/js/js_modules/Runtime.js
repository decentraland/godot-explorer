module.exports.getRealm = async function (body) { return {} }
module.exports.getWorldTime = async function (body) { return {} }
module.exports.readFile = async function (body) {
    console.log('readFile')
    const { hash, content } = await Deno.core.ops.op_read_file(body.fileName)
    console.log('get_file_hash', hash)

    const data = new Uint8Array(content)

    console.log('content', content.length)
    return {
        content: data,
        hash
    }
}
module.exports.getSceneInformation = async function (body) { return {} }
