module.exports.getSceneInfo = async function (body) {
    const sceneInfo = await Deno.core.ops.op_get_scene_information()
    sceneInfo.content = sceneInfo.content.map(item => ({
        hash: item.hash,
        file: item.file
    }))    
    return {
        cid: sceneInfo.urn,
        contents: sceneInfo.content,
        metadata: sceneInfo.metadataJson,
        baseUrl: sceneInfo.baseUrl
    }
}