
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
        if (typeof data !== 'string') {
            if (data instanceof Uint8Array) {
                Deno.core.ops.op_ws_send_bin(
                    this._internal_ws_id, data
                )
            }
        } else {
            Deno.core.ops.op_ws_send_text(
                this._internal_ws_id, data
            )
        }
    }

    close(code, reason) {
        Deno.core.ops.op_ws_close(
            this._internal_ws_id, code, reason
        )
        this.readyState = WebSocket.CLOSED
    }

    async _poll() {
        try {
            while (true) {
                const data = await Deno.core.opAsync(
                    "op_ws_poll", this._internal_ws_id
                )


                if (data.closed) {
                    if (typeof this.onclose === 'function') {
                        this.onclose({ type: "close" })
                    }
                    break
                }


                if (data.binary_data) {
                    if (typeof this.onmessage === 'function') {
                        this.onmessage({ type: "binary", data: data.binary_data })
                    }
                } else if (data.text_data) {
                    if (typeof this.onmessage === 'function') {
                        this.onmessage({ type: "text", data: data.text_data })
                    }
                } else if (data.connected) {
                    if (typeof this.onopen === 'function') {
                        this.onopen({ type: "open" })
                    }
                } else {
                    throw new Error("unreached")
                }
            }
        } catch (err) {
            if (typeof this.onerror === 'function') {
                this.onerror(err)
            }
        }

        Deno.core.ops.op_ws_cleanup(this._internal_ws_id)
    }
}

module.exports.WebSocket = WebSocket