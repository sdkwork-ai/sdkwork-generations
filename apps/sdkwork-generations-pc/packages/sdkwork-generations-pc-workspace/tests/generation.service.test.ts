import { describe, expect, it, vi } from "vitest";
import {
  createSdkworkGenerationService,
  type SdkworkGenerationCommandModality,
  type SdkworkGenerationOperationType,
} from "../src/generation-service";

const sdkGenerationRecord = {
  id: "generation-1",
  tenantId: "100001",
  organizationId: "0",
  userId: "user-1",
  modality: "image",
  operationType: "text_to_image",
  sourceProvider: "openai",
  promptPreview: "Generate a workspace banner",
  status: "succeeded",
  resultCount: 2,
  createdAt: "2026-04-03T01:00:00.000Z",
  updatedAt: "2026-04-03T01:00:02.500Z",
} as const;

function createGenerationsResourceClient(overrides = {}) {
  return {
    cancel: vi.fn().mockResolvedValue({
      ...sdkGenerationRecord,
      status: "canceled",
    }),
    favorite: vi.fn().mockResolvedValue({
      ...sdkGenerationRecord,
      favorite: true,
    }),
    retrieve: vi.fn().mockResolvedValue(sdkGenerationRecord),
    images: {
      imageEdit: vi.fn().mockResolvedValue({ generation: sdkGenerationRecord }),
      textToImage: vi.fn().mockResolvedValue({ generation: sdkGenerationRecord }),
    },
    list: vi.fn().mockResolvedValue({ items: [sdkGenerationRecord] }),
    music: {
      lyricsToMusic: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "music",
          operationType: "lyrics_to_music",
        },
      }),
      textToMusic: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "music",
          operationType: "text_to_music",
        },
      }),
    },
    results: {
      list: vi.fn().mockResolvedValue({
        items: [
          {
            id: "result-1",
            generationId: "generation-1",
            resultType: "image",
            driveSpaceId: "space-1",
            driveNodeId: "node-1",
            driveUri: "drive://space-1/node-1",
            previewText: "Preview",
            createdAt: "2026-04-03T01:00:03.000Z",
          },
        ],
        nextCursor: "next-results",
      }),
      saveToAssets: vi.fn().mockResolvedValue({
        id: "result-1",
        generationId: "generation-1",
        resultType: "image",
        assetId: "asset-1",
        createdAt: "2026-04-03T01:00:03.000Z",
      }),
    },
    retry: vi.fn().mockResolvedValue({ generation: sdkGenerationRecord }),
    soundEffects: {
      create: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "sfx",
          operationType: "sound_effect",
        },
      }),
    },
    timeline: {
      list: vi.fn().mockResolvedValue({
        items: [
          {
            id: "event-1",
            generationId: "generation-1",
            eventType: "generation.created",
            message: "Queued",
            payload: { queue: "default" },
            createdAt: "2026-04-03T01:00:00.000Z",
          },
        ],
      }),
    },
    videos: {
      imageToVideo: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "video",
          operationType: "image_to_video",
        },
      }),
      textToVideo: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "video",
          operationType: "text_to_video",
        },
      }),
      videoExtend: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "video",
          operationType: "video_extend",
        },
      }),
    },
    voice: {
      speech: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "voice",
          operationType: "speech",
        },
      }),
      transcription: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "voice",
          operationType: "transcription",
        },
      }),
      translation: vi.fn().mockResolvedValue({
        generation: {
          ...sdkGenerationRecord,
          modality: "voice",
          operationType: "translation",
        },
      }),
    },
    ...overrides,
  };
}

type GenerationsResourceClientFake = ReturnType<typeof createGenerationsResourceClient>;
type GenerationCommandSpy = GenerationsResourceClientFake["images"]["textToImage"];

