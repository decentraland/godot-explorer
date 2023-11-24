module.exports.movePlayerTo = async function (body) {
  const position = body.newRelativePosition;
  if ("cameraTarget" in body) {
    return await Deno.core.ops.op_move_player_to(
      [position.x, position.y, position.z],
      [body.cameraTarget.x, body.cameraTarget.y, body.cameraTarget.z]
    );
  } else {
    return await Deno.core.ops.op_move_player_to([
      position.x,
      position.y,
      position.z,
    ]);
  }
};
module.exports.teleportTo = async function (body) {
  return await Deno.core.ops.op_teleport_to([
    body.worldCoordinates.x,
    body.worldCoordinates.y,
  ]);
};
module.exports.triggerEmote = async function (body) {
  return await Deno.core.ops.op_trigger_emote(body.predefinedEmote);
};
module.exports.triggerSceneEmote = async function (body) {
  return await Deno.core.ops.op_trigger_scene_emote(body.src, body.looping);
};
module.exports.changeRealm = async function (body) {
  return await Deno.core.ops.op_change_realm(
    body.realm,
    body.message
  );
};
module.exports.openExternalUrl = async function (body) {
  return await Deno.core.ops.op_open_external_url(
    body.url,
  );
};
module.exports.openNftDialog = async function (body) {
  return await Deno.core.ops.op_open_nft_dialog(
    body.urn,
  );
};

// Reference Client doesn't have it. No implement it until decide what to do with it...
module.exports.setCommunicationsAdapter = async function (body) {
  return {};
};
