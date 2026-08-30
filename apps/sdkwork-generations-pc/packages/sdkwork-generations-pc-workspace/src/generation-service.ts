import {
  createEmptySdkworkGenerationWorkspace,
  type SdkworkGenerationRun,
  type SdkworkGenerationWorkspaceData,
} from "./generation.ts";

export type {
  SdkworkGenerationRun,
  SdkworkGenerationStatus,
  SdkworkGenerationWorkspaceData,
} from "./generation.ts";

export interface CreateSdkworkGenerationServiceOptions {
  getSessionTokens?: () => {
    authToken?: string;
  };
  includeSampleRuns?: boolean;
  listRuns?: () => Promise<readonly SdkworkGenerationRun[]>;
  pageSize?: number;
  runs?: readonly SdkworkGenerationRun[];
  sdkClients?: SdkworkGenerationSdkClients;
}

export interface SdkworkGenerationService {
  cancelGeneration(input: SdkworkGenerationActionInput): Promise<SdkworkGenerationRecord>;
  createGenerationCommand(input: SdkworkGenerationCommandInput): Promise<SdkworkGenerationCommandResult>;
  getEmptyWorkspace(): SdkworkGenerationWorkspaceData;
  getGeneration(generationId: string): Promise<SdkworkGenerationRecord>;
  getWorkspace(): Promise<SdkworkGenerationWorkspaceData>;
  listGenerationRecords(input?: SdkworkGenerationRecordsListInput): Promise<SdkworkGenerationRecordPage>;
  listGenerationResults(input: SdkworkGenerationResultsListInput): Promise<SdkworkGenerationResultPage>;
  listGenerationTimeline(input: SdkworkGenerationTimelineListInput): Promise<SdkworkGenerationTimelineEventPage>;
  retryGeneration(input: SdkworkGenerationActionInput): Promise<SdkworkGenerationCommandResult>;
  saveGenerationResultToAssets(input: SdkworkGenerationSaveResultToAssetsInput): Promise<SdkworkGenerationResult>;
  setFavorite(input: SdkworkGenerationFavoriteInput): Promise<SdkworkGenerationRecord>;
}

export type SdkworkGenerationCommandModality =
  | "audio"
  | "image"
  | "music"
  | "sfx"
  | "video"
  | "voice";

export type SdkworkGenerationOperationType =
  | "image_edit"
  | "image_to_video"
  | "lyrics_to_music"
  | "speech"
  | "sound_effect"
  | "text_to_image"
  | "text_to_music"
  | "text_to_video"
  | "transcription"
  | "translation"
  | "video_extend";

export interface SdkworkGenerationCommandInput {
  idempotencyKey?: string;
  inputAssetIds?: readonly string[];
  modality: SdkworkGenerationCommandModality;
  model?: string;
  operationType: SdkworkGenerationOperationType;
  organizationId?: string;
  parameters?: Record<string, unknown>;
  prompt: string;
  tenantId?: string;
}

export interface SdkworkGenerationCommandRequest {
  inputAssetIds?: string[];
  model?: string;
  organizationId?: string;
  parameters?: Record<string, unknown>;
  prompt: string;
}

export interface SdkworkGenerationCommandParams {
  idempotencyKey?: string;
}

export interface SdkworkGenerationCommandResponse {
  generation: SdkworkGenerationRecord;
}

export interface SdkworkGenerationCommandResult {
  generation: SdkworkGenerationRun;
  record: SdkworkGenerationRecord;
}

export interface SdkworkGenerationActionInput {
  generationId: string;
  idempotencyKey?: string;
  reason?: string;
}

export interface SdkworkGenerationFavoriteInput {
  favorite: boolean;
  generationId: string;
}

export interface SdkworkGenerationRecordsListInput {
  cursor?: string;
  modality?: string;
  operationType?: string;
  pageSize?: number;
  q?: string;
  status?: SdkworkGenerationRemoteStatus;
}

export interface SdkworkGenerationResultsListInput {
  cursor?: string;
  generationId: string;
  pageSize?: number;
}

