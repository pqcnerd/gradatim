import * as monaco from 'monaco-editor';
import './style.css';
import { translateLine } from './api/translatorClient';
import { initSettingsUI } from './settings';
import type { DirectoryNode, EditorSettings, TranslateLinePayload } from './types/electron';

type LineTrigger = 'enter' | 'shortcut' | 'regenerate';

const editorContainer = document.getElementById('editor');
const statusList = document.getElementById('status-log');
const autoToggle = document.getElementById('auto-toggle') as HTMLInputElement | null;
const languageSelect = document.getElementById('language-select') as HTMLSelectElement | null;
const regenerateButton = document.getElementById('regenerate-btn') as HTMLButtonElement | null;
const toast = document.getElementById('toast');
const fileTreeContainer = document.getElementById('file-tree');
const tabList = document.getElementById('tab-list') as HTMLDivElement | null;
const tabAddButton = document.getElementById('tab-add') as HTMLButtonElement | null;
const workspaceLabel = document.getElementById('workspace-label');
const sidebarRefreshButton = document.getElementById('sidebar-refresh');
const sidebarElement = document.querySelector('.sidebar') as HTMLElement | null;
const workspaceContainer = document.querySelector('.workspace') as HTMLElement | null;
const statusPanelElement = document.querySelector('.status-panel') as HTMLElement | null;

if (!editorContainer || !statusList || !toast || !fileTreeContainer || !tabList) {
  throw new Error('Renderer root elements are missing. Check index.html structure.');
}

const welcomeSnippet = [
  '// Welcome to Gradatim',
  '// Type an English instruction and press Enter.',
  '',
  'declare a, b 0',
  'set a to 5',
  'if a is greater than b'
].join('\n');

type EditorTab = {
  id: string;
  title: string;
  path?: string | null;
  absolutePath?: string;
  model: monaco.editor.ITextModel;
  dirty: boolean;
  language: string;
};

const tabs: EditorTab[] = [];
let activeTabId: string | null = null;
let untitledCounter = 1;
let workspaceRootPath: string | null = null;
let workspaceRootName: string | null = null;
let disposeMenuListener: (() => void) | undefined;
let sidebarVisible = true;
let activityVisible = true;
let minimapEnabled = true;
let zoomLevel = 0;

const editor = monaco.editor.create(editorContainer, {
  language: 'c',
  value: '',
  automaticLayout: true,
  minimap: { enabled: true, renderCharacters: false, showSlider: 'always' },
  fontSize: 16,
  fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
  cursorSmoothCaretAnimation: 'on',
  renderWhitespace: 'trailing',
  scrollBeyondLastLine: false,
  theme: 'vs-dark',
  quickSuggestions: false,
  suggestOnTriggerCharacters: false,
  wordBasedSuggestions: 'off',
  acceptSuggestionOnEnter: 'off',
  acceptSuggestionOnCommitCharacter: false
});

const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  '.c': 'c',
  '.h': 'c',
  '.cpp': 'cpp',
  '.hpp': 'cpp',
  '.py': 'python',
  '.js': 'javascript',
  '.ts': 'typescript'
};

const languageFromSetting = (setting?: string) =>
  setting && setting.toLowerCase() === 'python' ? 'python' : 'c';

const inferLanguageFromPath = (filePath?: string) => {
  if (!filePath) {
    return 'c';
  }
  const lower = filePath.toLowerCase();
  const dot = lower.lastIndexOf('.');
  if (dot === -1) {
    return 'c';
  }
  const ext = lower.slice(dot);
  return LANGUAGE_BY_EXTENSION[ext] ?? 'c';
};

