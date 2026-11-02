const UTF8_LABELS = new Set([
    'unicode-1-1-utf-8',
    'unicode11utf8',
    'unicode20utf8',
    'utf-8',
    'utf8',
    'x-unicode20utf8'
]);

class TextDecoder {
    constructor(label = 'utf-8', options = {}) {
        const normalized = String(label).replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, '').toLowerCase();
        if (!UTF8_LABELS.has(normalized)) {
            throw new RangeError(`The encoding label provided ('${label}') is invalid (only utf-8 is supported)`);
        }
        this.encoding = 'utf-8';
        this.fatal = !!(options && options.fatal);
        this.ignoreBOM = !!(options && options.ignoreBOM);
    }

    decode(input, options) {
        if (options && options.stream) {
            throw new TypeError('TextDecoder.decode: the stream option is not supported');
        }
        if (input === undefined) {
            return '';
        }
        let bytes;
        if (input instanceof ArrayBuffer) {
            bytes = new Uint8Array(input);
        } else if (ArrayBuffer.isView(input)) {
            bytes = new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
        } else {
            throw new TypeError('TextDecoder.decode: input must be an ArrayBuffer or ArrayBufferView');
        }
        try {
            return Deno.core.ops.op_utf8_decode(bytes, this.fatal, this.ignoreBOM);
        } catch (_) {
            throw new TypeError('TextDecoder.decode: the encoded data is not valid utf-8');
        }
    }
}

class TextEncoder {
    constructor() {
        this.encoding = 'utf-8';
    }

    encode(input = '') {
        return Deno.core.ops.op_utf8_encode(String(input));
    }

    encodeInto(source, destination) {
        if (!(destination instanceof Uint8Array)) {
            throw new TypeError('TextEncoder.encodeInto: destination must be a Uint8Array');
        }
        const text = String(source);
        const bytes = Deno.core.ops.op_utf8_encode(text);
        if (bytes.length <= destination.length) {
            destination.set(bytes);
            return { read: text.length, written: bytes.length };
        }
        let written = destination.length;
        while (written > 0 && (bytes[written] & 0xc0) === 0x80) {
            written -= 1;
        }
        const prefix = bytes.subarray(0, written);
        destination.set(prefix);
        // read = UTF-16 length of the written prefix: the op encodes lone
        // surrogates as U+FFFD (1 unit), so encode/decode round-trip unit-for-unit
        const read = Deno.core.ops.op_utf8_decode(prefix, false, true).length;
        return { read, written };
    }
}

module.exports.TextDecoder = TextDecoder;
module.exports.TextEncoder = TextEncoder;
