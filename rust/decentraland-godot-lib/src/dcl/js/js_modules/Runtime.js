module.exports.getRealm = async function (body) {
    return {
        realmInfo: await Deno.core.ops.op_get_realm()
    }
}
module.exports.getWorldTime = async function (body) {
    const seconds = 60 * 60 * 12 // noon seconds
    return {
        seconds
    }
}

module.exports.readFile = async function (body) {
    const { hash, url } = Deno.core.ops.op_get_file_url(body.fileName)
    const response = await fetch(url)
    const bytes = await response.bytes()
    const data = new Uint8Array(bytes)
    return {
        content: data,
        hash
    }
}

module.exports.getSceneInformation = async function (body) {
    return {
        urn: "", // this is either the entityId or the full URN of the scene that is running
        content: [], // contents of the deployed entities
        metadataJson: "", // JSON serialization of the entity.metadata field
        baseUrl: "" // baseUrl used to resolve all content files
    }
}
