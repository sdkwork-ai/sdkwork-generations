import { HttpClient } from './http/client';
import type { SdkworkAppConfig } from './types/common';
import type { AuthTokenManager } from '@sdkwork/sdk-common';
import { GenerationsApi } from './api/generations-api';
export declare class SdkworkAppClient {
    private httpClient;
    readonly generations: GenerationsApi;
    constructor(config: SdkworkAppConfig);
    setAuthToken(token: string): this;
    setAccessToken(token: string): this;
    setTokenManager(manager: AuthTokenManager): this;
    get http(): HttpClient;
}
export declare function createClient(config: SdkworkAppConfig): SdkworkAppClient;
export default SdkworkAppClient;
//# sourceMappingURL=sdk.d.ts.map