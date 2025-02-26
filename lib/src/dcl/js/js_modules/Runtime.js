module.exports.getRealm = async function (body) {
    return {
        realmInfo: Deno.core.ops.op_get_realm()
    }
}
module.exports.getWorldTime = async function (body) {
    const seconds = Deno.core.ops.op_get_world_time()
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

module.exports.getExplorerInformation = async function (body) {
    return {
        agent: 'godot',
        platform: 'desktop',
        configurations: {}
    }
}
