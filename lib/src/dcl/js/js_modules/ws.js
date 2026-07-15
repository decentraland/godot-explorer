
// /// --- WebSocket ---
//
// A WHATWG-ish WebSocket implementation backed by the native `op_ws_*` ops.
// It supports both the `on*` handler properties and addEventListener, reports
// the standard readyState values, delivers binary frames as ArrayBuffer (to
// match binaryType === 'arraybuffer'), and threads close code/reason in both
// directions.

class WebSocket {
    // WHATWG WebSocket `readyState` values. These MUST match the standard
    // (CONNECTING=0, OPEN=1, CLOSING=2, CLOSED=3): scenes and libraries such as
    // Colyseus compare `ws.readyState === WebSocket.OPEN` to decide whether it is
    // safe to send. Using non-standard values here silently breaks them.
    static CONNECTING = 0
    static OPEN = 1
    static CLOSING = 2
    static CLOSED = 3

    constructor(url, protocols) {
        this.url = url
        this.protocols = protocols

        this._readyState = WebSocket.CONNECTING
        this._protocol = ""
        this._bufferedAmount = 0
        // Null-prototype so inherited names ('toString', '__proto__', ...) passed
        // to addEventListener/dispatchEvent are ignored instead of matching.
        this._listeners = Object.create(null)
        this._listeners.open = new Set()
        this._listeners.message = new Set()
        this._listeners.close = new Set()
        this._listeners.error = new Set()

        this._internal_ws_id = Deno.core.ops.op_ws_create(url, protocols ?? [])

        this.onclose = null
        this.onerror = null
        this.onmessage = null
        this.onopen = null

        this._poll().catch(console.error)
    }

    get bufferedAmount() {
        return this._bufferedAmount
    }

    get readyState() {
        return this._readyState
    }

    // The spec exposes the state constants on instances too, not just the class.
    get CONNECTING() {
        return WebSocket.CONNECTING
    }

    get OPEN() {
        return WebSocket.OPEN
    }

    get CLOSING() {
        return WebSocket.CLOSING
    }

    get CLOSED() {
        return WebSocket.CLOSED
    }

    get binaryType() {
        return "arraybuffer"
    }

    set binaryType(value) {
        if (value !== "arraybuffer") {
            throw new Error("Only 'arraybuffer' is supported as binaryType")
        }
    }

    // The subprotocol the server selected during the handshake (empty if none).
    get protocol() {
        return this._protocol
    }

    // TODO: implement
    get extensions() {
        return ""
    }

    addEventListener(type, listener) {
        const set = this._listeners[type]
        if (set && typeof listener === 'function') {
            set.add(listener)
        }
    }

    removeEventListener(type, listener) {
        const set = this._listeners[type]
        if (set) {
            set.delete(listener)
        }
    }

    dispatchEvent(event) {
        this._emit(event.type, event)
        return true
    }

    // Dispatch to the matching `on<type>` handler and any addEventListener
    // listeners, isolating handler exceptions.
    _emit(type, event) {
        const handler = this["on" + type]
        if (typeof handler === 'function') {
            try {
                handler.call(this, event)
            } catch (err) {
                console.error(err)
            }
        }
        const set = this._listeners[type]
        if (set) {
            for (const listener of set) {
                try {
                    listener.call(this, event)
                } catch (err) {
                    console.error(err)
                }
            }
        }
    }

    send(data) {
        if (this._readyState === WebSocket.CONNECTING) {
            // Matches the browser: sending before the socket is open is an error.
            throw new Error("InvalidStateError: still in CONNECTING state")
        }
        if (this._readyState !== WebSocket.OPEN) {
            // CLOSING / CLOSED: per spec the data is silently discarded.
            return
        }

        let payload
        let size
        if (typeof data === 'string') {
            payload = { "type": "Text", data }
            size = data.length
        } else if (data instanceof ArrayBuffer) {
            payload = { "type": "Binary", data: Array.from(new Uint8Array(data)) }
            size = data.byteLength
        } else if (ArrayBuffer.isView(data)) {
            // Uint8Array and any other typed-array / DataView view.
            const view = new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
            payload = { "type": "Binary", data: Array.from(view) }
            size = data.byteLength
        } else if (Array.isArray(data)) {
            payload = { "type": "Binary", data }
            size = data.length
        } else {
            console.error(`Unsupported data type: ${typeof data}`, data)
            throw new Error("Unsupported data type")
        }

        // Best-effort bufferedAmount: bytes handed to the native queue that have
        // not been accepted yet. Lets scenes throttle with the standard idiom.
        this._bufferedAmount += size
        const settle = () => {
            this._bufferedAmount -= size
        }
        Deno.core.ops.op_ws_send(this._internal_ws_id, payload).then(settle).catch((err) => {
            settle()
            console.error(err)
        })
    }

