export interface TranslateLineResponse {
  kind: 'ok' | 'error';
  code?: string;
  message?: string;
}

export interface TranslateLinePayload {
  english_line: string;
  code_before: string;
  code_after: string;
  language: string;
  line_index: number;
  api_key?: string;
  model?: string;
  max_lines?: number;
}

export interface EditorSettings {
  rustCoreUrl: string;
  autoTranslate: boolean;
  targetLanguage: string;
  maxLinesPerTranslation: number;
  context: {
    beforeChars: number;
    afterChars: number;
  };
  ai: {
    provider: string;
    apiKey: string;
    model: string;
  };
}

declare global {
  interface Window {
    electronAPI?: {
      translateLine(payload: TranslateLinePayload): Promise<TranslateLineResponse>;
      loadSettings(): Promise<EditorSettings>;
      saveSettings(payload: Partial<EditorSettings>): Promise<EditorSettings>;
    };
  }
}

export {};