export interface SdkworkGenerationTimelineListInput {
  cursor?: string;
  generationId: string;
  pageSize?: number;
}

export interface SdkworkGenerationSaveResultToAssetsInput {
  collectionId?: string;
  generationId: string;
  idempotencyKey?: string;
  resultId: string;
  tags?: readonly string[];
  tenantId?: string;
  title?: string;
}

export interface SdkworkGenerationActionRequest {
  reason?: string;
}

export interface SdkworkGenerationFavoriteRequest {
  favorite: boolean;
}

export interface SdkworkGenerationSaveResultToAssetsRequest {
  collectionId?: string;
  tags?: string[];
  title?: string;
}

export type SdkworkGenerationRemoteStatus =
  | "canceled"
  | "failed"
  | "queued"
  | "requires_action"
  | "running"
  | "succeeded";

export interface SdkworkGenerationRecord {
  createdAt: string;
  favorite?: boolean;
  id: string;
  modality: string;
  operationType: string;
  organizationId?: string;
  promptPreview?: string;
  resultCount?: number;
  sourceProvider?: string;
  sourceJobId?: string;
  status: SdkworkGenerationRemoteStatus;
  tenantId?: string;
  updatedAt: string;
  userId?: string;
}

export interface SdkworkGenerationRecordPage {
  items?: readonly SdkworkGenerationRecord[];
  nextCursor?: string;
}

export interface SdkworkGenerationsListParams {
  cursor?: string;
  pageSize?: number;
  modality?: string;
  operationType?: string;
  q?: string;
  status?: SdkworkGenerationRemoteStatus;
}

export interface SdkworkGenerationResult {
  assetId?: string;
  createdAt: string;
  driveNodeId?: string;
  driveSpaceId?: string;
  driveUri?: string;
  generationId: string;
  id: string;
  previewText?: string;
  resourceSnapshot?: unknown;
  resultType: string;
}

export interface SdkworkGenerationResultPage {
  items?: readonly SdkworkGenerationResult[];
  nextCursor?: string;
}

export interface SdkworkGenerationTimelineEvent {
  createdAt: string;
  eventType: string;
  generationId: string;
  id: string;
  message?: string;
  payload?: Record<string, unknown>;
}

export interface SdkworkGenerationTimelineEventPage {
  items?: readonly SdkworkGenerationTimelineEvent[];
  nextCursor?: string;
}

export interface SdkworkGenerationResourceListParams {
  cursor?: string;
  pageSize?: number;
}

export interface SdkworkGenerationIdempotencyParams {
  idempotencyKey?: string;
}