const generateTabId = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `tab-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const basename = (input?: string | null) => {
  if (!input) {
    return '';
  }
  const normalized = input.replace(/\\/g, '/');
  const segments = normalized.split('/');
  const last = segments[segments.length - 1];
  return last || normalized;
};

const updateWorkspaceInfo = (info: { rootPath?: string | null; rootName?: string | null }) => {
  if (info.rootPath) {
    workspaceRootPath = info.rootPath;
  }
  if (info.rootName) {
    workspaceRootName = info.rootName;
    if (workspaceLabel) {
      workspaceLabel.textContent = info.rootName;
    }
  }
};

const applyWorkspaceSnapshot = (payload: { rootPath?: string | null; rootName?: string | null; snapshot?: DirectoryNode | null }) => {
  updateWorkspaceInfo({ rootPath: payload.rootPath ?? workspaceRootPath, rootName: payload.rootName ?? workspaceRootName });
  if (payload.snapshot) {
    renderFileTree(payload.snapshot);
  } else if (!payload.snapshot && fileTreeContainer) {
    fileTreeContainer.innerHTML = '<p class="tree-placeholder">No files detected in this workspace.</p>';
  }
};

const findTabByPath = (absolutePath?: string | null, relativePath?: string | null) => {
  if (!absolutePath && !relativePath) {
    return undefined;
  }
  return tabs.find(tab => {
    if (absolutePath && tab.absolutePath === absolutePath) {
      return true;
    }
    if (relativePath && tab.path === relativePath) {
      return true;
    }
    return false;
  });
};

const clampZoomLevel = (value: number) => Math.max(-3, Math.min(3, value));

const persistUiPreferences = () => {
  if (!window.electronAPI?.saveSettings) {
    return;
  }
  window.electronAPI
    .saveSettings({
      ui: {
        sidebarVisible,
        activityVisible,
        minimapEnabled,
        zoomLevel
      }
    })
    .catch(() => {});
};

const applySidebarVisibility = (visible: boolean, persist = false) => {
  sidebarVisible = visible;
  sidebarElement?.classList.toggle('is-hidden', !visible);
  workspaceContainer?.classList.toggle('sidebar-hidden', !visible);
  if (persist) {
    persistUiPreferences();
  }
};

const applyActivityVisibility = (visible: boolean, persist = false) => {
  activityVisible = visible;
  statusPanelElement?.classList.toggle('is-hidden', !visible);
  workspaceContainer?.classList.toggle('activity-hidden', !visible);
  if (persist) {
    persistUiPreferences();
  }
};

const applyMinimapPreference = (enabled: boolean, persist = false) => {
  minimapEnabled = enabled;
  editor.updateOptions({
    minimap: { enabled, renderCharacters: false, showSlider: 'always' }
  });
  if (persist) {
    persistUiPreferences();
  }
};

const applyZoomLevel = async (value: number, persist = false) => {
  zoomLevel = clampZoomLevel(value);
  if (window.electronAPI?.setZoomLevel) {
    try {
      const applied = await window.electronAPI.setZoomLevel(zoomLevel);
      if (typeof applied === 'number' && !Number.isNaN(applied)) {
        zoomLevel = clampZoomLevel(applied);
      }
    } catch (error) {
      showToast(
        `Zoom change failed: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }
  if (persist) {
    persistUiPreferences();
  }
};

const getActiveTab = () => (activeTabId ? tabs.find(tab => tab.id === activeTabId) ?? null : null);

