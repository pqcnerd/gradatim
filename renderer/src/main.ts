import * as monaco from 'monaco-editor';
import './style.css';
import { translateLine } from './api/translatorClient';
import { initSettingsUI } from './settings';
import type { EditorSettings, TranslateLinePayload } from './types/electron';

type LineTrigger = 'enter' | 'shortcut' | 'regenerate';

const editorContainer = document.getElementById('editor');
const statusList = document.getElementById('status-log');
const autoToggle = document.getElementById('auto-toggle') as HTMLInputElement | null;
const languageSelect = document.getElementById('language-select') as HTMLSelectElement | null;
const regenerateButton = document.getElementById('regenerate-btn') as HTMLButtonElement | null;
const toast = document.getElementById('toast');

if (!editorContainer || !statusList || !toast) {
  throw new Error('Renderer root elements are missing. Check index.html structure.');
}

const welcomeSnippet = [
  '// Welcome to Gradatim',
  '// Type an English instruction and press Enter.',
  '',
  'declare a, b 0',
  'set a to 5',
  'if a is greater than b print a otherwise print b'
].join('\n');

const editor = monaco.editor.create(editorContainer, {
  language: 'c',
  value: welcomeSnippet,
  automaticLayout: true,
  minimap: { enabled: false },
  fontSize: 16,
  fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
  cursorSmoothCaretAnimation: 'on',
  renderWhitespace: 'trailing',
  scrollBeyondLastLine: false,
  theme: 'vs-dark'
});

let autoTranslate = true;
let lastDecorations: string[] = [];
let requestCounter = 0;
const pendingRequests = new Map<number, number>();
let lastTriggeredLine: number | null = null;
let toastTimeout: ReturnType<typeof setTimeout> | undefined;
let ignoreAutoToggleChange = true;
const contextLimits = { before: 1600, after: 900 };
let currentSettings: EditorSettings | null = null;
const defaultMaxLines = 3;

autoToggle?.addEventListener('change', (event: Event) => {
  if (ignoreAutoToggleChange) {
    return;
  }

  autoTranslate = (event.target as HTMLInputElement).checked;
  pushStatus(`Auto translate ${autoTranslate ? 'enabled' : 'paused'}.`);
});

regenerateButton?.addEventListener('click', () => {
  if (lastTriggeredLine == null) {
    showToast('Translate a line first to enable regeneration.');
    return;
  }
  enqueueTranslation(lastTriggeredLine, 'regenerate');
});

languageSelect?.addEventListener('change', (event: Event) => {
  const nextLanguage = (event.target as HTMLSelectElement).value;
  if (!currentSettings) {
    currentSettings = {
      rustCoreUrl: '',
      autoTranslate,
      targetLanguage: nextLanguage,
      maxLinesPerTranslation: defaultMaxLines,
      context: { beforeChars: contextLimits.before, afterChars: contextLimits.after },
      ai: { provider: 'gemini', apiKey: '', model: 'gemini-1.5-flash' },
    };
  } else {
    currentSettings.targetLanguage = nextLanguage;
  }

  const savePromise = window.electronAPI?.saveSettings?.({ targetLanguage: nextLanguage });
  savePromise?.catch(error => {
    showToast(
      `Failed to persist language: ${error instanceof Error ? error.message : String(error)}`
    );
  });
});

const pushStatus = (message: string) => {
  const li = document.createElement('li');
  li.textContent = `${new Date().toLocaleTimeString()} — ${message}`;
  statusList.prepend(li);

  while (statusList.childElementCount > 6) {
    const tail = statusList.lastElementChild;
    if (tail) {
      statusList.removeChild(tail);
    } else {
      break;
    }
  }
};

const showToast = (message: string) => {
  toast.textContent = message;
  toast.classList.add('visible');
  if (toastTimeout) {
    clearTimeout(toastTimeout);
  }
  toastTimeout = window.setTimeout(() => {
    toast.classList.remove('visible');
  }, 3200);
};

const highlightLine = (lineNumber: number, variant: 'pending' | 'error' | 'success' = 'pending') => {
  const className = variant === 'error' ? 'line-commit-error' : 'line-commit-decoration';
  lastDecorations = editor.deltaDecorations(lastDecorations, [
    {
      range: new monaco.Range(lineNumber, 1, lineNumber, 1),
      options: {
        isWholeLine: true,
        className,
        linesDecorationsClassName: 'line-commit-gutter'
      }
    }
  ]);

  setTimeout(() => {
    lastDecorations = editor.deltaDecorations(lastDecorations, []);
  }, 1100);
};

