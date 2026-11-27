
module.exports.getTextureSize = async function (body) {
    const size = await Deno.core.ops.op_get_texture_size(body.src)
    // Ops return objects without a prototype; the scene gets a JSON copy it can turn into text.
    return { src: body.src, size: JSON.parse(JSON.stringify(size)) }
}
