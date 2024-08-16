module.exports.getBootstrapData = async function (body) {
    const sceneInfo = await Deno.core.ops.op_get_scene_information()
    sceneInfo.content = sceneInfo.content.map(item => ({
        hash: item.hash,
        file: item.file
    }))    
    return {
        id: sceneInfo.urn,
        baseUrl: sceneInfo.baseUrl,
        entity: {
            content: sceneInfo.content,
            metadataJson: sceneInfo.metadataJson
        },
        useFPSThrottling: false,
    }
}
module.exports.isPreviewMode = async function (body) {
    const realm = Deno.core.ops.op_get_realm()
    return {
        isPreview: realm.isPreview
    }
}
module.exports.getPlatform = async function (body) {
    return {
        platform: 'desktop' // TODO: Implement `vr`, `web`, `mobile` it's ready
    }
}
module.exports.areUnsafeRequestAllowed = async function (body) {
    return {
        status: false
    }
}
module.exports.getCurrentRealm = async function (body) {
    const realm = Deno.core.ops.op_get_realm()
    return {
        currentRealm: {
            protocol: 'v3',
            layer: '', // layer doesn't exists anymore
            room: '',
            serverName: realm.realmName,
            displayName: realm.realmName,
            domain: realm.baseUrl,
        }
    }
}
module.exports.getExplorerConfiguration = async function (body) {
    return {
        clientUri: '',
        configurations: {}
    }
}
module.exports.getDecentralandTime = async function (body) {
    const seconds = 60 * 60 * 12 // noon time in seconds
    return {
        seconds
    }
}