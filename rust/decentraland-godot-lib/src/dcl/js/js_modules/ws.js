
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
    static CLOSED = 1
    static CLOSING = 2
    static CONNECTING = 3
    static OPEN = 4

    constructor(url, protocols) {
        this.url = url
        this.protocols = protocols
        this.readyState = WebSocket.CONNECTING
        this._internal_ws_id = Deno.core.ops.op_ws_create(
            url, protocols ?? []
        )

        this.bufferedAmount = 0 // TODO: implement
        this.extensions = "" // TODO: implement
        this.protocol = "" // TODO: implement

        this.onclose = null
        this.onerror = null
        this.onmessage = null
        this.onopen = null

        this._poll().then(console.warn).catch(console.error)
    }

    send(data) {
        if (typeof data === 'string') {
            Deno.core.opAsync("op_ws_send", this._internal_ws_id, { "type": "Text", data }).then().catch(console.error)
        } else if (typeof data === 'object' && data instanceof Uint8Array) {
            Deno.core.opAsync("op_ws_send", this._internal_ws_id, { "type": "Binary", data: Array.from(data) }).then().catch(console.error)
        }
    }

    close(code, reason) {
        if (this.readyState != WebSocket.CLOSED) {
            Deno.core.ops.op_ws_send(this._internal_ws_id, { "type": "Close" })
            this.readyState = WebSocket.CLOSED
        }
    }

    async _poll() {
        const self = this
        async function poll_from_native() {
            const data = await Deno.core.opAsync(
                "op_ws_poll", self._internal_ws_id
            )

            console.log("poll data ", data)

            switch (data.type) {
                case "BinaryData":
                    if (typeof self.onmessage === 'function') {
                        self.onmessage({ type: "binary", data: data.data })
                    }
                    break
                case "TextData":
                    if (typeof self.onmessage === 'function') {
                        self.onmessage({ type: "text", data: data.data })
                    }
                    break
                case "Connected":
                    if (typeof self.onopen === 'function') {
                        self.onopen({ type: "open" })
                    }
                    break
                case "Closed":
                    if (typeof self.onclose === 'function') {
                        self.onclose({ type: "close" })
                    }
                    return false;
                default:
                    throw new Error("unreached")
            }
            return true
        }

        try {
            while (true) {
                if (!(await poll_from_native())) {
                    break
                }
            }
        } catch (err) {
            if (typeof this.onerror === 'function') {
                this.onerror(err)
            }
        }

        this.readyState = WebSocket.CLOSED
        Deno.core.ops.op_ws_cleanup(this._internal_ws_id)
    }
}

module.exports.WebSocket = WebSocket