const handleMenuCommand = (command: string) => {
  switch (command) {
    case 'file:new':
      createUntitledTab();
      break;
    case 'file:open':
      void openFileViaDialog();
      break;
    case 'file:openFolder':
      void openFolderViaDialog();
      break;
    case 'file:save':
      void saveActiveTab(false);
      break;
    case 'file:saveAs':
      void saveActiveTab(true);
      break;
    case 'file:closeTab':
      if (activeTabId) {
        closeTab(activeTabId);
      }
      break;
    case 'file:closeOthers':
      closeOtherTabs();
      break;
    case 'file:closeAll':
      closeAllTabs();
      break;
    case 'edit:undo':
      editor.trigger('menu', 'undo', undefined);
      break;
    case 'edit:redo':
      editor.trigger('menu', 'redo', undefined);
      break;
    case 'edit:cut':
      editor.trigger('menu', 'editor.action.clipboardCutAction', undefined);
      break;
    case 'edit:copy':
      editor.trigger('menu', 'editor.action.clipboardCopyAction', undefined);
      break;
    case 'edit:paste':
      editor.trigger('menu', 'editor.action.clipboardPasteAction', undefined);
      break;
    case 'edit:selectAll':
      editor.trigger('menu', 'editor.action.selectAll', undefined);
      break;
    case 'edit:find':
      editor.trigger('menu', 'actions.find', undefined);
      break;
    case 'edit:replace':
      editor.trigger('menu', 'editor.action.startFindReplaceAction', undefined);
      break;
    case 'edit:goToLine': {
      const input = window.prompt('Go to line number:');
      if (!input) {
        break;
      }
      const line = Number.parseInt(input, 10);
      if (Number.isNaN(line) || line <= 0) {
        showToast('Invalid line number.');
        break;
      }
      editor.revealLineInCenter(line);
      editor.setPosition({ lineNumber: line, column: 1 });
      editor.focus();
      break;
    }
    case 'view:toggleSidebar':
      applySidebarVisibility(!sidebarVisible, true);
      break;
    case 'view:toggleActivity':
      applyActivityVisibility(!activityVisible, true);
      break;
    case 'view:zoomIn':
      void applyZoomLevel(zoomLevel + 0.5, true);
      break;
    case 'view:zoomOut':
      void applyZoomLevel(zoomLevel - 0.5, true);
      break;
    case 'view:zoomReset':
      void applyZoomLevel(0, true);
      break;
    case 'view:toggleMinimap':
      applyMinimapPreference(!minimapEnabled, true);
      break;
    default:
      break;
  }
};

const renderTabs = () => {
  if (!tabList) {
    return;
  }
  tabList.innerHTML = '';
  tabs.forEach(tab => {
    const tabElement = document.createElement('div');
    tabElement.className = `tab${tab.id === activeTabId ? ' active' : ''}`;
    const title = document.createElement('span');
    title.className = 'tab-title';
    title.textContent = tab.dirty ? `${tab.title} •` : tab.title;
    tabElement.appendChild(title);

    const closeButton = document.createElement('button');
    closeButton.type = 'button';
    closeButton.className = 'tab-close';
    closeButton.textContent = '×';
    closeButton.addEventListener('click', event => {
      event.stopPropagation();
      closeTab(tab.id);
    });

    tabElement.addEventListener('click', () => setActiveTab(tab.id));
    tabElement.appendChild(closeButton);
    tabList.appendChild(tabElement);
  });
};

const setActiveTab = (tabId: string) => {
  const nextTab = tabs.find(tab => tab.id === tabId);
  if (!nextTab) {
    return;
  }
  activeTabId = tabId;
  editor.setModel(nextTab.model);
  monaco.editor.setModelLanguage(nextTab.model, nextTab.language);
  renderTabs();
  editor.focus();
};

const closeTab = (tabId: string) => {
  const index = tabs.findIndex(tab => tab.id === tabId);
  if (index === -1) {
    return;
  }
  const [removed] = tabs.splice(index, 1);
  clearPromptsForTab(removed.id);
  removed.model.dispose();

  if (activeTabId === tabId) {
    if (tabs.length === 0) {
      createUntitledTab();
    } else {
      const fallbackIndex = index === 0 ? 0 : index - 1;
      setActiveTab(tabs[fallbackIndex].id);
    }
  } else {
    renderTabs();
  }
};

