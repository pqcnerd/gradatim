import type { TranslateLinePayload, TranslateLineResponse } from '../types/electron';

export type TranslationResult =
  | { kind: 'ok'; code: string }
  | { kind: 'unhandled'; placeholder: string; message?: string }
  | { kind: 'error'; message: string };

export async function translateLine(payload: TranslateLinePayload): Promise<TranslationResult> {
  if (!window.electronAPI?.translateLine) {
    return {
      kind: 'error',
      message: 'Bridge unavailable: rust-core not reachable from renderer.',
    };
  }

  try {
    const response: TranslateLineResponse = await window.electronAPI.translateLine(payload);
    if (response.kind === 'ok' && response.code) {
      return { kind: 'ok', code: response.code };
    }

    if (response.kind === 'unhandled' && response.code) {
      return { kind: 'unhandled', placeholder: response.code, message: response.message ?? undefined };
    }

    return { kind: 'error', message: response.message ?? 'Unknown error' };
  } catch (error) {
    return { kind: 'error', message: (error as Error).message };
  }
}