    close(code, reason) {
        if (this._readyState === WebSocket.CLOSING || this._readyState === WebSocket.CLOSED) {
            return
        }
        if (code !== undefined && code !== 1000 && !(code >= 3000 && code <= 4999)) {
            throw new Error("InvalidAccessError: close code must be 1000 or in the range 3000-4999")
        }
        this._readyState = WebSocket.CLOSING
        Deno.core.ops
            .op_ws_send(this._internal_ws_id, {
                "type": "Close",
                code: code ?? null,
                reason: reason ?? null,
            })
            .catch(console.error)
    }

    async _poll() {
        const self = this

        // Deliver the close event exactly once. Per the WHATWG spec an error is
        // always followed by a close, and a connection must never emit `close`
        // more than once. Default to 1006 (abnormal closure) when no close frame
        // was received so libraries can distinguish a drop from a clean leave.
        let closeFired = false
        function fireClose(code, reason) {
            if (closeFired) {
                return
            }
            closeFired = true
            self._readyState = WebSocket.CLOSED
            self._emit("close", {
                type: "close",
                code: typeof code === 'number' ? code : 1006,
                reason: reason ?? "",
                // A numeric code means a close frame was received, i.e. a clean
                // closing handshake (any code, not just 1000).
                wasClean: typeof code === 'number',
            })
        }

        async function poll_from_native() {
            const data = await Deno.core.ops.op_ws_poll(self._internal_ws_id)

            switch (data.type) {
                case "BinaryData":
                    // Deliver a real ArrayBuffer, matching binaryType.
                    self._emit("message", {
                        type: "message",
                        data: new Uint8Array(data.data).buffer,
                    })
                    break
                case "TextData":
                    self._emit("message", { type: "message", data: data.data })
                    break
                case "Connected":
                    self._protocol = data.protocol ?? ""
                    // Transition to OPEN so `readyState` reports OPEN. Libraries
                    // that gate sends on `readyState === WebSocket.OPEN` (e.g.
                    // Colyseus `isOpen`) would otherwise buffer/drop every message.
                    // Ignore if the scene already called close() while CONNECTING.
                    if (self._readyState === WebSocket.CONNECTING) {
                        self._readyState = WebSocket.OPEN
                        self._emit("open", { type: "open" })
                    }
                    break
                case "Closed":
                    fireClose(data.code, data.reason)
                    return false
                default:
                    throw new Error("unreached")
            }
            return true
        }
        await new Promise((resolve) => setImmediate(resolve))

        try {
            while (true) {
                if (!(await poll_from_native())) {
                    break
                }
            }
        } catch (err) {
            self._emit("error", {
                type: "error",
                error: err,
                message: String((err && err.message) || err),
            })
            // Abnormal closure: no close frame was received from the peer.
            fireClose(1006, "")
        }

        // Safety net: guarantee a single close event on any exit path.
        fireClose(1006, "")
        Deno.core.ops.op_ws_cleanup(self._internal_ws_id)
    }
}

class RestrictedWebSocket extends WebSocket {
    constructor(url, protocols) {
        // Default to permissive (preview) if the realm info is unavailable, so a
        // missing op can never break WebSocket entirely.
        let previewMode = true
        try {
            previewMode = !!Deno.core.ops.op_get_realm().isPreview
        } catch (_err) {
            previewMode = true
        }
        // TODO: gate on the scene's declared USE_WEBSOCKET permission once a
        // permission model exists in the client (fetch.js has the same gap).
        const canUseWebsocket = true

        if (url.toString().toLowerCase().substr(0, 4) !== 'wss:') {
            if (previewMode) {
                console.log(
                    "⚠️ Warning: can't connect to unsafe WebSocket (ws) server in deployed scenes, consider upgrading to wss."
                )
            } else {
                throw new Error("Can't connect to unsafe WebSocket server, please upgrade to wss.")
            }
        }

        if (!canUseWebsocket) {
            throw new Error("This scene is not allowed to use WebSocket")
        }

        let realProtocols = []
        if (typeof protocols === 'string') {
            realProtocols = [protocols]
        } else if (Array.isArray(protocols)) {
            realProtocols = protocols
        }

        super(url.toString(), realProtocols)
    }
}


module.exports.WebSocket = RestrictedWebSocket
