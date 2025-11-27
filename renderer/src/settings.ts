import type { EditorSettings } from './types/electron';

const DEFAULT_SETTINGS: EditorSettings = {
  rustCoreUrl: 'http://127.0.0.1:4888',
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
    model: 'gemini-1.5-flash',
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
  };
}

function wireForm(settings: EditorSettings, options: InitOptions) {
  const modal = document.getElementById('settings-modal');
  const form = document.getElementById('settings-form') as HTMLFormElement | null;
  const openBtn = document.getElementById('open-settings');
  const closeBtn = document.getElementById('close-settings');

  if (!modal || !form || !openBtn || !closeBtn) {
    return;
  }

  applySettingsToForm(form, settings);

  openBtn.addEventListener('click', () => modal.classList.add('visible'));
  closeBtn.addEventListener('click', () => modal.classList.remove('visible'));
  modal.addEventListener('click', event => {
    if (event.target === modal) {
      modal.classList.remove('visible');
    }
  });

  form.addEventListener('submit', async event => {
    event.preventDefault();
    const next = collectFormSettings(form);

    if (!window.electronAPI?.saveSettings) {
      options.onUpdate(next);
      modal.classList.remove('visible');
      return;
    }

    try {
      const saved = await window.electronAPI.saveSettings(next);
      options.onUpdate(saved);
      modal.classList.remove('visible');
    } catch (error) {
      options.onError?.(`Failed to save settings: ${(error as Error).message}`);
    }
  });
}

function applySettingsToForm(form: HTMLFormElement, settings: EditorSettings) {
  const elements = form.elements as typeof form.elements & {
    rustCoreUrl: HTMLInputElement;
    autoTranslate: HTMLInputElement;
    beforeChars: HTMLInputElement;
    afterChars: HTMLInputElement;
    aiModel: HTMLInputElement;
    aiApiKey: HTMLInputElement;
    targetLanguage: HTMLSelectElement;
    maxLines: HTMLSelectElement;
  };

  elements.rustCoreUrl.value = settings.rustCoreUrl;
  elements.autoTranslate.checked = settings.autoTranslate;
  elements.beforeChars.value = settings.context.beforeChars.toString();
  elements.afterChars.value = settings.context.afterChars.toString();
  elements.aiModel.value = settings.ai.model;
  elements.aiApiKey.value = settings.ai.apiKey;
  elements.targetLanguage.value = settings.targetLanguage;
  elements.maxLines.value = settings.maxLinesPerTranslation.toString();
}

function collectFormSettings(form: HTMLFormElement): EditorSettings {
  const formData = new FormData(form);
  return {
    rustCoreUrl: (formData.get('rustCoreUrl') as string) || DEFAULT_SETTINGS.rustCoreUrl,
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
      provider: 'gemini',
      apiKey: (formData.get('aiApiKey') as string) ?? '',
      model: (formData.get('aiModel') as string) || DEFAULT_SETTINGS.ai.model,
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

