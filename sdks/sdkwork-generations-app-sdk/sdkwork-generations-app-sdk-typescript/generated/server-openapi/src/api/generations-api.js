import { appApiPath } from './paths';
export class GenerationsTimelineApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async list(generationId, params, requestOptions) {
        const query = buildQueryString([
            { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}/timeline`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
}
export class GenerationsResultsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async list(generationId, params, requestOptions) {
        const query = buildQueryString([
            { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}/results`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    async saveToAssets(generationId, resultId, body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}/results/${serializePathParameter(resultId, { name: 'resultId', style: 'simple', explode: false })}/save_to_assets`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class GenerationsVoiceApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async speech(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/voice/speech`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async transcription(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/voice/transcription`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async translation(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/voice/translation`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class GenerationsSoundEffectsApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async create(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/sound_effects`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class GenerationsMusicApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async textToMusic(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/music/text_to_music`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async lyricsToMusic(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/music/lyrics_to_music`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class GenerationsVideosApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async textToVideo(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/videos/text_to_video`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async imageToVideo(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/videos/image_to_video`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async videoExtend(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/videos/video_extend`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class GenerationsImagesApi {
    client;
    constructor(client) {
        this.client = client;
    }
    async textToImage(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/images/text_to_image`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async imageEdit(body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/images/image_edit`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export class GenerationsApi {
    client;
    images;
    videos;
    music;
    soundEffects;
    voice;
    results;
    timeline;
    constructor(client) {
        this.client = client;
        this.images = new GenerationsImagesApi(client);
        this.videos = new GenerationsVideosApi(client);
        this.music = new GenerationsMusicApi(client);
        this.soundEffects = new GenerationsSoundEffectsApi(client);
        this.voice = new GenerationsVoiceApi(client);
        this.results = new GenerationsResultsApi(client);
        this.timeline = new GenerationsTimelineApi(client);
    }
    async list(params, requestOptions) {
        const query = buildQueryString([
            { name: 'cursor', value: params?.cursor, style: 'form', explode: true, allowReserved: false },
            { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
            { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
            { name: 'modality', value: params?.modality, style: 'form', explode: true, allowReserved: false },
            { name: 'operation_type', value: params?.operationType, style: 'form', explode: true, allowReserved: false },
            { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
        ]);
        return this.client.request(appendQueryString(appApiPath(`/generations`), query), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'page' });
    }
    async retrieve(generationId, requestOptions) {
        return this.client.request(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'GET', sdkworkUnwrapKind: 'item' });
    }
    async cancel(generationId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}/cancel`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async retry(generationId, params, body, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}/retry`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', ...(body !== undefined ? { body, contentType: 'application/json' } : {}), ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
    async favorite(generationId, body, params, requestOptions) {
        const requestHeaders = buildRequestHeaders({
            'Idempotency-Key': { value: params.idempotencyKey, style: 'simple', explode: false },
        }, {});
        return this.client.request(appApiPath(`/generations/${serializePathParameter(generationId, { name: 'generationId', style: 'simple', explode: false })}/favorite`), { ...(requestOptions?.signal !== undefined ? { signal: requestOptions.signal } : {}), ...(requestOptions?.timeout !== undefined ? { timeout: requestOptions.timeout } : {}), method: 'POST', body, contentType: 'application/json', ...(requestHeaders !== undefined ? { headers: requestHeaders } : {}), sdkworkUnwrapKind: 'item' });
    }
}
export function createGenerationsApi(client) {
    return new GenerationsApi(client);
}
function appendQueryString(path, rawQueryString) {
    const query = rawQueryString.replace(/^\?+/, '');
    if (!query) {
        return path;
    }
    return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
}
function serializePathParameter(value, spec) {
    if (value === undefined || value === null) {
        return '';
    }
    const style = spec.style || 'simple';
    if (Array.isArray(value)) {
        return serializePathArray(spec.name, value, style, spec.explode);
    }
    if (typeof value === 'object') {
        return serializePathObject(spec.name, value, style, spec.explode);
    }
    return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}
function serializePathArray(name, values, style, explode) {
    const serialized = values
        .filter((item) => item !== undefined && item !== null)
        .map((item) => encodePathValue(serializePathPrimitive(item)));
    if (serialized.length === 0) {
        return pathPrefix(name, style, false);
    }
    if (style === 'matrix') {
        return explode
            ? serialized.map((item) => `;${name}=${item}`).join('')
            : `;${name}=${serialized.join(',')}`;
    }
    return pathPrefix(name, style, false) + serialized.join(explode ? '.' : ',');
}
function serializePathObject(name, value, style, explode) {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
    if (entries.length === 0) {
        return pathPrefix(name, style, true);
    }
    if (style === 'matrix') {
        return explode
            ? entries.map(([key, entryValue]) => `;${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join('')
            : `;${name}=${entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',')}`;
    }
    const serialized = explode
        ? entries.map(([key, entryValue]) => `${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join(style === 'label' ? '.' : ',')
        : entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',');
    return pathPrefix(name, style, true) + serialized;
}
function pathPrefix(name, style, _objectValue) {
    if (style === 'label')
        return '.';
    if (style === 'matrix')
        return `;${name}`;
    return '';
}
function encodePathValue(value) {
    return encodeURIComponent(value);
}
function serializePathPrimitive(value) {
    if (value instanceof Date) {
        return value.toISOString();
    }
    if (typeof value === 'object') {
        return JSON.stringify(value);
    }
    return String(value);
}
function buildQueryString(parameters) {
    const pairs = [];
    for (const parameter of parameters) {
        appendSerializedParameter(pairs, parameter);
    }
    return pairs.join('&');
}
function appendSerializedParameter(pairs, parameter) {
    if (parameter.value === undefined || parameter.value === null) {
        return;
    }
    if (parameter.contentType) {
        pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(JSON.stringify(parameter.value), parameter.allowReserved)}`);
        return;
    }
    const style = parameter.style || 'form';
    if (style === 'deepObject') {
        appendDeepObjectParameter(pairs, parameter.name, parameter.value, parameter.allowReserved);
        return;
    }
    if (Array.isArray(parameter.value)) {
        appendArrayParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
        return;
    }
    if (typeof parameter.value === 'object') {
        appendObjectParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
        return;
    }
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}
function appendArrayParameter(pairs, name, value, style, explode, allowReserved) {
    const values = value
        .filter((item) => item !== undefined && item !== null)
        .map((item) => serializePrimitive(item));
    if (values.length === 0) {
        return;
    }
    if (style === 'form' && explode) {
        for (const item of values) {
            pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(item, allowReserved)}`);
        }
        return;
    }
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(values.join(','), allowReserved)}`);
}
function appendObjectParameter(pairs, name, value, style, explode, allowReserved) {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
    if (entries.length === 0) {
        return;
    }
    if (style === 'form' && explode) {
        for (const [key, entryValue] of entries) {
            pairs.push(`${encodeQueryComponent(key)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
        }
        return;
    }
    const serialized = entries.flatMap(([key, entryValue]) => [key, serializePrimitive(entryValue)]).join(',');
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serialized, allowReserved)}`);
}
function appendDeepObjectParameter(pairs, name, value, allowReserved) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
        pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
        return;
    }
    for (const [key, entryValue] of Object.entries(value)) {
        if (entryValue === undefined || entryValue === null) {
            continue;
        }
        pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
}
function serializePrimitive(value) {
    if (value instanceof Date) {
        return value.toISOString();
    }
    if (typeof value === 'object') {
        return JSON.stringify(value);
    }
    return String(value);
}
function encodeQueryComponent(value) {
    return encodeURIComponent(value);
}
function encodeQueryValue(value, allowReserved) {
    const encoded = encodeURIComponent(value);
    if (!allowReserved) {
        return encoded;
    }
    return encoded.replace(/%3A/gi, ':')
        .replace(/%2F/gi, '/')
        .replace(/%3F/gi, '?')
        .replace(/%23/gi, '#')
        .replace(/%5B/gi, '[')
        .replace(/%5D/gi, ']')
        .replace(/%40/gi, '@')
        .replace(/%21/gi, '!')
        .replace(/%24/gi, '$')
        .replace(/%26/gi, '&')
        .replace(/%27/gi, "'")
        .replace(/%28/gi, '(')
        .replace(/%29/gi, ')')
        .replace(/%2A/gi, '*')
        .replace(/%2B/gi, '+')
        .replace(/%2C/gi, ',')
        .replace(/%3B/gi, ';')
        .replace(/%3D/gi, '=');
}
function buildRequestHeaders(headers, cookies = {}) {
    const requestHeaders = {};
    for (const [name, parameter] of Object.entries(headers)) {
        const serialized = serializeParameterValue(parameter);
        if (serialized !== undefined) {
            requestHeaders[name] = serialized;
        }
    }
    const cookieHeader = buildCookieHeader(cookies);
    if (cookieHeader) {
        requestHeaders.Cookie = requestHeaders.Cookie
            ? `${requestHeaders.Cookie}; ${cookieHeader}`
            : cookieHeader;
    }
    return Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined;
}
function buildCookieHeader(cookies) {
    const pairs = [];
    for (const [name, parameter] of Object.entries(cookies)) {
        const serialized = serializeParameterValue(parameter);
        if (serialized !== undefined) {
            pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(serialized)}`);
        }
    }
    return pairs.length > 0 ? pairs.join('; ') : undefined;
}
function serializeParameterValue(parameter) {
    const value = parameter?.value;
    if (value === undefined || value === null) {
        return undefined;
    }
    if (parameter?.contentType) {
        return JSON.stringify(value);
    }
    if (value instanceof Date) {
        return value.toISOString();
    }
    if (Array.isArray(value)) {
        return value.map((item) => serializeHeaderPrimitive(item)).join(',');
    }
    if (typeof value === 'object' && value !== null) {
        return serializeHeaderObject(value, parameter?.explode === true);
    }
    return serializeHeaderPrimitive(value);
}
function serializeHeaderObject(value, explode) {
    const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
    if (explode) {
        return entries.map(([key, entryValue]) => `${key}=${serializeHeaderPrimitive(entryValue)}`).join(',');
    }
    return entries.flatMap(([key, entryValue]) => [key, serializeHeaderPrimitive(entryValue)]).join(',');
}
function serializeHeaderPrimitive(value) {
    if (value instanceof Date) {
        return value.toISOString();
    }
    return String(value);
}
//# sourceMappingURL=generations-api.js.map