const createUntitledTab = (initialValue = '', language?: string) => {
  const lang = language ?? languageFromSetting(currentSettings?.targetLanguage);
  const extension = lang === 'python' ? 'py' : 'c';
  const title = `untitled-${untitledCounter++}.${extension}`;
  const model = monaco.editor.createModel(initialValue, lang);
  const newTab: EditorTab = {
    id: generateTabId(),
    title,
    path: undefined,
    model,
    dirty: false,
    language: lang
  };
  tabs.push(newTab);
  setActiveTab(newTab.id);
};

const renderFileTree = (root: DirectoryNode | null) => {
  if (!fileTreeContainer) {
    return;
  }
  if (!root || !(root.children?.length)) {
    fileTreeContainer.innerHTML =
      '<p class="tree-placeholder">No files detected in this workspace.</p>';
    return;
  }
  const list = document.createElement('ul');
  list.className = 'tree-children';
  root.children.forEach(child => list.appendChild(buildTreeNode(child)));
  fileTreeContainer.innerHTML = '';
  fileTreeContainer.appendChild(list);
};

const buildTreeNode = (node: DirectoryNode): HTMLLIElement => {
  const item = document.createElement('li');
  item.className = 'tree-node';
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'tree-title';
  const glyph = document.createElement('span');
  glyph.className = 'tree-glyph';
  glyph.textContent = node.type === 'folder' ? '▾' : '•';
  button.appendChild(glyph);
  const label = document.createElement('span');
  label.textContent = node.name;
  button.appendChild(label);
  item.appendChild(button);

  if (node.type === 'folder') {
    const children = document.createElement('ul');
    children.className = 'tree-children';
    (node.children ?? []).forEach(child => children.appendChild(buildTreeNode(child)));
    item.appendChild(children);
    button.addEventListener('click', () => {
      const collapsed = item.classList.toggle('collapsed');
      glyph.textContent = collapsed ? '▸' : '▾';
      children.hidden = collapsed;
    });
  } else {
    button.addEventListener('click', () => {
      void openFileInTab(node);
    });
  }

  return item;
};

const loadWorkspaceTree = async (override?: { rootPath: string; rootName?: string; snapshot: DirectoryNode | null }) => {
  if (!fileTreeContainer) {
    return;
  }
  if (override) {
    applyWorkspaceSnapshot(override);
    return;
  }
  if (!window.electronAPI?.listDirectory) {
    fileTreeContainer.innerHTML =
      '<p class="tree-placeholder">File system bridge unavailable in this build.</p>';
    return;
  }
  fileTreeContainer.innerHTML = '<p class="tree-placeholder">Loading workspace…</p>';
  try {
    const payload = await window.electronAPI.listDirectory();
    if (!payload) {
      return;
    }
    applyWorkspaceSnapshot({
      rootPath: payload.rootPath ?? workspaceRootPath,
      rootName: payload.rootName ?? workspaceRootName,
      snapshot: payload.snapshot ?? null
    });
  } catch (error) {
    fileTreeContainer.innerHTML = '<p class="tree-placeholder">Failed to load workspace.</p>';
    showToast(
      `File tree error: ${error instanceof Error ? error.message : String(error)}`
    );
  }
};

