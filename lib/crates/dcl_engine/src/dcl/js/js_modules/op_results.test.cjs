// The scene APIs that pass an op's object on to the scene must hand back ordinary objects.
// serde_v8 builds them without a prototype, and `'' + obj` then throws "Cannot convert object
// to primitive value" (Meta GamiMall, 1,95, died on that at load).
//
// Run with:  node --test lib/crates/dcl_engine/src/dcl/js/js_modules/op_results.test.cjs

const test = require("node:test");
const assert = require("node:assert/strict");

// What serde_v8 produces for a Rust struct: no prototype, nested objects alike.
function opObject(fields) {
    const obj = Object.create(null);
    for (const [key, value] of Object.entries(fields)) {
        obj[key] = value && typeof value === "object" && !Array.isArray(value) ? opObject(value) : value;
    }
    return obj;
}

const player = () =>
    opObject({
        userId: "0x" + "1".repeat(40),
        displayName: "Neo",
        avatar: { bodyShape: "urn:decentraland:off-chain:base-avatars:BaseMale", skinColor: { r: 1, g: 0.8, b: 0.6, a: 1 } },
    });

globalThis.Deno = {
    core: {
        ops: {
            op_get_player_data: async () => player(),
            op_get_realm: () =>
                opObject({ baseUrl: "https://peer.decentraland.org", realmName: "main", networkId: 1, commsAdapter: "", isPreview: false }),
            op_portable_spawn: async () => opObject({ pid: "p1", parentCid: "", name: "px", ens: null }),
            op_portable_list: async () => [opObject({ pid: "p1", parentCid: "", name: "px", ens: null })],
            op_get_texture_size: async () => opObject({ width: 512, height: 256 }),
        },
    },
};

function assertOrdinary(value, path) {
    assert.equal(Object.getPrototypeOf(value), Array.isArray(value) ? Array.prototype : Object.prototype, path);
    assert.doesNotThrow(() => "" + value, path);
    for (const [key, child] of Object.entries(value)) {
        if (child && typeof child === "object") assertOrdinary(child, `${path}.${key}`);
    }
}

test("getUserData returns the player as ordinary objects", async () => {
    const { data } = await require("./UserIdentity.js").getUserData({});
    assertOrdinary(data, "data");
    assert.equal(data.avatar.skinColor.g, 0.8);
});

test("getPlayerData returns the player as ordinary objects", async () => {
    const { data } = await require("./Players.js").getPlayerData({ userId: "0x1" });
    assertOrdinary(data, "data");
    assert.equal(data.displayName, "Neo");
});

test("getUserData and getPlayerData keep a missing player as null", async () => {
    Deno.core.ops.op_get_player_data = async () => null;
    try {
        assert.equal((await require("./UserIdentity.js").getUserData({})).data, null);
        assert.equal((await require("./Players.js").getPlayerData({ userId: "0x2" })).data, null);
    } finally {
        Deno.core.ops.op_get_player_data = async () => player();
    }
});

test("getRealm returns the realm as an ordinary object", async () => {
    const { realmInfo } = await require("./Runtime.js").getRealm({});
    assertOrdinary(realmInfo, "realmInfo");
    assert.equal(realmInfo.realmName, "main");
});

test("portable experience spawn and list return ordinary objects", async () => {
    const pe = require("./PortableExperiences.js");
    assertOrdinary(await pe.spawn({ pid: "p1" }), "spawn");
    assertOrdinary((await pe.getPortableExperiencesLoaded({})).loaded, "loaded");
});

test("getTextureSize returns the size as an ordinary object", async () => {
    const { size } = await require("./AdaptationLayerHelper.js").getTextureSize({ src: "a.png" });
    assertOrdinary(size, "size");
    assert.equal(size.width, 512);
});
