module.exports.getRealm = async function (body) { return {} }
module.exports.getWorldTime = async function (body) { return {} }

// sync implementation
module.exports.readFile = async function (body) {
    // body.fileName

    const fileBody = op_read_file(body.fileName);
    if (!fileBody) {
        throw new Error("File not found")
    }

    const response = {
        content: fileBody,
        hash: "string"
    }
    return response
}
module.exports.getSceneInformation = async function (body) { return {} }