const openFileInTab = async (node: DirectoryNode) => {
  if (!window.electronAPI?.readFile) {
    showToast('File open not supported in this build.');
    return;
  }
  const existing = tabs.find(tab => tab.path === node.path);
  if (existing) {
    setActiveTab(existing.id);
    return;
  }
  try {
    const file = await window.electronAPI.readFile(node.path);
    const relativePath = file.relativePath ?? node.path;
    const absolutePath = file.absolutePath;
    const language = inferLanguageFromPath(absolutePath ?? relativePath ?? node.name);
    const existing = findTabByPath(absolutePath, relativePath);
    if (existing) {
      existing.model.setValue(file.content);
      existing.dirty = false;
      renderTabs();
      setActiveTab(existing.id);
      pushStatus(`[${existing.title}] Reloaded from disk.`);
      return;
    }
    const model = monaco.editor.createModel(file.content, language);
    const title = node.name || basename(relativePath ?? absolutePath ?? 'file');
    const newTab: EditorTab = {
      id: generateTabId(),
      title,
      path: relativePath ?? absolutePath ?? title,
      absolutePath: absolutePath ?? undefined,
      model,
      dirty: false,
      language
    };
    tabs.push(newTab);
    setActiveTab(newTab.id);
  } catch (error) {
    showToast(`Failed to open ${node.name}: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const openFileViaDialog = async () => {
  try {
    const result = await window.electronAPI?.openFileDialog?.();
    if (!result || result.canceled || !result.file) {
      return;
    }
    const { file } = result;
    const language = inferLanguageFromPath(file.absolutePath ?? file.relativePath ?? file.name);
    const existing = findTabByPath(file.absolutePath, file.relativePath ?? undefined);
    if (existing) {
      existing.model.setValue(file.content);
      existing.dirty = false;
      renderTabs();
      setActiveTab(existing.id);
      pushStatus(`[${existing.title}] Reloaded from disk.`);
      return;
    }
    const model = monaco.editor.createModel(file.content, language);
    const title = file.name || basename(file.relativePath ?? file.absolutePath ?? 'file');
    const newTab: EditorTab = {
      id: generateTabId(),
      title,
      path: file.relativePath ?? file.absolutePath ?? title,
      absolutePath: file.absolutePath,
      model,
      dirty: false,
      language
    };
    tabs.push(newTab);
    setActiveTab(newTab.id);
    pushStatus(`Opened ${title}`);
  } catch (error) {
    showToast(`Open failed: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const openFolderViaDialog = async () => {
  try {
    const result = await window.electronAPI?.openFolderDialog?.();
    if (!result || result.canceled || !result.snapshot) {
      return;
    }
    await loadWorkspaceTree({
      rootPath: result.rootPath ?? workspaceRootPath ?? '',
      rootName: result.rootName,
      snapshot: result.snapshot
    });
    pushStatus(`Workspace changed to ${result.rootName ?? result.rootPath}`);
  } catch (error) {
    showToast(`Open folder failed: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const saveActiveTab = async (forceSaveAs = false) => {
  const tab = getActiveTab();
  if (!tab) {
    return;
  }
  if (!window.electronAPI?.writeFile || !window.electronAPI.saveFileAs) {
    showToast('Saving is not supported in this build.');
    return;
  }
  const content = tab.model.getValue();
  try {
    if (!tab.absolutePath || forceSaveAs || !tab.path) {
      const result = await window.electronAPI.saveFileAs({
        defaultPath: tab.absolutePath ?? workspaceRootPath ?? undefined,
        suggestedName: tab.title,
        content
      });
      if (result.canceled) {
        return;
      }
      tab.absolutePath = result.absolutePath ?? tab.absolutePath;
      tab.path = result.relativePath ?? tab.absolutePath ?? tab.path;
      if (result.name) {
        tab.title = result.name;
      } else if (tab.path) {
        tab.title = basename(tab.path);
      }
      tab.dirty = false;
      renderTabs();
      pushStatus(`Saved ${tab.title}`);
      await loadWorkspaceTree();
    } else {
      await window.electronAPI.writeFile({
        absolutePath: tab.absolutePath,
        relativePath: tab.path ?? undefined,
        content
      });
      tab.dirty = false;
      renderTabs();
      pushStatus(`Saved ${tab.title}`);
    }
  } catch (error) {
    showToast(`Save failed: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const closeOtherTabs = () => {
  const active = getActiveTab();
  if (!active) {
    return;
  }
  tabs.slice().forEach(tab => {
    if (tab.id !== active.id) {
      clearPromptsForTab(tab.id);
      tab.model.dispose();
    }
  });
  const survivors = tabs.filter(tab => tab.id === active.id);
  tabs.length = 0;
  tabs.push(...survivors);
  activeTabId = active.id;
  renderTabs();
};

const closeAllTabs = () => {
  while (tabs.length) {
    const tab = tabs.pop();
    if (tab) {
      clearPromptsForTab(tab.id);
      tab.model.dispose();
    }
  }
  activeTabId = null;
  createUntitledTab();
};

tabAddButton?.addEventListener('click', () => createUntitledTab());
sidebarRefreshButton?.addEventListener('click', () => {
  void loadWorkspaceTree();
});

disposeMenuListener = window.electronAPI?.onMenuCommand?.(handleMenuCommand) ?? disposeMenuListener;
window.addEventListener('beforeunload', () => {
  disposeMenuListener?.();
});

let autoTranslate = true;
let lastDecorations: string[] = [];
let requestCounter = 0;
const pendingRequests = new Map<string, number>();
let lastTriggeredLine: number | null = null;
let lastTriggeredTabId: string | null = null;
let toastTimeout: ReturnType<typeof setTimeout> | undefined;
let ignoreAutoToggleChange = true;
const contextLimits = { before: 1600, after: 900 };
let currentSettings: EditorSettings | null = null;
const defaultMaxLines = 3;
const STATUS_LOG_LIMIT = 7;

type PendingPromptContext = {
  key: string;
  tabId: string;
  lineNumber: number;
  englishLine: string;
  placeholder: string;
  element: HTMLLIElement;
};

const pendingPrompts = new Map<string, PendingPromptContext>();

const makePromptKey = (tabId: string, lineNumber: number) => `${tabId}:${lineNumber}`;

autoToggle?.addEventListener('change', (event: Event) => {
  if (ignoreAutoToggleChange) {
    return;
  }

  autoTranslate = (event.target as HTMLInputElement).checked;
  pushStatus(`Auto translate ${autoTranslate ? 'enabled' : 'paused'}.`);
});

regenerateButton?.addEventListener('click', () => {
  if (lastTriggeredLine == null || !lastTriggeredTabId) {
    showToast('Translate a line first to enable regeneration.');
    return;
  }
  if (lastTriggeredTabId !== activeTabId) {
    setActiveTab(lastTriggeredTabId);
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
  li.className = 'status-entry';
  li.textContent = `${new Date().toLocaleTimeString()} — ${message}`;
  statusList.prepend(li);
  trimStatusLog();
};

const trimStatusLog = () => {
  const children = Array.from(statusList.children);
  if (children.length <= STATUS_LOG_LIMIT) {
    return;
  }

  let removed = 0;
  for (let idx = children.length - 1; idx >= 0 && children.length - removed > STATUS_LOG_LIMIT; idx -= 1) {
    const entry = children[idx] as HTMLElement;
    if (entry.classList.contains('status-entry--prompt')) {
      continue;
    }
    statusList.removeChild(entry);
    removed += 1;
  }
};

const resolvePrompt = (tabId: string, lineNumber: number) => {
  const key = makePromptKey(tabId, lineNumber);
  const context = pendingPrompts.get(key);
  if (!context) {
    return;
  }

  pendingPrompts.delete(key);
  context.element.remove();
};

const clearPromptsForTab = (tabId: string) => {
  Array.from(pendingPrompts.values())
    .filter(context => context.tabId === tabId)
    .forEach(context => {
      pendingPrompts.delete(context.key);
      context.element.remove();
    });
};

const buildTodoPlaceholder = (englishLine: string) => {
  const trimmed = englishLine.trim();
  const lang = (currentSettings?.targetLanguage ?? 'c').toLowerCase();
  if (lang === 'python') {
    return `# TODO: ${trimmed}`;
  }
  return `// TODO: ${trimmed}`;
};

const showUnrecognizedPrompt = (
  tab: EditorTab,
  lineNumber: number,
  englishLine: string,
  placeholder: string,
  message?: string
) => {
  const key = makePromptKey(tab.id, lineNumber);
  resolvePrompt(tab.id, lineNumber);

  const li = document.createElement('li');
  li.className = 'status-entry status-entry--prompt';

  const title = document.createElement('div');
  title.className = 'prompt-title';
  title.textContent = `[${tab.title}] Line ${lineNumber} needs guidance`;

  const description = document.createElement('div');
  description.className = 'prompt-description';
  description.textContent =
    message ?? 'Neither the AI nor the rule-based translator could interpret this instruction.';

  const instruction = document.createElement('div');
  instruction.className = 'prompt-instruction';
  instruction.textContent = `“${englishLine}”`;

  const actions = document.createElement('div');
  actions.className = 'prompt-actions';

  const manualButton = document.createElement('button');
  manualButton.type = 'button';
  manualButton.textContent = 'Enter code';

  const retryButton = document.createElement('button');
  retryButton.type = 'button';
  retryButton.textContent = 'Retry with AI';

  const skipButton = document.createElement('button');
  skipButton.type = 'button';
  skipButton.textContent = 'Skip / TODO';

  actions.append(manualButton, retryButton, skipButton);
  li.append(title, description, instruction, actions);
  statusList.prepend(li);

  const context: PendingPromptContext = {
    key,
    tabId: tab.id,
    lineNumber,
    englishLine,
    placeholder,
    element: li
  };

  pendingPrompts.set(key, context);
  trimStatusLog();

  manualButton.addEventListener('click', () => {
    renderManualInput(context);
  });

  retryButton.addEventListener('click', () => {
    resolvePrompt(context.tabId, context.lineNumber);
    setActiveTab(context.tabId);
    enqueueTranslation(context.lineNumber, 'regenerate');
  });

  skipButton.addEventListener('click', () => {
    const snippet =
      context.placeholder && context.placeholder.trim().length > 0
        ? context.placeholder
        : buildTodoPlaceholder(context.englishLine);
    applyTranslation(context.tabId, context.lineNumber, snippet);
    pushStatus(
      `[${tab.title}] Line ${context.lineNumber} left as TODO.`
    );
    resolvePrompt(context.tabId, context.lineNumber);
  });
};

const renderManualInput = (context: PendingPromptContext) => {
  const existing = context.element.querySelector('.prompt-manual');
  if (existing) {
    const textarea = existing.querySelector('textarea') as HTMLTextAreaElement | null;
    textarea?.focus();
    return;
  }

  const manual = document.createElement('div');
  manual.className = 'prompt-manual';
  const textarea = document.createElement('textarea');
  textarea.rows = 4;
  textarea.placeholder = 'Type the code to insert…';
  manual.appendChild(textarea);

  const manualActions = document.createElement('div');
  manualActions.className = 'prompt-actions prompt-actions--inline';

  const insertButton = document.createElement('button');
  insertButton.type = 'button';
  insertButton.textContent = 'Insert code';

  const cancelButton = document.createElement('button');
  cancelButton.type = 'button';
  cancelButton.textContent = 'Cancel';

  manualActions.append(insertButton, cancelButton);
  manual.appendChild(manualActions);
  context.element.appendChild(manual);
  textarea.focus();

  insertButton.addEventListener('click', () => {
    const snippet = textarea.value.trim();
    if (!snippet) {
      textarea.focus();
      return;
    }
    applyTranslation(context.tabId, context.lineNumber, snippet);
    const tabTitle = tabs.find(tab => tab.id === context.tabId)?.title ?? 'Untitled';
    pushStatus(`[${tabTitle}] Manual code inserted for line ${context.lineNumber}.`);
    resolvePrompt(context.tabId, context.lineNumber);
  });

  cancelButton.addEventListener('click', () => {
    manual.remove();
  });
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

createUntitledTab(welcomeSnippet);
loadWorkspaceTree();

const highlightLine = (
  tabId: string,
  lineNumber: number,
  variant: 'pending' | 'error' | 'success' = 'pending'
) => {
  if (tabId !== activeTabId) {
    return;
  }
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

const collectContext = (model: monaco.editor.ITextModel, lineNumber: number) => {
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

const applyTranslation = (tabId: string, lineNumber: number, rawSnippet: string) => {
  const targetTab = tabs.find(tab => tab.id === tabId);
  if (!targetTab) {
    return;
  }

  const model = targetTab.model;
  const snippet = ensureStructuralCompleteness(rawSnippet, currentSettings?.targetLanguage);
  const original = model.getLineContent(lineNumber);
  const indent = original.match(/^\s*/)?.[0] ?? '';
  const indentedSnippet = snippet
    .split('\n')
    .map(part => `${indent}${part}`)
    .join('\n');

  const range = new monaco.Range(lineNumber, 1, lineNumber, original.length + 1);
  model.pushEditOperations([], [{ range, text: indentedSnippet }], () => null);
  targetTab.dirty = true;
  renderTabs();
  highlightLine(tabId, lineNumber, 'success');
};

const buildPayload = (
  model: monaco.editor.ITextModel,
  lineNumber: number,
  englishLine: string
): TranslateLinePayload => {
  const { code_before, code_after } = collectContext(model, lineNumber);
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
  const targetTab = getActiveTab();
  if (!targetTab) {
    return;
  }

  const model = targetTab.model;
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

  const englishInstruction = rawLine.trim();
  const payload = buildPayload(model, lineNumber, englishInstruction);
  const requestId = ++requestCounter;
  const requestKey = `${targetTab.id}:${lineNumber}`;
  pendingRequests.set(requestKey, requestId);
  lastTriggeredLine = lineNumber;
  lastTriggeredTabId = targetTab.id;

  highlightLine(targetTab.id, lineNumber);
  pushStatus(`[${targetTab.title}] Queued line ${lineNumber} via ${trigger}.`);

  const result = await translateLine(payload);

  if (pendingRequests.get(requestKey) !== requestId) {
    pendingRequests.delete(requestKey);
    return;
  }

  pendingRequests.delete(requestKey);

  if (result.kind === 'ok') {
    resolvePrompt(targetTab.id, lineNumber);
    applyTranslation(targetTab.id, lineNumber, result.code);
    const producedLines = Math.max(1, countSnippetLines(result.code));
    const langLabel = (currentSettings?.targetLanguage ?? 'c').toUpperCase();
    pushStatus(
      `[${targetTab.title}] Translated line ${lineNumber} → ${producedLines} line${
        producedLines === 1 ? '' : 's'
      } (${langLabel}).`
    );
  } else if (result.kind === 'unhandled') {
    highlightLine(targetTab.id, lineNumber, 'error');
    showUnrecognizedPrompt(targetTab, lineNumber, englishInstruction, result.placeholder, result.message);
    pushStatus(
      `[${targetTab.title}] Line ${lineNumber} needs guidance — choose an action in the activity panel.`
    );
  } else {
    highlightLine(targetTab.id, lineNumber, 'error');
    pushStatus(`[${targetTab.title}] Line ${lineNumber} failed: ${result.message}`);
    showToast(result.message);
  }
};

editor.onDidChangeModelContent((event: monaco.editor.IModelContentChangedEvent) => {
  const activeTab = getActiveTab();
  if (activeTab && !activeTab.dirty) {
    activeTab.dirty = true;
    renderTabs();
  }

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

    const uiPrefs = settings.ui ?? {};
    applySidebarVisibility(uiPrefs.sidebarVisible ?? true);
    applyActivityVisibility(uiPrefs.activityVisible ?? true);
    applyMinimapPreference(uiPrefs.minimapEnabled ?? true);
    void applyZoomLevel(
      typeof uiPrefs.zoomLevel === 'number' ? uiPrefs.zoomLevel : 0,
      false
    );

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

