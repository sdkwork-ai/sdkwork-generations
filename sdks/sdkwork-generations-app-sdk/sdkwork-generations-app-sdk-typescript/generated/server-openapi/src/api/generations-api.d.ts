import type { ApiRequestOptions, HttpClient } from '../http/client';
import type { CreateGenerationCommandRequest, FavoriteGenerationRequest, GenerationActionRequest, GenerationCommandResponse, GenerationModality, GenerationRecord, GenerationResult, GenerationStatus, GenerationTimelineEvent, SaveGenerationResultToAssetsRequest } from '../types';
export interface GenerationsTimelineListParams {
    cursor?: string;
    pageSize?: number;
}
export declare class GenerationsTimelineApi {
    private client;
    constructor(client: HttpClient);
    list(generationId: string, params?: GenerationsTimelineListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: GenerationTimelineEvent[];
        pageInfo: {
            mode: 'cursor';
            nextCursor?: string | null;
            hasMore: boolean;
        };
    }>;
}
export interface GenerationsResultsListParams {
    cursor?: string;
    pageSize?: number;
}
export interface GenerationsResultsSaveToAssetsParams {
    idempotencyKey: string;
}
export declare class GenerationsResultsApi {
    private client;
    constructor(client: HttpClient);
    list(generationId: string, params?: GenerationsResultsListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: GenerationResult[];
        pageInfo: {
            mode: 'cursor';
            nextCursor?: string | null;
            hasMore: boolean;
        };
    }>;
    saveToAssets(generationId: string, resultId: string, body: SaveGenerationResultToAssetsRequest, params: GenerationsResultsSaveToAssetsParams, requestOptions?: ApiRequestOptions): Promise<GenerationResult>;
}
export interface GenerationsVoiceSpeechParams {
    idempotencyKey: string;
}
export interface GenerationsVoiceTranscriptionParams {
    idempotencyKey: string;
}
export interface GenerationsVoiceTranslationParams {
    idempotencyKey: string;
}
export declare class GenerationsVoiceApi {
    private client;
    constructor(client: HttpClient);
    speech(body: CreateGenerationCommandRequest, params: GenerationsVoiceSpeechParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    transcription(body: CreateGenerationCommandRequest, params: GenerationsVoiceTranscriptionParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    translation(body: CreateGenerationCommandRequest, params: GenerationsVoiceTranslationParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
}
export interface GenerationsSoundEffectsCreateParams {
    idempotencyKey: string;
}
export declare class GenerationsSoundEffectsApi {
    private client;
    constructor(client: HttpClient);
    create(body: CreateGenerationCommandRequest, params: GenerationsSoundEffectsCreateParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
}
export interface GenerationsMusicTextToMusicParams {
    idempotencyKey: string;
}
export interface GenerationsMusicLyricsToMusicParams {
    idempotencyKey: string;
}
export declare class GenerationsMusicApi {
    private client;
    constructor(client: HttpClient);
    textToMusic(body: CreateGenerationCommandRequest, params: GenerationsMusicTextToMusicParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    lyricsToMusic(body: CreateGenerationCommandRequest, params: GenerationsMusicLyricsToMusicParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
}
export interface GenerationsVideosTextToVideoParams {
    idempotencyKey: string;
}
export interface GenerationsVideosImageToVideoParams {
    idempotencyKey: string;
}
export interface GenerationsVideosVideoExtendParams {
    idempotencyKey: string;
}
export declare class GenerationsVideosApi {
    private client;
    constructor(client: HttpClient);
    textToVideo(body: CreateGenerationCommandRequest, params: GenerationsVideosTextToVideoParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    imageToVideo(body: CreateGenerationCommandRequest, params: GenerationsVideosImageToVideoParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    videoExtend(body: CreateGenerationCommandRequest, params: GenerationsVideosVideoExtendParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
}
export interface GenerationsImagesTextToImageParams {
    idempotencyKey: string;
}
export interface GenerationsImagesImageEditParams {
    idempotencyKey: string;
}
export declare class GenerationsImagesApi {
    private client;
    constructor(client: HttpClient);
    textToImage(body: CreateGenerationCommandRequest, params: GenerationsImagesTextToImageParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    imageEdit(body: CreateGenerationCommandRequest, params: GenerationsImagesImageEditParams, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
}
export interface GenerationsListParams {
    cursor?: string;
    pageSize?: number;
    status?: GenerationStatus;
    modality?: GenerationModality;
    operationType?: string;
    q?: string;
}
export interface GenerationsCancelParams {
    idempotencyKey: string;
}
export interface GenerationsRetryParams {
    idempotencyKey: string;
}
export interface GenerationsFavoriteParams {
    idempotencyKey: string;
}
export declare class GenerationsApi {
    private client;
    readonly images: GenerationsImagesApi;
    readonly videos: GenerationsVideosApi;
    readonly music: GenerationsMusicApi;
    readonly soundEffects: GenerationsSoundEffectsApi;
    readonly voice: GenerationsVoiceApi;
    readonly results: GenerationsResultsApi;
    readonly timeline: GenerationsTimelineApi;
    constructor(client: HttpClient);
    list(params?: GenerationsListParams, requestOptions?: ApiRequestOptions): Promise<{
        items: GenerationRecord[];
        pageInfo: {
            mode: 'cursor';
            nextCursor?: string | null;
            hasMore: boolean;
        };
    }>;
    retrieve(generationId: string, requestOptions?: ApiRequestOptions): Promise<GenerationRecord>;
    cancel(generationId: string, params: GenerationsCancelParams, body?: GenerationActionRequest, requestOptions?: ApiRequestOptions): Promise<GenerationRecord>;
    retry(generationId: string, params: GenerationsRetryParams, body?: GenerationActionRequest, requestOptions?: ApiRequestOptions): Promise<GenerationCommandResponse>;
    favorite(generationId: string, body: FavoriteGenerationRequest, params: GenerationsFavoriteParams, requestOptions?: ApiRequestOptions): Promise<GenerationRecord>;
}
export declare function createGenerationsApi(client: HttpClient): GenerationsApi;
//# sourceMappingURL=generations-api.d.ts.map