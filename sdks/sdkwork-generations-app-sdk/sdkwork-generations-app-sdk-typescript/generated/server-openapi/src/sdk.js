import { createHttpClient } from './http/client';
import { createGenerationsApi } from './api/generations-api';
export class SdkworkAppClient {
    httpClient;
    generations;
    constructor(config) {
        this.httpClient = createHttpClient(config);
        this.generations = createGenerationsApi(this.httpClient);
    }
    setAuthToken(token) {
        this.httpClient.setAuthToken(token);
        return this;
    }
    setAccessToken(token) {
        this.httpClient.setAccessToken(token);
        return this;
    }
    setTokenManager(manager) {
        this.httpClient.setTokenManager(manager);
        return this;
    }
    get http() {
        return this.httpClient;
    }
}
export function createClient(config) {
    return new SdkworkAppClient(config);
}
export default SdkworkAppClient;
//# sourceMappingURL=sdk.js.map