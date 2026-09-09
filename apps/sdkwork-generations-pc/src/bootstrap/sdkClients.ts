import {
  createClient as createDriveAppClient,
  type SdkworkDriveAppClient,
} from "@sdkwork/drive-app-sdk";
import {createTokenManager, resolveBaseUrlWithAlignProtocol, type AuthTokenManager} from "@sdkwork/sdk-common";
import type { SdkworkGenerationSdkClients } from "@sdkwork/generations-pc-workspace";
import {
  createClient as createGenerationsAppClient,
  type SdkworkAppClient as SdkworkGenerationsAppClient,
} from "@sdkwork/generations-app-sdk";

const APP_API_PREFIX = "/app/v3/api";

export interface GenerationsPcRuntimeConfig {
  driveAppApiBaseUrl?: string;
  generationsAppApiBaseUrl?: string;
}

export interface CreateGenerationsPcSdkClientsOptions {
  config?: GenerationsPcRuntimeConfig;
  tokenManager?: AuthTokenManager;
}

export interface GenerationsPcSdkClients extends SdkworkGenerationSdkClients {
  driveApp: SdkworkDriveAppClient;
  generationsApp: SdkworkGenerationsAppClient;
  tokenManager: AuthTokenManager;
}

function readViteEnvValue(name: string): string | undefined {
  const value = import.meta.env[name];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

/**
 * Authored per-surface overrides win; the shared fallback resolves through
 * `@sdkwork/sdk-common` `resolveBaseUrl` (ENVIRONMENT_SPEC.md §6.3): the
 * unified `SDKWORK_API_BASE_URL` candidates are matched against the current
 * page host, environment and deployment profile (`pnpm dev` standalone ->
 * same-origin ip+port, cloud -> local cloud-gateway dev port; built pages ->
 * same origin (standalone) or `api[-<env>].<brand>` (cloud)).
 */
function readGenerationsPcRuntimeConfig(): GenerationsPcRuntimeConfig {
  const sharedAppApiOrigin = resolveBaseUrlWithAlignProtocol().url || undefined;
  return {
    driveAppApiBaseUrl: readViteEnvValue("VITE_SDKWORK_GENERATIONS_DRIVE_APP_API_BASE_URL")
      ?? readViteEnvValue("VITE_SDKWORK_DRIVE_APP_API_BASE_URL")
      ?? readViteEnvValue("VITE_SDKWORK_GENERATIONS_APP_API_BASE_URL")
      ?? readViteEnvValue("VITE_SDKWORK_GENERATIONS_PC_APP_API_BASE_URL")
      ?? (sharedAppApiOrigin ? `${sharedAppApiOrigin}/app/v3/api` : undefined),
    generationsAppApiBaseUrl: readViteEnvValue("VITE_SDKWORK_GENERATIONS_APP_API_BASE_URL")
      ?? readViteEnvValue("VITE_SDKWORK_GENERATIONS_PC_APP_API_BASE_URL")
      ?? (sharedAppApiOrigin ? `${sharedAppApiOrigin}/app/v3/api` : undefined),
  };
}

export function normalizeGeneratedAppSdkBaseUrl(value: string | undefined): string {
  const normalized = (value ?? "").trim().replace(/\/+$/u, "");
  if (!normalized) {
    return "";
  }

  if (normalized.endsWith(APP_API_PREFIX)) {
    return normalized.slice(0, -APP_API_PREFIX.length).replace(/\/+$/u, "");
  }

  return normalized;
}

export function createGenerationsPcSdkClients(
  options: CreateGenerationsPcSdkClientsOptions = {},
): GenerationsPcSdkClients {
  const config = {
    ...readGenerationsPcRuntimeConfig(),
    ...(options.config ?? {}),
  };
  const tokenManager = options.tokenManager ?? createTokenManager();
  const generationsApp = createGenerationsAppClient({
    baseUrl: normalizeGeneratedAppSdkBaseUrl(config.generationsAppApiBaseUrl),
    tokenManager,
  });
  const driveApp = createDriveAppClient({
    baseUrl: normalizeGeneratedAppSdkBaseUrl(config.driveAppApiBaseUrl),
    tokenManager,
  });

  generationsApp.setTokenManager(tokenManager);
  driveApp.setTokenManager(tokenManager);

  return {
    driveApp,
    generationsApp,
    tokenManager,
  };
}