const collectContext = (lineNumber: number) => {
  const model = editor.getModel();
  if (!model) {
    return { code_before: '', code_after: '' };
  }

  const beforeRange = new monaco.Range(1, 1, lineNumber, 1);
  const afterRange = new monaco.Range(lineNumber + 1, 1, model.getLineCount() + 1, 1);
  const rawBefore = model.getValueInRange(beforeRange).trimEnd();
  const rawAfter = model.getValueInRange(afterRange).trim();

  return {
    code_before: rawBefore.slice(-contextLimits.before),
    code_after: rawAfter.slice(0, contextLimits.after)
  };
};

const ensureStructuralCompleteness = (snippet: string, language: string | undefined): string => {
  const lang = (language ?? 'c').toLowerCase();
  if (lang === 'python') {
    const trimmed = snippet.trimEnd();
    if (trimmed.endsWith(':') && !trimmed.includes('\n')) {
      return `${trimmed}\n    pass`;
    }
    return snippet;
  }

  // Default to C-like braces.
  const openBraces = (snippet.match(/{/g) ?? []).length;
  const closeBraces = (snippet.match(/}/g) ?? []).length;
  if (openBraces > closeBraces) {
    let result = snippet;
    const missing = openBraces - closeBraces;
    const endsWithOpener = snippet.trimEnd().endsWith('{');
    for (let i = 0; i < missing; i += 1) {
      if (endsWithOpener && i === 0) {
        result += '\n    \n}';
      } else {
        result += '\n}';
      }
    }
    return result;
  }

  if (openBraces === closeBraces && snippet.trimEnd().endsWith('{')) {
    return `${snippet}\n    \n}`;
  }

  return snippet;
};

const applyTranslation = (lineNumber: number, rawSnippet: string) => {
  const model = editor.getModel();
  if (!model) {
    return;
  }

  const snippet = ensureStructuralCompleteness(rawSnippet, currentSettings?.targetLanguage);
  const original = model.getLineContent(lineNumber);
  const indent = original.match(/^\s*/)?.[0] ?? '';
  const indentedSnippet = snippet
    .split('\n')
    .map(part => `${indent}${part}`)
    .join('\n');

  const range = new monaco.Range(lineNumber, 1, lineNumber, original.length + 1);
  model.pushEditOperations([], [{ range, text: indentedSnippet }], () => null);
  highlightLine(lineNumber, 'success');
};

const buildPayload = (lineNumber: number, englishLine: string): TranslateLinePayload => {
  const { code_before, code_after } = collectContext(lineNumber);
  const apiKey = currentSettings?.ai.apiKey.trim();
  return {
    english_line: englishLine,
    code_before,
    code_after,
    language: currentSettings?.targetLanguage ?? 'c',
    line_index: lineNumber - 1,
    api_key: apiKey ? apiKey : undefined,
    model: currentSettings?.ai.model || undefined,
    max_lines: currentSettings?.maxLinesPerTranslation ?? defaultMaxLines,
  };
};

