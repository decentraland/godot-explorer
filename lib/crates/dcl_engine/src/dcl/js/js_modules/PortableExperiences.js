module.exports.spawn = async function (body) {
    const res = await Deno.core.ops.op_portable_spawn(body.pid, body.ens);
    return res;
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
        loaded: res
    }
}