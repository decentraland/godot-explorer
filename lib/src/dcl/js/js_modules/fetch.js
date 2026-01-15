
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

    forEach(callback, thisArg) {
        for (const name in this.headers) {
            if (this.headers.hasOwnProperty(name)) {
                const values = this.headers[name];
                // Fix: properly iterate over header values
                values.forEach(value => {
                    callback.call(thisArg, value, name, this);
                });
            }
        }
    }

    get(name) {
        name = name.toLowerCase();
        const values = this.headers[name];
        return values ? values.join(', ') : null;
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

// Response class implementation following the fetch specification
class Response {
    constructor(body, init = {}) {
        // Store internal state
        this._bodyUsed = false;
        this._body = body;
        this._reqId = init._internal_req_id;
        this._networkInspectorId = init.network_inspector_id || 0;
        
        // Public properties
        this.headers = init.headers instanceof Headers ? init.headers : new Headers(init.headers || {});
        this.ok = init.ok !== undefined ? init.ok : (init.status >= 200 && init.status < 300);
        this.redirected = init.redirected || false;
        this.status = init.status !== undefined ? init.status : 200;
        this.statusText = init.statusText || '';
        this.type = init.type || 'default';
        this.url = init.url || '';
    }
    
    get bodyUsed() {
        return this._bodyUsed;
    }
    
    _checkBodyUsed() {
        if (this._bodyUsed) {
            throw new TypeError('Body has already been consumed.');
        }
        this._bodyUsed = true;
    }
    
    _checkError() {
        if (this.type === 'error') {
            throw new TypeError('Failed to fetch: ' + this.statusText);
        }
    }
    
    async arrayBuffer() {
        this._checkBodyUsed();
        
        // Don't check for error type here - let the response be consumed
        // This matches standard fetch behavior where you can read error responses
        
        if (this._reqId !== undefined) {
            const data = await Deno.core.ops.op_fetch_consume_bytes(
                this._reqId,
                this._networkInspectorId
            );
            return data;
        }
        
        // For error responses without reqId, throw an error
        throw new TypeError('Failed to fetch');
    }
    
    async json() {
        this._checkBodyUsed();
        
        // Don't check for error type here - let the response be consumed
        // This matches standard fetch behavior where you can read error responses
        
        if (this._reqId !== undefined) {
            const text = await Deno.core.ops.op_fetch_consume_text(
                this._reqId,
                this._networkInspectorId
            );
            try {
                return JSON.parse(text);
            } catch (err) {
                console.error('Failed to parse response as JSON.', { url: this.url }, ' data ', text);
                throw new SyntaxError('Failed to parse JSON: ' + err.message);
            }
        }
        
        // For error responses without reqId, throw an error instead of returning null
        // This matches the behavior when network request fails completely
        throw new TypeError('Failed to fetch');
    }
    
    async text() {
        this._checkBodyUsed();
        
        // Don't check for error type here - let the response be consumed
        // This matches standard fetch behavior where you can read error responses
        
        if (this._reqId !== undefined) {
            const data = await Deno.core.ops.op_fetch_consume_text(
                this._reqId,
                this._networkInspectorId
            );
            return data;
        }
        
        // For error responses without reqId, return the status text as the body
        // This provides some information about what went wrong
        return this.statusText || '';
    }
    
    async bytes() {
        this._checkBodyUsed();
        
        // Don't check for error type here - let the response be consumed
        // This matches standard fetch behavior where you can read error responses
        
        if (this._reqId !== undefined) {
            const data = await Deno.core.ops.op_fetch_consume_bytes(
                this._reqId,
                this._networkInspectorId
            );
            return data;
        }
        
        // For error responses without reqId, throw an error
        throw new TypeError('Failed to fetch');
    }
    
    // Clone method for potential future use
    clone() {
        if (this._bodyUsed) {
            throw new TypeError('Failed to clone response: Body has already been consumed.');
        }
        
        return new Response(this._body, {
            headers: new Headers(this.headers),
            ok: this.ok,
            redirected: this.redirected,
            status: this.status,
            statusText: this.statusText,
            type: this.type,
            url: this.url,
            _internal_req_id: this._reqId,
            network_inspector_id: this._networkInspectorId
        });
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

    // Call the Rust fetch operation
    const rustResponse = await Deno.core.ops.op_fetch_custom(
        reqMethod, url, reqHeaders, hasBody, body ?? '', reqRedirect, reqTimeout
    )

    // Create a proper Response object using our Response class
    const response = new Response(null, {
        headers: rustResponse.headers || {},
        ok: rustResponse.ok,
        redirected: rustResponse.redirected,
        status: rustResponse.status,
        statusText: rustResponse.statusText || '',
        type: rustResponse.type || 'basic',
        url: rustResponse.url || url,
        _internal_req_id: rustResponse._internal_req_id,
        network_inspector_id: rustResponse.network_inspector_id || 0
    });

    return response;
}
module.exports.fetch = restrictedFetch
module.exports.Headers = Headers
module.exports.Response = Response