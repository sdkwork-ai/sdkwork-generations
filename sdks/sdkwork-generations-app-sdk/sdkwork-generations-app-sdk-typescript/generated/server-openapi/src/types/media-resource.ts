export interface MediaResource {
  mediaResourceId?: string;
  /** Media kind, for example image, video, or audio. */
  kind?: string;
  /** Provenance of the media, for example generated or external_url. */
  source?: string;
  /** Canonical media URL. */
  url?: string;
  /** Public CDN URL when it differs from url. */
  publicUrl?: string;
  /** Provider or drive URI for the media. */
  uri?: string;
  mediaType?: string;
  contentType?: string;
  width?: number;
  height?: number;
  /** Media duration in milliseconds (int64 as decimal string). */
  durationMs?: string;
  /** Media size in bytes (int64 as decimal string). */
  sizeBytes?: string;
  checksumSha256?: string;
  metadata?: Record<string, unknown>;
}
