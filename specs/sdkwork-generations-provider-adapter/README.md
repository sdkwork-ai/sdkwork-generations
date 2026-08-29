# sdkwork-generations-provider-adapter

Vendor adapters that dispatch generation commands to external AI services
through the generated cloudrouter Rust SDK (`cloudrouter_open_sdk::SdkworkAiClient`).
One `GenerationProvider` per modality (image, video, music, voice, sfx);
vendor surfaces: OpenAI images/video/audio, nano-banana, midjourney, vidu,
kling (image+video), volcengine (image+video), suno.
