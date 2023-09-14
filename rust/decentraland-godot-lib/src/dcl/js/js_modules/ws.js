
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
        this._internal_ws_id = Deno.core.opSync(
            "op_ws_create",
            url, protocols
        )

        this.bufferedAmount = 0 // TODO
        this.extensions = ""
        this.protocol = ""

        this.onclose = null
        this.onerror = null
        this.onmessage = null
        this.onopen = null

        _poll().then().catch()
    }

    send(data) {
        Deno.core.opSync(
            "op_ws_send",
            this._internal_req_id, data
        )
    }

    close(code, reason) {
        Deno.core.opSync(
            "op_ws_close",
            this._internal_req_id, code, reason
        )
        this.readyState = WebSocket.CLOSED
    }

    async _poll() {
        try {
            while (true) {
                const data = await Deno.core.opAsync(
                    "op_ws_poll", this._internal_req_id
                )
            }
        } catch (err) {
            if (typeof this.onerror === 'function') {
                this.onerror(err)
            }
        }

        Deno.core.opSync(
            "op_ws_cleanup", this._internal_req_id
        )
    }
}

module.exports.WebSocket = WebSocket