const ENGLISH_CUES = ['declare', 'make', 'create', 'loop', 'otherwise', 'print', 'set', 'function'];
const ENGLISH_CUE_REGEXES = ENGLISH_CUES.map(
  cue => new RegExp(`\\b${cue.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`, 'i')
);
const CODE_PREFIXES = [
  '#include',
  '#define',
  'int ',
  'float ',
  'double ',
  'char ',
  'bool ',
  'void ',
  'struct ',
  'enum ',
  'typedef',
  'return ',
  'if (',
  'for (',
  'while (',
  'do ',
  'switch (',
  'case ',
  'default:',
  '//',
  '/*',
];
const ASSIGNMENT_PATTERN = /^[A-Za-z_][\w]*\s*=/;
const FUNCTION_PATTERN = /^[A-Za-z_][\w\s\*]*\([^)]*\)\s*\{?$/;
const CLASSIFICATION_LABELS: Record<string, string> = {
  prefix: 'looks like real code',
  terminator: 'statement with ;/{/}',
  assignment: 'assignment',
  signature: 'function signature',
};

const containsEnglishCue = (line: string) => ENGLISH_CUE_REGEXES.some(regex => regex.test(line));

const classifyLine = (line: string): string | null => {
  const trimmed = line.trim();
  if (!trimmed.length) {
    return 'empty';
  }

  if (CODE_PREFIXES.some(prefix => trimmed.startsWith(prefix))) {
    return 'prefix';
  }

  if ((trimmed.endsWith(';') || trimmed.endsWith('{') || trimmed.endsWith('}')) && !containsEnglishCue(trimmed)) {
    return 'terminator';
  }

  if (ASSIGNMENT_PATTERN.test(trimmed) && !containsEnglishCue(trimmed)) {
    return 'assignment';
  }

  if (FUNCTION_PATTERN.test(trimmed) && !containsEnglishCue(trimmed)) {
    return 'signature';
  }

  return null;
};

const enqueueTranslation = async (lineNumber: number, trigger: LineTrigger) => {
  const model = editor.getModel();
  if (!model) {
    return;
  }

  const rawLine = model.getLineContent(lineNumber);
  if (!rawLine.trim().length) {
    pushStatus(`Skipped empty line ${lineNumber}.`);
    return;
  }

  if (trigger === 'enter') {
    const classification = classifyLine(rawLine);
    if (classification) {
      const reason = CLASSIFICATION_LABELS[classification] ?? 'real code';
      pushStatus(`Skipped line ${lineNumber} — ${reason}.`);
      return;
    }
  }

  const payload = buildPayload(lineNumber, rawLine.trim());
  const requestId = ++requestCounter;
  pendingRequests.set(lineNumber, requestId);
  lastTriggeredLine = lineNumber;

  highlightLine(lineNumber);
  pushStatus(`Queued line ${lineNumber} via ${trigger}.`);

  const result = await translateLine(payload);

  if (pendingRequests.get(lineNumber) !== requestId) {
    pendingRequests.delete(lineNumber);
    return;
  }

  pendingRequests.delete(lineNumber);

  if (result.kind === 'ok') {
    applyTranslation(lineNumber, result.code);
    const producedLines = Math.max(1, countSnippetLines(result.code));
    const langLabel = (currentSettings?.targetLanguage ?? 'c').toUpperCase();
    pushStatus(
      `Translated line ${lineNumber} → ${producedLines} line${producedLines === 1 ? '' : 's'} (${langLabel}).`
    );
  } else {
    highlightLine(lineNumber, 'error');
    pushStatus(`Line ${lineNumber} failed: ${result.message}`);
    showToast(result.message);
  }
};

editor.onDidChangeModelContent((event: monaco.editor.IModelContentChangedEvent) => {
  if (!autoTranslate) {
    return;
  }

  const enteredNewLine = event.changes.some(
    (change: monaco.editor.IModelContentChange) => change.text.includes('\n')
  );
  if (!enteredNewLine) {
    return;
  }

  const position = editor.getPosition();
  if (!position) {
    return;
  }

  const committedLine = Math.max(1, position.lineNumber - 1);
  enqueueTranslation(committedLine, 'enter');
});

editor.addAction({
  id: 'gradatim-translate-line',
  label: 'Translate current line',
  keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Enter],
  run: () => {
    const position = editor.getPosition();
    if (position) {
      enqueueTranslation(position.lineNumber, 'shortcut');
    }
  }
});

initSettingsUI({
  onUpdate: settings => {
    currentSettings = settings;
    contextLimits.before = Math.max(200, settings.context.beforeChars);
    contextLimits.after = Math.max(100, settings.context.afterChars);
    autoTranslate = settings.autoTranslate;

    if (autoToggle) {
      ignoreAutoToggleChange = true;
      autoToggle.checked = autoTranslate;
      ignoreAutoToggleChange = false;
    }

    if (languageSelect) {
      languageSelect.value = settings.targetLanguage;
    }
  },
  onError: message => showToast(message),
}).catch(error => {
  showToast(`Failed to load settings: ${error instanceof Error ? error.message : String(error)}`);
});

const countSnippetLines = (snippet: string): number => {
  if (!snippet.length) {
    return 0;
  }
  const lines = snippet.split('\n');
  if (lines.length === 0) {
    return 0;
  }
  if (lines.length === 1 && lines[0] === '') {
    return 0;
  }
  if (lines[lines.length - 1] === '') {
    return lines.length - 1;
  }
  return lines.length;
};

