module.exports.getRealm = async function (body) {
    return {
        realmInfo: await Deno.core.ops.op_get_realm()
    }
}
module.exports.getWorldTime = async function (body) {
    // TODO: Implement time when skybox feature has time
    const seconds = 60 * 60 * 12 // noon time in seconds
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
    const sceneInfo = await Deno.core.ops.op_get_scene_information()
    sceneInfo.content = sceneInfo.content.map(item => ({
        hash: item.hash,
        file: item.file
    }))    
    return {
        ...sceneInfo,
    }
}
