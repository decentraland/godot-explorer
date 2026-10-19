// Unit tests for the JS-side TextEncoder/TextDecoder shim (text_encoding.js).
//
// The shim only talks to the native layer through `Deno.core.ops`, so we stub
// op_utf8_encode / op_utf8_decode with Node's WHATWG-conformant codecs, which
// match the semantics the Rust ops implement (BOM strip unless ignoreBOM,
// fatal vs U+FFFD replacement). No npm dependencies — Node's built-in test
// runner only.
//
// Run with:  node --test lib/src/dcl/js/js_modules/text_encoding.test.cjs

const test = require("node:test");
const assert = require("node:assert/strict");

const native = {
    TextEncoder: globalThis.TextEncoder,
    TextDecoder: globalThis.TextDecoder,
};

globalThis.Deno = {
    core: {
        ops: {
            op_utf8_encode: (text) => new native.TextEncoder().encode(text),
            op_utf8_decode: (bytes, fatal, ignoreBOM) =>
                new native.TextDecoder("utf-8", { fatal, ignoreBOM }).decode(bytes),
        },
    },
};

const { TextEncoder, TextDecoder } = require("./text_encoding.js");

test("decoder accepts the utf-8 label set, case- and whitespace-insensitive", () => {
    for (const label of [
        "unicode-1-1-utf-8",
        "unicode11utf8",
        "unicode20utf8",
        "utf-8",
        "utf8",
        "x-unicode20utf8",
        "UTF-8",
        " \tutf-8\n",
        undefined,
    ]) {
        const decoder = label === undefined ? new TextDecoder() : new TextDecoder(label);
        assert.equal(decoder.encoding, "utf-8");
    }
});

test("decoder rejects any other label with RangeError", () => {
    for (const label of ["utf-16", "latin1", "", "utf 8", "unicode"]) {
        assert.throws(() => new TextDecoder(label), RangeError);
    }
});

test("decoder exposes fatal and ignoreBOM", () => {
    assert.deepEqual(
        (({ fatal, ignoreBOM }) => ({ fatal, ignoreBOM }))(new TextDecoder()),
        { fatal: false, ignoreBOM: false }
    );
    const decoder = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true });
    assert.equal(decoder.fatal, true);
    assert.equal(decoder.ignoreBOM, true);
});

test("decode with no argument returns the empty string", () => {
    assert.equal(new TextDecoder().decode(), "");
});

test("decode accepts ArrayBuffer and views honoring byteOffset/byteLength", () => {
    const bytes = new native.TextEncoder().encode("__héllo…__");
    assert.equal(new TextDecoder().decode(bytes.buffer), "__héllo…__");
    const view = new Uint8Array(bytes.buffer, 2, bytes.length - 4);
    assert.equal(new TextDecoder().decode(view), "héllo…");
    const dataView = new DataView(bytes.buffer, 2, bytes.length - 4);
    assert.equal(new TextDecoder().decode(dataView), "héllo…");
});

test("decode rejects non-buffer input with TypeError", () => {
    assert.throws(() => new TextDecoder().decode("text"), TypeError);
    assert.throws(() => new TextDecoder().decode(null), TypeError);
});

test("decode strips the BOM unless ignoreBOM", () => {
    const bytes = new Uint8Array([0xef, 0xbb, 0xbf, 0x68, 0x69]);
    assert.equal(new TextDecoder().decode(bytes), "hi");
    assert.equal(
        new TextDecoder("utf-8", { ignoreBOM: true }).decode(bytes),
        "﻿hi"
    );
});

test("decode rejects the stream option loudly", () => {
    const bytes = new Uint8Array([0x68]);
    assert.throws(() => new TextDecoder().decode(bytes, { stream: true }), TypeError);
    assert.equal(new TextDecoder().decode(bytes, { stream: false }), "h");
});

test("invalid utf-8: U+FFFD by default, TypeError when fatal", () => {
    const invalid = new Uint8Array([0x61, 0xff, 0x62]);
    assert.equal(new TextDecoder().decode(invalid), "a�b");
    assert.throws(
        () => new TextDecoder("utf-8", { fatal: true }).decode(invalid),
        TypeError
    );
});

test("encoder encodes to utf-8 bytes", () => {
    const encoder = new TextEncoder();
    assert.equal(encoder.encoding, "utf-8");
    assert.deepEqual(encoder.encode(), new Uint8Array(0));
    assert.deepEqual(
        encoder.encode("aé€\u{1d4b3}"),
        new Uint8Array([0x61, 0xc3, 0xa9, 0xe2, 0x82, 0xac, 0xf0, 0x9d, 0x92, 0xb3])
    );
});

test("encodeInto reports read/written when everything fits", () => {
    const dest = new Uint8Array(8);
    const result = new TextEncoder().encodeInto("a\u{1d4b3}b", dest);
    assert.deepEqual(result, { read: 4, written: 6 });
    assert.deepEqual(dest.subarray(6), new Uint8Array(2));
});

test("encodeInto truncates on a utf-8 character boundary", () => {
    const encoder = new TextEncoder();

    let dest = new Uint8Array(2);
    assert.deepEqual(encoder.encodeInto("aé", dest), { read: 1, written: 1 });
    assert.equal(dest[0], 0x61);

    dest = new Uint8Array(3);
    assert.deepEqual(encoder.encodeInto("\u{1d4b3}", dest), { read: 0, written: 0 });

    dest = new Uint8Array(5);
    assert.deepEqual(encoder.encodeInto("a\u{1d4b3}b", dest), { read: 3, written: 5 });
    assert.deepEqual(
        dest,
        new Uint8Array([0x61, 0xf0, 0x9d, 0x92, 0xb3])
    );
});

test("encodeInto requires a Uint8Array destination", () => {
    assert.throws(() => new TextEncoder().encodeInto("a", new Uint16Array(4)), TypeError);
    assert.throws(() => new TextEncoder().encodeInto("a", []), TypeError);
});
