module.exports.spawn = async function (body) {
    const res = await Deno.core.ops.op_portable_spawn(body.pid, body.ens);
    // Ops return objects without a prototype; a scene that turns one into text throws, so the
    // scene gets JSON copies (here and in getPortableExperiencesLoaded).
    return JSON.parse(JSON.stringify(res));
}
module.exports.kill = async function (body) {
    const res = await Deno.core.ops.op_portable_kill(body.pid);
    return {
        status: res
    }
}
module.exports.exit = async function (body) {
    console.error("PortableExperiences::exit not implemented", body)
    return {}
}
module.exports.getPortableExperiencesLoaded = async function (body) {
    const res = await Deno.core.ops.op_portable_list();
    return {
        loaded: JSON.parse(JSON.stringify(res))
    }
}