export interface SdkworkGenerationsImagesResourceClient {
  imageEdit(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
  textToImage(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
}

export interface SdkworkGenerationsVideosResourceClient {
  imageToVideo(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
  textToVideo(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
  videoExtend(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
}

export interface SdkworkGenerationsMusicResourceClient {
  lyricsToMusic(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
  textToMusic(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
}

export interface SdkworkGenerationsSoundEffectsResourceClient {
  create(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
}

export interface SdkworkGenerationsVoiceResourceClient {
  speech(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
  transcription(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
  translation(
    body: SdkworkGenerationCommandRequest,
    params?: SdkworkGenerationCommandParams,
  ): Promise<SdkworkGenerationCommandResponse>;
}

export interface SdkworkGenerationsResultsResourceClient {
  list(
    generationId: string,
    params?: SdkworkGenerationResourceListParams,
  ): Promise<SdkworkGenerationResultPage>;
  saveToAssets(
    generationId: string,
    resultId: string,
    body: SdkworkGenerationSaveResultToAssetsRequest,
    params?: SdkworkGenerationIdempotencyParams,
  ): Promise<SdkworkGenerationResult>;
}

export interface SdkworkGenerationsTimelineResourceClient {
  list(
    generationId: string,
    params?: SdkworkGenerationResourceListParams,
  ): Promise<SdkworkGenerationTimelineEventPage>;
}

export interface SdkworkGenerationsResourceClient {
  cancel(
    generationId: string,
    params?: SdkworkGenerationIdempotencyParams,
    body?: SdkworkGenerationActionRequest,
  ): Promise<SdkworkGenerationRecord>;
  favorite(
    generationId: string,
    body: SdkworkGenerationFavoriteRequest,
    params?: SdkworkGenerationIdempotencyParams,
  ): Promise<SdkworkGenerationRecord>;
  retrieve(generationId: string): Promise<SdkworkGenerationRecord>;
  images: SdkworkGenerationsImagesResourceClient;
  list(params?: SdkworkGenerationsListParams): Promise<SdkworkGenerationRecordPage>;
  music: SdkworkGenerationsMusicResourceClient;
  results: SdkworkGenerationsResultsResourceClient;
  retry(
    generationId: string,
    params?: SdkworkGenerationIdempotencyParams,
    body?: SdkworkGenerationActionRequest,
  ): Promise<SdkworkGenerationCommandResponse>;
  soundEffects: SdkworkGenerationsSoundEffectsResourceClient;
  timeline: SdkworkGenerationsTimelineResourceClient;
  videos: SdkworkGenerationsVideosResourceClient;
  voice: SdkworkGenerationsVoiceResourceClient;
}

export interface SdkworkGenerationsAppClient {
  generations: SdkworkGenerationsResourceClient;
}

export interface SdkworkGenerationSdkClients {
  generationsApp?: SdkworkGenerationsAppClient;
  tokenManager?: SdkworkGenerationTokenManager;
}

export interface SdkworkGenerationTokenManager {
  getAccessToken?: () => string | undefined;
  getAuthToken?: () => string | undefined;
  hasToken?: () => boolean;
}

const DEFAULT_GENERATION_PAGE_SIZE = 20;

function normalizeText(value: string | undefined): string {
  return (value ?? "").trim().toLowerCase();
}

function resolveSettledValue<T>(
  result: PromiseSettledResult<T>,
  fallback: T,
): T {
  return result.status === "fulfilled" ? result.value : fallback;
}

function readDefaultGenerationSessionTokens(): { authToken?: string } {
  return {};
}

function hasAuthenticatedSession(
  getSessionTokens: () => { authToken?: string },
  sdkClients: SdkworkGenerationSdkClients | undefined,
): boolean {
  const tokenManager = sdkClients?.tokenManager;
  if (tokenManager?.hasToken?.()) {
    return true;
  }

  return Boolean(normalizeText(
    tokenManager?.getAuthToken?.()
      ?? tokenManager?.getAccessToken?.()
      ?? getSessionTokens().authToken,
  ));
}

function toTimestamp(value: string | undefined): number {
  const timestamp = Date.parse(value ?? "");
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function toRunStatus(status: SdkworkGenerationRemoteStatus): SdkworkGenerationRun["status"] {
  if (status === "succeeded") {
    return "completed";
  }
  if (status === "canceled") {
    return "failed";
  }
  if (status === "requires_action") {
    return "queued";
  }
  return status;
}

function toTitleToken(value: string | undefined): string {
  const normalized = (value ?? "")
    .trim()
    .replace(/[-_]+/gu, " ")
    .replace(/\s+/gu, " ")
    .toLowerCase();

  return normalized ? normalized : "";
}

function toGenerationRunTitle(record: SdkworkGenerationRecord): string {
  const modality = toTitleToken(record.modality);
  const operationType = toTitleToken(record.operationType);
  const title = [modality, operationType].filter(Boolean).join(" ");

  if (!title) {
    return record.id;
  }

  return title.charAt(0).toUpperCase() + title.slice(1);
}

function toGenerationRunLatency(record: SdkworkGenerationRecord): number {
  const createdAt = toTimestamp(record.createdAt);
  const updatedAt = toTimestamp(record.updatedAt);
  if (!createdAt || !updatedAt || updatedAt < createdAt) {
    return 0;
  }

  return updatedAt - createdAt;
}

export function mapSdkworkGenerationRecordToRun(
  record: SdkworkGenerationRecord,
): SdkworkGenerationRun {
  return {
    id: record.id,
    latencyMs: toGenerationRunLatency(record),
    model: record.sourceProvider?.trim() || "sdkwork-generations",
    promptPreview: record.promptPreview?.trim() || record.operationType || record.id,
    status: toRunStatus(record.status),
    title: toGenerationRunTitle(record),
    tokensUsed: 0,
    updatedAt: record.updatedAt,
  };
}

async function listRunsFromGenerationsApp(
  client: SdkworkGenerationsAppClient,
  pageSize: number,
): Promise<readonly SdkworkGenerationRun[]> {
  const page = await client.generations.list({ pageSize });
  return (page.items ?? []).map(mapSdkworkGenerationRecordToRun);
}

function requireGenerationsResourceClient(
  sdkClients: SdkworkGenerationSdkClients | undefined,
): SdkworkGenerationsResourceClient {
  const generations = sdkClients?.generationsApp?.generations;
  if (!generations) {
    throw new Error("generations app SDK client is required");
  }

  return generations;
}

function createCommandRequest(input: SdkworkGenerationCommandInput): SdkworkGenerationCommandRequest {
  return {
    ...(input.organizationId ? { organizationId: input.organizationId } : {}),
    prompt: input.prompt,
    ...(input.model ? { model: input.model } : {}),
    ...(input.inputAssetIds ? { inputAssetIds: [...input.inputAssetIds] } : {}),
    ...(input.parameters ? { parameters: input.parameters } : {}),
  };
}

function createIdempotencyParams(
  idempotencyKey: string | undefined,
): SdkworkGenerationIdempotencyParams {
  return idempotencyKey ? { idempotencyKey } : {};
}

function createActionRequest(
  reason: string | undefined,
): SdkworkGenerationActionRequest | undefined {
  return reason ? { reason } : undefined;
}

function createListParams(
  input: Pick<SdkworkGenerationResultsListInput, "cursor" | "pageSize">,
): SdkworkGenerationResourceListParams | undefined {
  const params: SdkworkGenerationResourceListParams = {
    ...(input.cursor ? { cursor: input.cursor } : {}),
    ...(input.pageSize !== undefined ? { pageSize: input.pageSize } : {}),
  };

  return Object.keys(params).length > 0 ? params : undefined;
}

function createCommandResult(
  response: SdkworkGenerationCommandResponse,
): SdkworkGenerationCommandResult {
  return {
    generation: mapSdkworkGenerationRecordToRun(response.generation),
    record: response.generation,
  };
}

type SdkworkGenerationCommandInvoker = (
  body: SdkworkGenerationCommandRequest,
  params?: SdkworkGenerationCommandParams,
) => Promise<SdkworkGenerationCommandResponse>;

function resolveCommandInvoker(
  generations: SdkworkGenerationsResourceClient,
  input: Pick<SdkworkGenerationCommandInput, "modality" | "operationType">,
): SdkworkGenerationCommandInvoker {
  if (input.modality === "image" && input.operationType === "text_to_image") {
    return generations.images.textToImage.bind(generations.images);
  }
  if (input.modality === "image" && input.operationType === "image_edit") {
    return generations.images.imageEdit.bind(generations.images);
  }
  if (input.modality === "video" && input.operationType === "text_to_video") {
    return generations.videos.textToVideo.bind(generations.videos);
  }
  if (input.modality === "video" && input.operationType === "image_to_video") {
    return generations.videos.imageToVideo.bind(generations.videos);
  }
  if (input.modality === "video" && input.operationType === "video_extend") {
    return generations.videos.videoExtend.bind(generations.videos);
  }
  if (input.modality === "music" && input.operationType === "text_to_music") {
    return generations.music.textToMusic.bind(generations.music);
  }
  if (input.modality === "music" && input.operationType === "lyrics_to_music") {
    return generations.music.lyricsToMusic.bind(generations.music);
  }
  if (input.modality === "sfx" && input.operationType === "sound_effect") {
    return generations.soundEffects.create.bind(generations.soundEffects);
  }
  if ((input.modality === "voice" || input.modality === "audio") && input.operationType === "speech") {
    return generations.voice.speech.bind(generations.voice);
  }
  if (input.modality === "voice" && input.operationType === "transcription") {
    return generations.voice.transcription.bind(generations.voice);
  }
  if (input.modality === "voice" && input.operationType === "translation") {
    return generations.voice.translation.bind(generations.voice);
  }

  throw new Error(`Unsupported generation operation: ${input.modality}/${input.operationType}`);
}

export function createSdkworkGenerationService(
  options: CreateSdkworkGenerationServiceOptions = {},
): SdkworkGenerationService {
  const getSessionTokens = options.getSessionTokens ?? readDefaultGenerationSessionTokens;
  const fallbackWorkspace = createEmptySdkworkGenerationWorkspace({
    includeSampleRuns: options.includeSampleRuns,
    runs: options.runs,
  });

  return {
    async cancelGeneration(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.cancel(
        input.generationId,
        createIdempotencyParams(input.idempotencyKey),
        createActionRequest(input.reason),
      );
    },

    async createGenerationCommand(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      const invokeCommand = resolveCommandInvoker(generations, input);
      const response = await invokeCommand(
        createCommandRequest(input),
        createIdempotencyParams(input.idempotencyKey),
      );

      return createCommandResult(response);
    },

    getEmptyWorkspace() {
      return createEmptySdkworkGenerationWorkspace({
        includeSampleRuns: options.includeSampleRuns,
        isAuthenticated: hasAuthenticatedSession(getSessionTokens, options.sdkClients),
        runs: options.runs ?? fallbackWorkspace.runs,
      });
    },

    async getGeneration(generationId) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.retrieve(generationId);
    },

    async getWorkspace() {
      const isAuthenticated = hasAuthenticatedSession(getSessionTokens, options.sdkClients);
      const generationsApp = options.sdkClients?.generationsApp;
      const listRuns = generationsApp
        ? () => listRunsFromGenerationsApp(generationsApp, options.pageSize ?? DEFAULT_GENERATION_PAGE_SIZE)
        : options.listRuns;

      if (!listRuns) {
        return createEmptySdkworkGenerationWorkspace({
          includeSampleRuns: options.includeSampleRuns,
          isAuthenticated,
          runs: options.runs ?? fallbackWorkspace.runs,
        });
      }

      const results = await Promise.allSettled([listRuns()]);
      const runs = resolveSettledValue(results[0], options.runs ?? fallbackWorkspace.runs);

      return createEmptySdkworkGenerationWorkspace({
        includeSampleRuns: options.includeSampleRuns,
        isAuthenticated,
        runs,
      });
    },

    async listGenerationRecords(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.list(input);
    },

    async listGenerationResults(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.results.list(
        input.generationId,
        createListParams(input),
      );
    },

    async listGenerationTimeline(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.timeline.list(
        input.generationId,
        createListParams(input),
      );
    },

    async retryGeneration(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      const response = await generations.retry(
        input.generationId,
        createIdempotencyParams(input.idempotencyKey),
        createActionRequest(input.reason),
      );

      return createCommandResult(response);
    },

    async saveGenerationResultToAssets(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.results.saveToAssets(
        input.generationId,
        input.resultId,
        {
          ...(input.collectionId ? { collectionId: input.collectionId } : {}),
          ...(input.title ? { title: input.title } : {}),
          ...(input.tags ? { tags: [...input.tags] } : {}),
        },
        createIdempotencyParams(input.idempotencyKey),
      );
    },

    async setFavorite(input) {
      const generations = requireGenerationsResourceClient(options.sdkClients);
      return generations.favorite(
        input.generationId,
        {
          favorite: input.favorite,
        },
        createIdempotencyParams(undefined),
      );
    },
  };
}

export const sdkworkGenerationService = createSdkworkGenerationService();
