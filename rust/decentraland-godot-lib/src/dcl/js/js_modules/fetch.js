
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
class Headers {
    constructor(init = {}) {
        this.headers = {};

        if (init instanceof Headers) {
            init.forEach((value, name) => {
                this.append(name, value);
            });
        } else if (Array.isArray(init)) {
            init.forEach(([name, value]) => {
                this.append(name, value);
            });
        } else if (init && typeof init === 'object') {
            Object.keys(init).forEach(name => {
                this.append(name, init[name]);
            });
        }
    }

    append(name, value) {
        name = name.toLowerCase();
        if (!this.headers[name]) {
            this.headers[name] = [];
        }
        this.headers[name].push(value);
    }

    delete(name) {
        name = name.toLowerCase();
        delete this.headers[name];
    }

    entries() {
        const result = [];
        this.forEach((value, name) => {
            result.push([name, value]);
        });
        return result;
    }

    forEach(callback) {
        for (const name in this.headers) {
            if (this.headers.hasOwnProperty(name)) {
                const values = this.headers[name];
                name.split(',').forEach(callback.bind(null, values, name));
            }
        }
    }

    get(name) {
        name = name.toLowerCase();
        return this.headers[name] ? this.headers[name][0] : null;
    }

    getSetCookie() {
        const setCookieHeaders = this.getAll('Set-Cookie');
        return setCookieHeaders.map(header => header.split(';')[0]);
    }

    has(name) {
        name = name.toLowerCase();
        return !!this.headers[name];
    }

    keys() {
        return Object.keys(this.headers);
    }

    set(name, value) {
        name = name.toLowerCase();
        this.headers[name] = [value];
    }

    values() {
        const result = [];
        this.forEach(value => {
            result.push(value);
        });
        return result;
    }

    getAll(name) {
        name = name.toLowerCase();
        return this.headers[name] || [];
    }
}

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

    response.headers = new Headers(response.headers)
    // TODO: the headers object should be read-only

    let alreadyConsumed = false
    function notifyConsume() {
        if (alreadyConsumed) {
            throw new Error("Response body has already been consumed.")
        }
        alreadyConsumed = true
    }

    function throwErrorFailed() {
        if (response.type === "error") {
            throw new Error("Failed to fetch " + response.statusText)
        }
    }


    Object.assign(response, {
        async arrayBuffer() {
            notifyConsume()
            throwErrorFailed()
            const data = await Deno.core.opAsync(
                "op_fetch_consume_bytes",
                reqId
            )
            alreadyConsumed = true
            return data
        },
        async json() {
            notifyConsume()
            throwErrorFailed()
            const data = await Deno.core.opAsync(
                "op_fetch_consume_text",
                reqId
            )
            return JSON.parse(data)
        },
        async text() {
            notifyConsume()
            throwErrorFailed()
            const data = await Deno.core.opAsync(
                "op_fetch_consume_text",
                reqId
            )
            return data
        },
        async bytes() {
            throwErrorFailed()
            notifyConsume()
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