describe("sdkwork-generations-pc-workspace service", () => {
  it("loads runs through the injected generated app SDK client", async () => {
    const list = vi.fn().mockResolvedValue({
      items: [
        {
          id: "sdk-run",
          tenantId: "100001",
          userId: "user-1",
          modality: "image",
          operationType: "text_to_image",
          sourceProvider: "openai",
          promptPreview: "Generate a workspace banner",
          status: "succeeded",
          resultCount: 2,
          createdAt: "2026-04-03T01:00:00.000Z",
          updatedAt: "2026-04-03T01:00:02.500Z",
        },
      ],
    });

    const service = createSdkworkGenerationService({
      getSessionTokens: () => ({ authToken: "token" }),
      includeSampleRuns: false,
      sdkClients: {
        generationsApp: {
          generations: createGenerationsResourceClient({ list }),
        },
      },
    });

    const workspace = await service.getWorkspace();

    expect(list).toHaveBeenCalledWith({ pageSize: 20 });
    expect(workspace.isAuthenticated).toBe(true);
    expect(workspace.runs).toEqual([
      {
        id: "sdk-run",
        latencyMs: 2500,
        model: "openai",
        promptPreview: "Generate a workspace banner",
        status: "completed",
        title: "Image text to image",
        tokensUsed: 0,
        updatedAt: "2026-04-03T01:00:02.500Z",
      },
    ]);
    expect(workspace.digest).toEqual({
      completedRuns: 1,
      failedRuns: 0,
      runningRuns: 0,
      totalRuns: 1,
      totalTokensUsed: 0,
    });
  });

  it("uses fallback runs when the generated app SDK list operation fails", async () => {
    const service = createSdkworkGenerationService({
      includeSampleRuns: false,
      runs: [
        {
          id: "fallback",
          latencyMs: 1200,
          model: "gpt-5.4-mini",
          promptPreview: "Fallback",
          status: "queued",
          title: "Fallback Run",
          tokensUsed: 300,
          updatedAt: "2026-04-01T01:00:00.000Z",
        },
      ],
      sdkClients: {
        generationsApp: {
          generations: createGenerationsResourceClient({
            list: vi.fn().mockRejectedValue(new Error("down")),
          }),
        },
      },
    });

    const workspace = await service.getWorkspace();

    expect(workspace.runs[0]?.id).toBe("fallback");
  });

  it("delegates filtered generation record list queries to the injected generated app SDK resource", async () => {
    const list = vi.fn().mockResolvedValue({
      items: [sdkGenerationRecord],
      nextCursor: "next-records",
    });
    const service = createSdkworkGenerationService({
      sdkClients: {
        generationsApp: {
          generations: createGenerationsResourceClient({ list }),
        },
      },
    });

    await expect(service.listGenerationRecords({
      cursor: "cursor-1",
      pageSize: 15,
      status: "succeeded",
      modality: "image",
      operationType: "text_to_image",
      q: "workspace banner",
    })).resolves.toEqual({
      items: [sdkGenerationRecord],
      nextCursor: "next-records",
    });

    expect(list).toHaveBeenCalledWith({
      cursor: "cursor-1",
      pageSize: 15,
      status: "succeeded",
      modality: "image",
      operationType: "text_to_image",
      q: "workspace banner",
    });
  });

  it("uses fallback runs when remote list operation fails", async () => {
    const listRuns = vi.fn()
      .mockResolvedValueOnce([
        {
          id: "remote",
          latencyMs: 900,
          model: "gpt-5.4-mini",
          promptPreview: "Remote",
          status: "completed",
          title: "Remote Run",
          tokensUsed: 1000,
          updatedAt: "2026-04-03T01:00:00.000Z",
        },
      ])
      .mockRejectedValueOnce(new Error("down"));

    const service = createSdkworkGenerationService({
      getSessionTokens: () => ({ authToken: "token" }),
      listRuns,
      runs: [
        {
          id: "fallback",
          latencyMs: 1200,
          model: "gpt-5.4-mini",
          promptPreview: "Fallback",
          status: "queued",
          title: "Fallback Run",
          tokensUsed: 300,
          updatedAt: "2026-04-01T01:00:00.000Z",
        },
      ],
    });

    const first = await service.getWorkspace();
    expect(first.runs[0]?.id).toBe("remote");
    expect(first.isAuthenticated).toBe(true);

    const second = await service.getWorkspace();
    expect(second.runs[0]?.id).toBe("fallback");
  });

  it("returns an empty workspace when sample runs are disabled", async () => {
    const service = createSdkworkGenerationService({
      getSessionTokens: () => ({ authToken: "token" }),
      includeSampleRuns: false,
    });

    const emptyWorkspace = service.getEmptyWorkspace();
    expect(emptyWorkspace.isAuthenticated).toBe(true);
    expect(emptyWorkspace.runs).toEqual([]);
    expect(emptyWorkspace.digest).toEqual({
      completedRuns: 0,
      failedRuns: 0,
      runningRuns: 0,
      totalRuns: 0,
      totalTokensUsed: 0,
    });

    const workspace = await service.getWorkspace();
    expect(workspace.isAuthenticated).toBe(true);
    expect(workspace.runs).toEqual([]);
    expect(workspace.digest).toEqual({
      completedRuns: 0,
      failedRuns: 0,
      runningRuns: 0,
      totalRuns: 0,
      totalTokensUsed: 0,
    });
  });

  it("derives workspace authentication from the injected shared token manager", async () => {
    const service = createSdkworkGenerationService({
      includeSampleRuns: false,
      sdkClients: {
        generationsApp: {
          generations: createGenerationsResourceClient(),
        },
        tokenManager: {
          hasToken: () => true,
        },
      },
    });

    expect(service.getEmptyWorkspace().isAuthenticated).toBe(true);
    await expect(service.getWorkspace()).resolves.toMatchObject({
      isAuthenticated: true,
    });
  });

  it("derives workspace authentication from an injected access-token-only manager", async () => {
    const service = createSdkworkGenerationService({
      includeSampleRuns: false,
      sdkClients: {
        generationsApp: {
          generations: createGenerationsResourceClient(),
        },
        tokenManager: {
          getAccessToken: () => "access-token",
        },
      },
    });

    expect(service.getEmptyWorkspace().isAuthenticated).toBe(true);
  });

  it("routes text-to-image commands through the injected generated app SDK resource", async () => {
    const generations = createGenerationsResourceClient();
    const service = createSdkworkGenerationService({
      sdkClients: {
        generationsApp: {
          generations,
        },
      },
    });

    const result = await service.createGenerationCommand({
      tenantId: "100001",
      organizationId: "0",
      modality: "image",
      operationType: "text_to_image",
      prompt: "Generate a workspace banner",
      model: "gpt-image-1",
      inputAssetIds: ["asset-1"],
      parameters: {
        size: "1024x1024",
      },
      idempotencyKey: "command-1",
    });

    expect(generations.images.textToImage).toHaveBeenCalledWith(
      {
        organizationId: "0",
        prompt: "Generate a workspace banner",
        model: "gpt-image-1",
        inputAssetIds: ["asset-1"],
        parameters: {
          size: "1024x1024",
        },
      },
      {
        idempotencyKey: "command-1",
      },
    );
    expect(result.generation).toEqual({
      id: "generation-1",
      latencyMs: 2500,
      model: "openai",
      promptPreview: "Generate a workspace banner",
      status: "completed",
      title: "Image text to image",
      tokensUsed: 0,
      updatedAt: "2026-04-03T01:00:02.500Z",
    });
  });

  it.each([
    ["image", "image_edit", "generations.images.imageEdit", (generations) => generations.images.imageEdit],
    ["video", "text_to_video", "generations.videos.textToVideo", (generations) => generations.videos.textToVideo],
    ["video", "image_to_video", "generations.videos.imageToVideo", (generations) => generations.videos.imageToVideo],
    ["video", "video_extend", "generations.videos.videoExtend", (generations) => generations.videos.videoExtend],
    ["music", "text_to_music", "generations.music.textToMusic", (generations) => generations.music.textToMusic],
    ["music", "lyrics_to_music", "generations.music.lyricsToMusic", (generations) => generations.music.lyricsToMusic],
    ["sfx", "sound_effect", "generations.soundEffects.create", (generations) => generations.soundEffects.create],
    ["audio", "speech", "generations.voice.speech", (generations) => generations.voice.speech],
    ["voice", "speech", "generations.voice.speech", (generations) => generations.voice.speech],
    ["voice", "transcription", "generations.voice.transcription", (generations) => generations.voice.transcription],
    ["voice", "translation", "generations.voice.translation", (generations) => generations.voice.translation],
  ] as const satisfies readonly [
    SdkworkGenerationCommandModality,
    SdkworkGenerationOperationType,
    string,
    (generations: GenerationsResourceClientFake) => GenerationCommandSpy,
  ][])(
    "routes %s/%s commands through %s",
    async (modality, operationType, _methodLabel, selectMethod) => {
      const generations = createGenerationsResourceClient();
      const service = createSdkworkGenerationService({
        sdkClients: {
          generationsApp: {
            generations,
          },
        },
      });

      await service.createGenerationCommand({
        tenantId: "100001",
        modality,
        operationType,
        prompt: "Create media",
      });

      expect(selectMethod(generations)).toHaveBeenCalledWith(
        {
          prompt: "Create media",
        },
        undefined,
      );
    },
  );

  it("rejects generation commands when no generated app SDK client is injected", async () => {
    const service = createSdkworkGenerationService();

    await expect(service.createGenerationCommand({
      tenantId: "100001",
      modality: "image",
      operationType: "text_to_image",
      prompt: "Create media",
    })).rejects.toThrow("generations app SDK client is required");
  });

  it("rejects unsupported generation command operation types", async () => {
    const service = createSdkworkGenerationService({
      sdkClients: {
        generationsApp: {
          generations: createGenerationsResourceClient(),
        },
      },
    });

    await expect(service.createGenerationCommand({
      tenantId: "100001",
      modality: "image",
      operationType: "text_to_video",
      prompt: "Create media",
    })).rejects.toThrow("Unsupported generation operation: image/text_to_video");
  });

  it("delegates generation record actions to injected generated app SDK resources", async () => {
    const generations = createGenerationsResourceClient();
    const service = createSdkworkGenerationService({
      sdkClients: {
        generationsApp: {
          generations,
        },
      },
    });

    await expect(service.getGeneration("generation-1")).resolves.toEqual(sdkGenerationRecord);
    await expect(service.cancelGeneration({
      generationId: "generation-1",
      reason: "User stopped it",
      idempotencyKey: "cancel-1",
    })).resolves.toMatchObject({ status: "canceled" });
    await expect(service.retryGeneration({
      generationId: "generation-1",
      reason: "Retry after provider failure",
      idempotencyKey: "retry-1",
    })).resolves.toMatchObject({
      generation: {
        id: "generation-1",
      },
    });
    await expect(service.setFavorite({
      generationId: "generation-1",
      favorite: true,
    })).resolves.toMatchObject({ favorite: true });

    expect(generations.retrieve).toHaveBeenCalledWith("generation-1");
    expect(generations.cancel).toHaveBeenCalledWith(
      "generation-1",
      { idempotencyKey: "cancel-1" },
      { reason: "User stopped it" },
    );
    expect(generations.retry).toHaveBeenCalledWith(
      "generation-1",
      { idempotencyKey: "retry-1" },
      { reason: "Retry after provider failure" },
    );
    expect(generations.favorite).toHaveBeenCalledWith(
      "generation-1",
      { favorite: true },
      {},
    );
  });

  it("delegates result and timeline operations to injected generated app SDK resources", async () => {
    const generations = createGenerationsResourceClient();
    const service = createSdkworkGenerationService({
      sdkClients: {
        generationsApp: {
          generations,
        },
      },
    });

    await expect(service.listGenerationResults({
      generationId: "generation-1",
      cursor: "cursor-1",
      pageSize: 5,
    })).resolves.toMatchObject({ nextCursor: "next-results" });

    await expect(service.saveGenerationResultToAssets({
      generationId: "generation-1",
      resultId: "result-1",
      tenantId: "100001",
      collectionId: "collection-1",
      title: "Workspace banner",
      tags: ["workspace", "banner"],
      idempotencyKey: "save-1",
    })).resolves.toMatchObject({ assetId: "asset-1" });

    await expect(service.listGenerationTimeline({
      generationId: "generation-1",
      pageSize: 10,
    })).resolves.toMatchObject({
      items: [
        {
          id: "event-1",
        },
      ],
    });

    expect(generations.results.list).toHaveBeenCalledWith(
      "generation-1",
      {
        cursor: "cursor-1",
        pageSize: 5,
      },
    );
    expect(generations.results.saveToAssets).toHaveBeenCalledWith(
      "generation-1",
      "result-1",
      {
        collectionId: "collection-1",
        title: "Workspace banner",
        tags: ["workspace", "banner"],
      },
      {
        idempotencyKey: "save-1",
      },
    );
    expect(generations.timeline.list).toHaveBeenCalledWith(
      "generation-1",
      {
        pageSize: 10,
      },
    );
  });
});
