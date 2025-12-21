export interface TranslateLineResponse {
  kind: 'ok' | 'error' | 'unhandled';
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
  ui?: {
    sidebarVisible?: boolean;
    activityVisible?: boolean;
    minimapEnabled?: boolean;
    zoomLevel?: number;
  };
}

export interface DirectoryNode {
  type: 'file' | 'folder';
  name: string;
  path: string;
  children?: DirectoryNode[];
}

export interface FileReadResult {
  path: string | null;
  relativePath: string | null;
  absolutePath: string;
  content: string;
  modified: number;
}

export interface FileDialogResult {
  canceled: boolean;
  file?: {
    name: string;
    absolutePath: string;
    relativePath: string | null;
    content: string;
  };
}

export interface SaveResult {
  canceled: boolean;
  absolutePath?: string;
  relativePath?: string | null;
  name?: string;
}

export interface TerminalCreateResult {
  success: boolean;
  id?: string;
  pid?: number;
  error?: string;
}

export interface TerminalDataEvent {
  id: string;
  data: string;
}

export interface TerminalExitEvent {
  id: string;
  exitCode: number;
  signal?: number;
}

export interface TerminalAPI {
  create(options?: {
    shell?: string;
    args?: string[];
    cwd?: string;
    cols?: number;
    rows?: number;
    env?: Record<string, string>;
  }): Promise<TerminalCreateResult>;
  write(id: string, data: string): Promise<boolean>;
  resize(id: string, cols: number, rows: number): Promise<boolean>;
  close(id: string): Promise<boolean>;
  runCommand(command: string, cwd?: string): Promise<TerminalCreateResult>;
  list(): Promise<string[]>;
  onData(callback: (event: TerminalDataEvent) => void): () => void;
  onExit(callback: (event: TerminalExitEvent) => void): () => void;
}

declare global {
  interface Window {
    electronAPI?: {
      translateLine(payload: TranslateLinePayload): Promise<TranslateLineResponse>;
      loadSettings(): Promise<EditorSettings>;
      saveSettings(payload: Partial<EditorSettings>): Promise<EditorSettings>;
      listDirectory(): Promise<{
        rootPath: string;
        rootName: string;
        snapshot: DirectoryNode;
      }>;
      readFile(relativePath: string): Promise<FileReadResult>;
      createFile(relativePath: string): Promise<{ path: string | null; absolutePath: string }>;
      writeFile(payload: { absolutePath?: string; relativePath?: string; content: string }): Promise<{
        absolutePath: string;
        relativePath: string | null;
        name: string;
      }>;
      saveFileAs(payload: { defaultPath?: string; suggestedName?: string; content: string }): Promise<SaveResult>;
      openFileDialog(): Promise<FileDialogResult>;
      openFolderDialog(): Promise<{
        canceled: boolean;
        rootPath?: string;
        rootName?: string;
        snapshot?: DirectoryNode;
      }>;
      getZoomLevel?(): Promise<number>;
      setZoomLevel?(level: number): Promise<number>;
      onMenuCommand?(callback: (command: string) => void): () => void;
      terminal?: TerminalAPI;
    };
  }
}

export {};

