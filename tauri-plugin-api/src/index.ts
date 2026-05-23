import { invoke, Channel } from '@tauri-apps/api/core';

export interface SynthesizeResponse {
  wavBase64: string;
  durationSecs: number;
  sampleRate: number;
}

export interface ModelProgressEvent {
  file: string;
  bytesDownloaded: number;
  totalBytes: number | null;
}

export interface StatusResponse {
  engineLoaded: boolean;
  currentVoice: string;
  sampleRate: number;
}

/** Synthesize text to audio, returns base64-encoded WAV */
export async function synthesize(
  text: string,
  lang?: string,
  totalStep?: number,
  speed?: number,
  silenceDuration?: number,
): Promise<SynthesizeResponse> {
  return await invoke<SynthesizeResponse>('plugin:supertonic|synthesize', {
    text,
    lang: lang ?? 'en',
    totalStep,
    speed,
    silenceDuration,
  });
}

/** Download and load a TTS model from HuggingFace */
export async function loadModel(
  modelId?: string,
  voiceStyle?: string,
  onProgress?: (event: ModelProgressEvent) => void,
): Promise<void> {
  const channel = new Channel<ModelProgressEvent>();
  if (onProgress) {
    channel.onmessage = onProgress;
  }
  return await invoke('plugin:supertonic|load_model', {
    modelId: modelId ?? 'Supertone/supertonic-3',
    voiceStyle: voiceStyle ?? 'M1',
    onProgress: channel,
  });
}

/** List available voice styles */
export async function listVoices(): Promise<string[]> {
  return await invoke<string[]>('plugin:supertonic|list_voices');
}

/** Select a voice style by name */
export async function selectVoice(voiceName: string): Promise<void> {
  return await invoke('plugin:supertonic|select_voice', {
    voiceName,
  });
}

/** Get engine status */
export async function getStatus(): Promise<StatusResponse> {
  return await invoke<StatusResponse>('plugin:supertonic|get_status');
}
