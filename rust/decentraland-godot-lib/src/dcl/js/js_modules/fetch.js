
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


async function restrictedFetch(url, init) {
    const canUseFetch = true // TODO: this should be exposed from Deno.env
    const previewMode = true // TODO: this should be exposed from Deno.env

    if (url.toLowerCase().substr(0, 8) !== "https://") {
        if (previewMode) {
            console.log(
                "⚠️ Warning: Can't make an unsafe http request in deployed scenes, please consider upgrading to https. url=" +
                url
            )
        } else {
            return Promise.reject(new Error("Can't make an unsafe http request, please upgrade to https. url=" + url))
        }
    }

    if (!canUseFetch) {
        return Promise.reject(new Error("This scene is not allowed to use fetch."))
    }

    return await fetch(url, init)
}


async function fetch(url, init) {
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
    console.log({ reqTimeout })
    // TODO: the headers object should be read-only

    Object.assign(response, {
        async arrayBuffer() {
            const data = await Deno.core.opAsync(
                "op_fetch_consume_bytes",
                reqId
            )
            return data
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
        async bytes() {
            const data = await Deno.core.opAsync(
                "op_fetch_consume_bytes",
                reqId
            )
            return data
        }
    })

    return response
}
module.exports.fetch = restrictedFetch