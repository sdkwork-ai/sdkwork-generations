import type { GenerationModality } from './generation-modality';

export interface GenerationPricingEstimateRequest {
  modality: GenerationModality;
  /** Explicit operation the command would use. Omitted operations resolve through the modality defaults the creation endpoints apply. */
  operationType?: 'text_to_image' | 'image_edit' | 'text_to_video' | 'image_to_video' | 'video_extend' | 'avatar' | 'motion_mimicry' | 'text_to_music' | 'lyrics_to_music' | 'sound_effects' | 'speech' | 'transcription' | 'translation';
  /** Model id the command would carry. Required for a priced estimate; a request without one answers `priced: false` instead of guessing a default. */
  model?: string;
  /** Free-form generation parameters exactly as the command carries them, including the `generationConfig` block (aspectRatio, resolution, durationSeconds, imageCount, videoMode, ...). */
  parameters?: Record<string, unknown>;
  /** Prompt the command would carry. Only its character length is read, for character-metered modalities (voice/speech); the text itself is not persisted by the estimate. */
  prompt?: string;
}
