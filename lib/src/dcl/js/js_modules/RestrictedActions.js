module.exports.movePlayerTo = async function (body) {
  Deno.core.ops.op_move_player_to(
    body.newRelativePosition.x,
    body.newRelativePosition.y,
    body.newRelativePosition.z,
    body.cameraTarget?.x ?? NaN,
    body.cameraTarget?.y ?? NaN,
    body.cameraTarget?.z ?? NaN,
    body.avatarTarget?.x ?? NaN,
    body.avatarTarget?.y ?? NaN,
    body.avatarTarget?.z ?? NaN
  );
  return {};
};
// `worldCoordinates` and `realm` are both optional (protocol#477): omitted coordinates mean
// the realm's default spawn, an omitted realm means the player's current one. The op boundary
// can't take optionals, so they travel as sentinels (same idiom as movePlayerTo's NaNs).
module.exports.teleportTo = async function (body) {
  const coords = body.worldCoordinates;
  return await Deno.core.ops.op_teleport_to(
    coords ? parseInt(coords.x) : 0,
    coords ? parseInt(coords.y) : 0,
    coords != null,
    body.realm ?? ''
  );
};
module.exports.triggerEmote = async function (body) {
  // mask: optional AvatarMask enum (AM_UPPER_BODY = 0); -1 sentinel = full body.
  return await Deno.core.ops.op_trigger_emote(body.predefinedEmote, body.mask ?? -1);
};
module.exports.triggerSceneEmote = async function (body) {
  const loop = body.loop ?? false;
  return await Deno.core.ops.op_trigger_scene_emote(body.src, loop, body.mask ?? -1);
};
// StopEmoteRequest is empty (restricted_actions.proto): the stop always targets
// whatever the player is playing right now, so there is nothing to pass.
module.exports.stopEmote = async function () {
  return await Deno.core.ops.op_stop_emote();
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
