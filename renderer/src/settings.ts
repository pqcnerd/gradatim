import type { EditorSettings } from './types/electron';

const DEFAULT_SETTINGS: EditorSettings = {
  autoTranslate: true,
  targetLanguage: 'c',
  maxLinesPerTranslation: 3,
  context: {
    beforeChars: 1600,
    afterChars: 900,
  },
  ai: {
    provider: 'gemini',
    apiKey: '',
    model: 'gemini-2.5-flash',
  },
  editor: {
    tabSize: 4,
    insertSpaces: true,
    fontSize: 16,
    fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    renderWhitespace: 'trailing',
    cursorStyle: 'line',
  },
  ui: {
    sidebarVisible: true,
    activityVisible: true,
    minimapEnabled: true,
    zoomLevel: 0,
    theme: 'light',
  },
};

interface InitOptions {
  onUpdate(settings: EditorSettings): void;
  onError?(message: string): void;
}

export async function initSettingsUI(options: InitOptions): Promise<EditorSettings> {
  const initial = await loadSettings();
  wireForm(initial, options);
  options.onUpdate(initial);
  return initial;
}

async function loadSettings(): Promise<EditorSettings> {
  if (!window.electronAPI?.loadSettings) {
    return DEFAULT_SETTINGS;
  }

  try {
    const stored = await window.electronAPI.loadSettings();
    return mergeSettings(DEFAULT_SETTINGS, stored);
  } catch {
    return DEFAULT_SETTINGS;
  }
}

function mergeSettings(base: EditorSettings, incoming?: Partial<EditorSettings>): EditorSettings {
  if (!incoming) {
    return base;
  }

  return {
    ...base,
    ...incoming,
    maxLinesPerTranslation: incoming.maxLinesPerTranslation ?? base.maxLinesPerTranslation,
    context: {
      ...base.context,
      ...(incoming.context ?? {}),
    },
    ai: {
      ...base.ai,
      ...(incoming.ai ?? {}),
    },
    editor: {
      ...base.editor,
      ...(incoming.editor ?? {}),
    },
    ui: {
      ...base.ui,
      ...(incoming.ui ?? {}),
    },
  };
}

function wireForm(settings: EditorSettings, options: InitOptions) {
  const modal = document.getElementById('settings-modal');
  const form = document.getElementById('settings-form') as HTMLFormElement | null;
  const openBtn = document.getElementById('open-settings');
  const closeBtn = document.getElementById('close-settings');
  const providerSelect = document.getElementById('ai-provider') as HTMLSelectElement | null;
  const modelInput = document.getElementById('ai-model') as HTMLInputElement | null;
  const providerLink = document.getElementById('ai-key-link') as HTMLAnchorElement | null;

  if (!modal || !form || !openBtn || !closeBtn) {
    return;
  }

  applySettingsToForm(form, settings);
  syncAiProviderUi(providerSelect, modelInput, providerLink);

  providerSelect?.addEventListener('change', () => {
    syncAiProviderUi(providerSelect, modelInput, providerLink);
  });

  openBtn.addEventListener('click', () => modal.classList.add('visible'));
  closeBtn.addEventListener('click', () => modal.classList.remove('visible'));
  modal.addEventListener('click', event => {
    if (event.target === modal) {
      modal.classList.remove('visible');
    }
  });

  form.addEventListener('submit', async event => {
    event.preventDefault();
    const next = {
      ...settings,
      ...collectFormSettings(form),
      ui: settings.ui,
    };

    if (!window.electronAPI?.saveSettings) {
      options.onUpdate(next);
      modal.classList.remove('visible');
      return;
    }

    try {
      const saved = await window.electronAPI.saveSettings(next);
      settings = mergeSettings(settings, saved);
      options.onUpdate(saved);
      modal.classList.remove('visible');
    } catch (error) {
      options.onError?.(`Failed to save settings: ${(error as Error).message}`);
    }
  });
}

function applySettingsToForm(form: HTMLFormElement, settings: EditorSettings) {
  const elements = form.elements as typeof form.elements & {
    autoTranslate: HTMLInputElement;
    beforeChars: HTMLInputElement;
    afterChars: HTMLInputElement;
    aiProvider: HTMLSelectElement;
    aiModel: HTMLInputElement;
    aiApiKey: HTMLInputElement;
    targetLanguage: HTMLSelectElement;
    maxLines: HTMLSelectElement;
    editorTabSize: HTMLSelectElement;
    editorInsertSpaces: HTMLInputElement;
    editorFontSize: HTMLInputElement;
    editorFontFamily: HTMLInputElement;
    editorRenderWhitespace: HTMLSelectElement;
    editorCursorStyle: HTMLSelectElement;
  };

  elements.autoTranslate.checked = settings.autoTranslate;
  elements.beforeChars.value = settings.context.beforeChars.toString();
  elements.afterChars.value = settings.context.afterChars.toString();
  elements.aiProvider.value = normalizeAiProvider(settings.ai.provider);
  elements.aiModel.value = normalizeAiModel(settings.ai.model);
  elements.aiApiKey.value = settings.ai.apiKey;
  elements.targetLanguage.value = settings.targetLanguage;
  elements.maxLines.value = settings.maxLinesPerTranslation.toString();
  elements.editorTabSize.value = (settings.editor?.tabSize ?? DEFAULT_SETTINGS.editor.tabSize).toString();
  elements.editorInsertSpaces.checked =
    settings.editor?.insertSpaces ?? DEFAULT_SETTINGS.editor.insertSpaces;
  elements.editorFontSize.value = (
    settings.editor?.fontSize ?? DEFAULT_SETTINGS.editor.fontSize
  ).toString();
  elements.editorFontFamily.value =
    settings.editor?.fontFamily ?? DEFAULT_SETTINGS.editor.fontFamily;
  elements.editorRenderWhitespace.value =
    settings.editor?.renderWhitespace ?? DEFAULT_SETTINGS.editor.renderWhitespace;
  elements.editorCursorStyle.value =
    settings.editor?.cursorStyle ?? DEFAULT_SETTINGS.editor.cursorStyle;
}

