
// types defined in the index.d.ts of the SDK

// type RequestRedirect = 'follow' | 'error' | 'manual'
// type ResponseType = 'basic' | 'cors' | 'default' | 'error' | 'opaque' | 'opaqueredirect'

// interface RequestInit {
//   // whatwg/fetch standard options
//   body?: string
//   headers?: { [index: string]: string }
//   method?: string
//   redirect?: RequestRedirect

//   // custom DCL property
//   timeout?: number
// }

// interface ReadOnlyHeaders {
//   get(name: string): string | null
//   has(name: string): boolean
//   forEach(callbackfn: (value: string, key: string, parent: ReadOnlyHeaders) => void, thisArg?: any): void
// }

// interface Response {
//   readonly headers: ReadOnlyHeaders
//   readonly ok: boolean
//   readonly redirected: boolean
//   readonly status: number
//   readonly statusText: string
//   readonly type: ResponseType
//   readonly url: string

//   json(): Promise<any>
//   text(): Promise<string>
// }

// declare function fetch(url: string, init?: RequestInit): Promise<Response>

module.exports.fetch = async function (url, init) {
    const { body, headers, method, redirect, timeout } = init ?? {}
    const hasBody = typeof body === 'string'
    const reqMethod = method ?? 'GET'
    const reqTimeout = timeout ?? 30
    const reqHeaders = headers ?? {}
    const reqRedirect = redirect ?? 'follow'
    const response = await Deno.core.opAsync(
        "op_fetch_custom",
        reqMethod, url, reqHeaders, hasBody, body ?? '', reqRedirect, reqTimeout
    )
    const reqId = response._internal_req_id

    Object.assign(response, {
        async arrayBuffer() {
            return new ArrayBuffer(0)
        },
        async json() {
            const data = await Deno.core.opAsync(
                "op_fetch_consume_text",
                reqId
            )
            return JSON.parse(data)
        },
        async text() {
            const data = await Deno.core.opAsync(
                "op_fetch_consume_text",
                reqId
            )
            return data
        },
        async _internal_bytes() {
            const data = await Deno.core.opAsync(
                "op_fetch_consume_bytes",
                reqId
            )
            return data
        }
    })

    return response
}