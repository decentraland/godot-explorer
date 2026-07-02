
// /// --- WebSocket ---

// interface Event {
//     readonly type: string
//   }

//   interface MessageEvent extends Event {
//     /**
//      * Returns the data of the message.
//      */
//     readonly data: any
//   }

//   interface CloseEvent extends Event {
//     readonly code: number
//     readonly reason: string
//     readonly wasClean: boolean
//   }

//   interface WebSocket {
//     readonly bufferedAmount: number
//     readonly extensions: string
//     onclose: ((this: WebSocket, ev: CloseEvent) => any) | null
//     onerror: ((this: WebSocket, ev: Event) => any) | null
//     onmessage: ((this: WebSocket, ev: MessageEvent) => any) | null
//     onopen: ((this: WebSocket, ev: Event) => any) | null
//     readonly protocol: string
//     readonly readyState: number
//     readonly url: string
//     close(code?: number, reason?: string): void
//     send(data: string): void
//     readonly CLOSED: number
//     readonly CLOSING: number
//     readonly CONNECTING: number
//     readonly OPEN: number
//   }

//   declare var WebSocket: {
//     prototype: WebSocket
//     new (url: string, protocols?: string | string[]): WebSocket
//     readonly CLOSED: number
//     readonly CLOSING: number
//     readonly CONNECTING: number
//     readonly OPEN: number
//   }

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

        this._internal_ws_id = Deno.core.ops.op_ws_create(
            url, protocols ?? []
        )

        this.onclose = null
        this.onerror = null
        this.onmessage = null
        this.onopen = null

        this._poll().then(console.warn).catch(console.error)
    }

    // There is no send buffer here
    get bufferedAmount() {
        return 0
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

    // TODO: implement
    get protocol() {
        return ""
    }

    // TODO: implement
    get extensions() {
        return ""
    }

    send(data) {
        if (typeof data === 'string') {
            Deno.core.ops.op_ws_send(this._internal_ws_id, { "type": "Text", data }).then().catch(console.error)
        } else if (typeof data === 'object' && data instanceof Uint8Array) {
            Deno.core.ops.op_ws_send(this._internal_ws_id, { "type": "Binary", data: Array.from(data) }).then().catch(console.error)
        } else if (typeof data === 'object' && data instanceof ArrayBuffer) {
            Deno.core.ops.op_ws_send(this._internal_ws_id, { "type": "Binary", data: Array.from(new Uint8Array(data)) }).then().catch(console.error)
        } else if (Array.isArray(data)) {
            Deno.core.ops.op_ws_send(this._internal_ws_id, { "type": "Binary", data }).then().catch(console.error)
        } else {
            console.error(`Unsupported data type: ${typeof data}`, data)
            throw new Error("Unsupported data type")
        }
    }

    // TODO: forward `code` and `reason` to the peer in the close frame.
    close(code, reason) {
        if (this._readyState === WebSocket.CLOSING || this._readyState === WebSocket.CLOSED) {
            return
        }
        this._readyState = WebSocket.CLOSING
        Deno.core.ops.op_ws_send(this._internal_ws_id, { "type": "Close" }).catch(console.error)
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
            if (typeof self.onclose === 'function') {
                self.onclose({
                    type: "close",
                    code: typeof code === 'number' ? code : 1006,
                    reason: reason ?? "",
                    wasClean: code === 1000,
                })
            }
        }

        async function poll_from_native() {
            const data = await Deno.core.ops.op_ws_poll(self._internal_ws_id)

            switch (data.type) {
                case "BinaryData":
                    if (typeof self.onmessage === 'function') {
                        self.onmessage({ type: "binary", data: new Uint8Array(data.data) })
                    }
                    break
                case "TextData":
                    if (typeof self.onmessage === 'function') {
                        self.onmessage({ type: "text", data: data.data })
                    }
                    break
                case "Connected":
                    // Transition to OPEN so `readyState` reports OPEN. Libraries
                    // that gate sends on `readyState === WebSocket.OPEN` (e.g.
                    // Colyseus `isOpen`) would otherwise buffer/drop every message.
                    self._readyState = WebSocket.OPEN
                    if (typeof self.onopen === 'function') {
                        self.onopen({ type: "open" })
                    }
                    break
                case "Closed":
                    fireClose(data.code, data.reason)
                    return false;
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
            if (typeof self.onerror === 'function') {
                self.onerror(err)
            }
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
        const previewMode = true // TODO: this should be exposed from Deno.env
        const canUseWebsocket = true // TODO: this should be exposed from Deno.env

        if (url.toString().toLowerCase().substr(0, 4) !== 'wss:') {
            if (previewMode) {
                console.log(
                    "⚠️ Warning: can't connect to unsafe WebSocket (ws) server in deployed scenes, consider upgrading to wss."
                )
            } else {
                throw new Error("Can't connect to unsafe WebSocket server")
            }
        }

        if (!canUseWebsocket) {
            throw new Error("This scene doesn't have allowed to use WebSocket")
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