function collectFormSettings(form: HTMLFormElement): EditorSettings {
  const formData = new FormData(form);
  return {
    autoTranslate: formData.get('autoTranslate') === 'on',
    targetLanguage: normalizeLanguage(
      (formData.get('targetLanguage') as string) || DEFAULT_SETTINGS.targetLanguage
    ),
    maxLinesPerTranslation: normalizeMaxLines(
      Number(formData.get('maxLines')) || DEFAULT_SETTINGS.maxLinesPerTranslation
    ),
    context: {
      beforeChars: Number(formData.get('beforeChars')) || DEFAULT_SETTINGS.context.beforeChars,
      afterChars: Number(formData.get('afterChars')) || DEFAULT_SETTINGS.context.afterChars,
    },
    ai: {
      provider: normalizeAiProvider(
        (formData.get('aiProvider') as string) || DEFAULT_SETTINGS.ai.provider
      ),
      apiKey: (formData.get('aiApiKey') as string) ?? '',
      model: normalizeAiModel(
        (formData.get('aiModel') as string) || DEFAULT_SETTINGS.ai.model,
        normalizeAiProvider((formData.get('aiProvider') as string) || DEFAULT_SETTINGS.ai.provider)
      ),
    },
    editor: {
      tabSize: normalizeTabSize(Number(formData.get('editorTabSize'))),
      insertSpaces: formData.get('editorInsertSpaces') === 'on',
      fontSize: normalizeFontSize(Number(formData.get('editorFontSize'))),
      fontFamily:
        (formData.get('editorFontFamily') as string) || DEFAULT_SETTINGS.editor.fontFamily,
      renderWhitespace: normalizeRenderWhitespace(
        (formData.get('editorRenderWhitespace') as string) ||
          DEFAULT_SETTINGS.editor.renderWhitespace
      ),
      cursorStyle: normalizeCursorStyle(
        (formData.get('editorCursorStyle') as string) ||
          DEFAULT_SETTINGS.editor.cursorStyle
      ),
    },
  };
}

function normalizeLanguage(value: string): string {
  const normalized = value.toLowerCase();
  return ['c', 'python'].includes(normalized) ? normalized : 'c';
}

function normalizeMaxLines(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_SETTINGS.maxLinesPerTranslation;
  }
  return Math.min(3, Math.max(1, Math.floor(value)));
}

function normalizeTabSize(value: number): 2 | 4 | 8 {
  return value === 2 || value === 8 ? value : 4;
}

function normalizeFontSize(value: number): number {
  if (!Number.isFinite(value)) {
    return DEFAULT_SETTINGS.editor.fontSize;
  }
  return Math.min(32, Math.max(10, Math.floor(value)));
}

function normalizeRenderWhitespace(value: string): 'none' | 'trailing' | 'all' {
  const normalized = value.toLowerCase();
  if (normalized === 'none' || normalized === 'all') {
    return normalized;
  }
  return 'trailing';
}

function normalizeCursorStyle(value: string): 'line' | 'block' | 'underline' {
  const normalized = value.toLowerCase();
  if (normalized === 'block' || normalized === 'underline') {
    return normalized;
  }
  return 'line';
}

function normalizeAiProvider(value: string): 'gemini' | 'openai' | 'deepseek' | 'other' {
  const normalized = value.toLowerCase();
  if (normalized === 'openai' || normalized === 'deepseek' || normalized === 'other') {
    return normalized;
  }
  return 'gemini';
}

function normalizeAiModel(
  value: string,
  provider: 'gemini' | 'openai' | 'deepseek' | 'other' = 'gemini'
): string {
  const trimmed = value.trim();
  if (trimmed.length) {
    return trimmed;
  }
  if (provider === 'openai') {
    return 'gpt-4.1-nano';
  }
  if (provider === 'deepseek') {
    return 'deepseek-chat';
  }
  if (provider === 'other') {
    return 'gpt-4.1-nano';
  }
  return 'gemini-2.5-flash';
}

function syncAiProviderUi(
  providerSelect: HTMLSelectElement | null,
  modelInput: HTMLInputElement | null,
  providerLink: HTMLAnchorElement | null
) {
  if (!providerSelect || !modelInput || !providerLink) {
    return;
  }
  const provider = normalizeAiProvider(providerSelect.value);
  if (!modelInput.value.trim()) {
    modelInput.value = normalizeAiModel('', provider);
  }

  if (provider === 'openai') {
    modelInput.placeholder = 'gpt-4.1-nano';
    providerLink.href = 'https://platform.openai.com/api-keys';
    providerLink.textContent = 'Get an OpenAI API key';
    return;
  }
  if (provider === 'deepseek') {
    modelInput.placeholder = 'deepseek-chat';
    providerLink.href = 'https://platform.deepseek.com/api_keys';
    providerLink.textContent = 'Get a DeepSeek API key';
    return;
  }
  if (provider === 'other') {
    modelInput.placeholder = 'Provider model name (OpenAI-compatible)';
    providerLink.href = 'https://platform.openai.com/docs/api-reference/authentication';
    providerLink.textContent = 'OpenAI-compatible API key docs';
    return;
  }
  modelInput.placeholder = 'gemini-2.5-flash';
  providerLink.href = 'https://aistudio.google.com/app/apikey';
  providerLink.textContent = 'Get a Gemini API key';
}

