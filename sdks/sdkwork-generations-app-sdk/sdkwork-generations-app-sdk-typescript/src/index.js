import { createClient as createGeneratedGenerationsAppClient, SdkworkAppClient, } from '../generated/server-openapi/src/index';
export { SdkworkAppClient, createGeneratedGenerationsAppClient };
export * from '../generated/server-openapi/src/types';
export * from '../generated/server-openapi/src/api';
export * from '../generated/server-openapi/src/http';
export * from '../generated/server-openapi/src/auth';
export function createGenerationsAppClient(config) {
    return createGeneratedGenerationsAppClient(config);
}
export function createClient(config) {
    return createGenerationsAppClient(config);
}
//# sourceMappingURL=index.js.map