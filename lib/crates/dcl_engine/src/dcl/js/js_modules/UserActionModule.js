// @deprecated, only available for SDK6 compatibility. Use RestrictedActions/TeleportTo
module.exports.requestTeleport = async function (body) {
    const { destination } = body
    // 'magic' (a random place) and 'crowd' (a busy one) are a kernel-era convention: the
    // protocol only defines a free-form destination, and this client has no such lookup. Reject
    // rather than silently substituting a parcel the scene never asked for.
    if (destination === 'magic' || destination === 'crowd') {
        return await Promise.reject(`teleportTo: destination ${destination} is not supported`)
    } else if (!/^\-?\d+\,\-?\d+$/.test(destination)) {
      return await Promise.reject(`teleportTo: invalid destination ${destination}`)
    }

    const coords = destination.split(',');

    // Convert the separate parts to whole numbers.
    let x = parseInt(coords[0], 10);
    let y = parseInt(coords[1], 10);

    return await Deno.core.ops.op_teleport_to(x, y, true, '');
}