// engine module
const { op_crdt_recv_wait, op_crdt_recv_drain, op_crdt_send_to_renderer, op_subscribe, op_send_batch } = Deno.core.ops;

// Persistent recv buffer reused across every CRDT round-trip. 64 KB initial
// covers typical steady-state recvs (~1-25 KB); the buffer auto-grows on
// demand for the main_crdt snapshot on first recv per scene.
let _recvBuf = new Uint8Array(64 * 1024);

// Decode the framed buffer op_crdt_recv_drain wrote:
//   [u32 LE main_crdt_len][main_crdt bytes][data bytes]
// `slice()` per message (not `subarray()`) because the next recv overwrites
// `_recvBuf` and SDK7 may hold references to the returned Uint8Arrays.
function decodeRecvFrame(len) {
    const view = new DataView(_recvBuf.buffer, _recvBuf.byteOffset, len);
    const mainLen = view.getUint32(0, true);
    const headerEnd = 4 + mainLen;
    const messages = [];
    if (mainLen > 0) {
        messages.push(_recvBuf.slice(4, headerEnd));
    }
    messages.push(_recvBuf.slice(headerEnd, len));
    return messages;
}

async function recvFramed() {
    const len = await op_crdt_recv_wait();
    if (len > _recvBuf.byteLength) {
        // Grow to at least the needed size, with a 2× headroom so we don't
        // re-grow every burst. Powers of two keep the allocator happy.
        const newSize = Math.max(len, _recvBuf.byteLength * 2);
        _recvBuf = new Uint8Array(newSize);
    }
    op_crdt_recv_drain(_recvBuf);
    return decodeRecvFrame(len);
}

module.exports.crdtSendToRenderer = async function (messages) {
    op_crdt_send_to_renderer(messages.data.buffer.slice(messages.data.byteOffset, messages.data.byteLength + messages.data.byteOffset));
    return { data: await recvFramed() };
}


module.exports.crdtGetState = async function () {
    return { data: await recvFramed() };
}


module.exports.isServer = async function () {
    return {
        isServer: false
    }
}

/**
 * @deprecated this is an SDK6 API.
 * This function subscribe to an event from the renderer
 */
module.exports.subscribe = async function (message) {
    op_subscribe(message.eventId);
}

/**
 * @deprecated this is an SDK6 API.
 * This function unsubscribe to an event from the renderer
 */
module.exports.unsubscribe = async function (message) {
    op_unsubscribe(message.eventId);
}

/**
 * @deprecated this is an SDK6 API.
 * This function polls events from the renderer
 */
module.exports.sendBatch = async function () {
    return { events: op_send_batch() }
}
