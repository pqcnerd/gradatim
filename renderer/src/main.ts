import * as monaco from 'monaco-editor';
import './style.css';
import './terminal/terminal.css';
import { translateLine } from './api/translatorClient';
import { initSettingsUI } from './settings';
import { TerminalPanel } from './terminal';
import type { DirectoryNode, EditorSettings, TranslateLinePayload } from './types/electron';

type LineTrigger = 'enter' | 'shortcut' | 'regenerate';
type UiTheme = 'light' | 'dark' | 'glass';

const editorContainer = document.getElementById('editor');
const statusList = document.getElementById('status-log');
const autoToggle = document.getElementById('auto-toggle') as HTMLInputElement | null;
const languageDropdown = document.getElementById('language-dropdown') as HTMLDivElement | null;
const languageTrigger = document.getElementById('language-trigger') as HTMLButtonElement | null;
const languageMenu = document.getElementById('language-menu') as HTMLDivElement | null;
const languageTriggerValue = document.getElementById('language-trigger-value');
const languageOptions = Array.from(document.querySelectorAll('.language-option')) as HTMLButtonElement[];
const regenerateButton = document.getElementById('regenerate-btn') as HTMLButtonElement | null;
const themeCycleButton = document.getElementById('theme-cycle') as HTMLButtonElement | null;
const toast = document.getElementById('toast');
const fileTreeContainer = document.getElementById('file-tree');
const tabList = document.getElementById('tab-list') as HTMLDivElement | null;
const tabAddButton = document.getElementById('tab-add') as HTMLButtonElement | null;
const workspaceLabel = document.getElementById('workspace-label');
const sidebarRefreshButton = document.getElementById('sidebar-refresh');
const workspaceContainer = document.querySelector('.workspace') as HTMLElement | null;
const activitySectionElement = document.querySelector('.activity-section') as HTMLElement | null;
const filesSectionElement = document.querySelector('.files-section') as HTMLElement | null;
const terminalPanelContainer = document.getElementById('terminal-panel');
const menuGroups = Array.from(document.querySelectorAll('.menu-group')) as HTMLDivElement[];
const menuCommands = Array.from(document.querySelectorAll('.menu-command')) as HTMLButtonElement[];
const menuItems = Array.from(document.querySelectorAll('.menu-item')) as HTMLButtonElement[];
const menuDropdowns = Array.from(document.querySelectorAll('.menu-dropdown')) as HTMLDivElement[];
const windowMinimizeButton = document.getElementById('window-minimize') as HTMLButtonElement | null;
const windowMaximizeButton = document.getElementById('window-maximize') as HTMLButtonElement | null;
const windowCloseButton = document.getElementById('window-close') as HTMLButtonElement | null;

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
  translatedLines: Set<number>;
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
let wordWrapEnabled = false;
let lineNumbersEnabled = true;
let zoomLevel = 0;
let currentTheme: UiTheme = 'light';
let terminalPanel: TerminalPanel | null = null;
let activeTopMenuKey: string | null = null;

monaco.editor.defineTheme('gradatim-glass', {
  base: 'vs-dark',
  inherit: true,
  rules: [],
  colors: {
    'editor.background': '#05091399',
    'editor.foreground': '#E4EDFF',
    'editorLineNumber.foreground': '#7283A9',
    'editorLineNumber.activeForeground': '#BFD2FA',
    'editorCursor.foreground': '#60A5FA',
    'editor.selectionBackground': '#60A5FA44',
    'editor.inactiveSelectionBackground': '#60A5FA22',
    'editor.lineHighlightBackground': '#60A5FA12',
    'editorIndentGuide.background1': '#8AA1CC22',
    'editorIndentGuide.activeBackground1': '#AFC2EA55'
  }
});

monaco.editor.defineTheme('gradatim-light', {
  base: 'vs',
  inherit: true,
  rules: [],
  colors: {
    'editor.background': '#FFFFFFA6',
    'editor.foreground': '#1D2B4F',
    'editorLineNumber.foreground': '#7A88A8',
    'editorLineNumber.activeForeground': '#415A97',
    'editorCursor.foreground': '#3265D7',
    'editor.selectionBackground': '#3265D744',
    'editor.inactiveSelectionBackground': '#3265D722',
    'editor.lineHighlightBackground': '#3265D714',
    'editorIndentGuide.background1': '#7B93C033',
    'editorIndentGuide.activeBackground1': '#5972B766'
  }
});

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
  theme: 'gradatim-glass',
  quickSuggestions: false,
  suggestOnTriggerCharacters: false,
  wordBasedSuggestions: 'off',
  acceptSuggestionOnEnter: 'off',
  acceptSuggestionOnCommitCharacter: false,
  // Enable auto-indentation so pressing Enter maintains indent level
  autoIndent: 'full',
  // Format on paste to maintain consistency
  formatOnPaste: true,
  // Tab settings
  tabSize: 4,
  insertSpaces: true,
  detectIndentation: false,
  bracketPairColorization: { enabled: true },
  smoothScrolling: true,
  stickyScroll: { enabled: true },
  linkedEditing: true,
  wordWrap: 'off',
  lineNumbers: 'on'
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

// Initialize terminal panel
if (terminalPanelContainer) {
  terminalPanel = new TerminalPanel(terminalPanelContainer);
  // Start visible by default and open a session right away.
  terminalPanel.show();
  
  // Listen for terminal resize events to adjust editor layout
  window.addEventListener('terminal-panel-resize', () => {
    editor.layout();
  });
}

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
    // Update terminal panel workspace path
    if (terminalPanel) {
      terminalPanel.workspacePath = info.rootPath;
    }
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
        zoomLevel,
        theme: currentTheme
      }
    })
    .catch(() => {});
};

const applySidebarVisibility = (visible: boolean, persist = false) => {
  sidebarVisible = visible;
  filesSectionElement?.classList.toggle('is-hidden', !visible);
  workspaceContainer?.classList.toggle('sidebar-hidden', !visible);
  if (persist) {
    persistUiPreferences();
  }
};

const applyActivityVisibility = (visible: boolean, persist = false) => {
  activityVisible = visible;
  activitySectionElement?.classList.toggle('is-hidden', !visible);
  workspaceContainer?.classList.toggle('activity-hidden', !visible);
  if (persist) {
    persistUiPreferences();
  }
};

const normalizeTheme = (theme?: string): UiTheme => {
  if (theme === 'dark' || theme === 'glass') {
    return theme;
  }
  return 'light';
};

const applyTheme = (theme: UiTheme, persist = false) => {
  currentTheme = normalizeTheme(theme);
  document.body.dataset.theme = currentTheme;
  monaco.editor.setTheme(currentTheme === 'light' ? 'gradatim-light' : 'gradatim-glass');
  terminalPanel?.setTheme(currentTheme);
  if (themeCycleButton) {
    const label = currentTheme[0].toUpperCase() + currentTheme.slice(1);
    themeCycleButton.textContent = label;
    themeCycleButton.title = `Theme: ${label}`;
  }
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

const applyWordWrapPreference = (enabled: boolean) => {
  wordWrapEnabled = enabled;
  editor.updateOptions({
    wordWrap: enabled ? 'on' : 'off'
  });
};

const applyLineNumbersPreference = (enabled: boolean) => {
  lineNumbersEnabled = enabled;
  editor.updateOptions({
    lineNumbers: enabled ? 'on' : 'off'
  });
};

const applyEditorPreferences = (prefs?: EditorSettings['editor']) => {
  if (!prefs) {
    return;
  }
  editor.updateOptions({
    tabSize: prefs.tabSize ?? 4,
    insertSpaces: prefs.insertSpaces ?? true,
    fontSize: prefs.fontSize ?? 16,
    fontFamily: prefs.fontFamily ?? "'JetBrains Mono', 'Fira Code', monospace",
    renderWhitespace: prefs.renderWhitespace ?? 'trailing',
    cursorStyle: prefs.cursorStyle ?? 'line'
  });
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
    case 'view:toggleWordWrap':
      applyWordWrapPreference(!wordWrapEnabled);
      pushStatus(`Word wrap ${wordWrapEnabled ? 'enabled' : 'disabled'}.`);
      break;
    case 'view:toggleLineNumbers':
      applyLineNumbersPreference(!lineNumbersEnabled);
      pushStatus(`Line numbers ${lineNumbersEnabled ? 'shown' : 'hidden'}.`);
      break;
    case 'view:toggleTerminal':
      terminalPanel?.toggle();
      break;
    case 'terminal:new':
      if (terminalPanel) {
        if (!terminalPanel.visible) {
          terminalPanel.show();
        } else {
          terminalPanel.createTerminal();
        }
      }
      break;
    case 'terminal:runFile':
      void runCurrentFile();
      break;
    case 'terminal:buildFile':
      void buildCurrentFile();
      break;
    case 'terminal:close':
      terminalPanel?.hide();
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
  refreshUntranslatedDecorations();
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
    language: lang,
    translatedLines: new Set<number>()
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
      existing.translatedLines.clear();
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
      language,
      translatedLines: new Set<number>()
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
      existing.translatedLines.clear();
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
      language,
      translatedLines: new Set<number>()
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
let transientHighlightDecorations: string[] = [];
let untranslatedDecorations: string[] = [];
let requestCounter = 0;
const pendingRequests = new Map<string, number>();
let lastTriggeredLine: number | null = null;
let lastTriggeredTabId: string | null = null;
let toastTimeout: ReturnType<typeof setTimeout> | undefined;
let ignoreAutoToggleChange = true;
const contextLimits = { before: 1600, after: 900 };
let currentSettings: EditorSettings | null = null;
const defaultMaxLines = 3;
const STATUS_LOG_LIMIT = 40;

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

const positionLanguageMenu = () => {
  if (languageTrigger && languageMenu) {
    const rect = languageTrigger.getBoundingClientRect();
    languageMenu.style.left = `${rect.left}px`;
    languageMenu.style.top = `${rect.bottom + 6}px`;
    languageMenu.style.width = `${Math.max(180, Math.round(rect.width + 56))}px`;
  }
};

const openLanguageMenu = () => {
  positionLanguageMenu();
  languageDropdown?.classList.add('open');
  languageMenu?.classList.add('open');
  languageTrigger?.setAttribute('aria-expanded', 'true');
  languageMenu?.setAttribute('aria-hidden', 'false');
};

const closeLanguageMenu = () => {
  languageDropdown?.classList.remove('open');
  languageMenu?.classList.remove('open');
  languageTrigger?.setAttribute('aria-expanded', 'false');
  languageMenu?.setAttribute('aria-hidden', 'true');
};

const getTopMenuElements = (menuKey: string) => {
  const group = menuGroups.find(entry => entry.dataset.menu === menuKey);
  const trigger = group?.querySelector('.menu-item') as HTMLButtonElement | null;
  const dropdown = document.getElementById(`menu-${menuKey}-dropdown`) as HTMLDivElement | null;
  return { group, trigger, dropdown };
};

const positionTopMenu = (menuKey: string) => {
  const { trigger, dropdown } = getTopMenuElements(menuKey);
  if (!trigger || !dropdown) {
    return;
  }
  const rect = trigger.getBoundingClientRect();
  dropdown.style.left = `${rect.left}px`;
  dropdown.style.top = `${rect.bottom + 6}px`;
  dropdown.style.minWidth = `${Math.max(190, Math.round(rect.width + 36))}px`;
};

const closeTopMenus = () => {
  activeTopMenuKey = null;
  menuGroups.forEach(group => {
    const button = group.querySelector('.menu-item') as HTMLButtonElement | null;
    const menuKey = group.dataset.menu ?? '';
    const dropdown = document.getElementById(`menu-${menuKey}-dropdown`) as HTMLDivElement | null;
    button?.setAttribute('aria-expanded', 'false');
    dropdown?.classList.remove('open');
    dropdown?.setAttribute('aria-hidden', 'true');
    group.classList.remove('open');
  });
};

const openTopMenu = (menuKey: string) => {
  closeTopMenus();
  const { group, trigger, dropdown } = getTopMenuElements(menuKey);
  if (!group || !trigger || !dropdown) {
    return;
  }
  activeTopMenuKey = menuKey;
  positionTopMenu(menuKey);
  group.classList.add('open');
  trigger.setAttribute('aria-expanded', 'true');
  dropdown.classList.add('open');
  dropdown.setAttribute('aria-hidden', 'false');
};

const toggleTopMenu = (menuKey: string) => {
  if (activeTopMenuKey === menuKey) {
    closeTopMenus();
  } else {
    openTopMenu(menuKey);
  }
};

const applyTargetLanguage = (nextLanguage: string, persist = true) => {
  const normalizedLanguage = nextLanguage.toLowerCase() === 'python' ? 'python' : 'c';
  if (!currentSettings) {
    currentSettings = {
      autoTranslate,
      targetLanguage: normalizedLanguage,
      maxLinesPerTranslation: defaultMaxLines,
      context: { beforeChars: contextLimits.before, afterChars: contextLimits.after },
      ai: { provider: 'gemini', apiKey: '', model: 'gemini-2.5-flash' },
      editor: {
        tabSize: 4,
        insertSpaces: true,
        fontSize: 16,
        fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
        renderWhitespace: 'trailing',
        cursorStyle: 'line'
      },
      ui: {
        sidebarVisible,
        activityVisible,
        minimapEnabled,
        zoomLevel,
        theme: currentTheme
      }
    };
  } else {
    currentSettings.targetLanguage = normalizedLanguage;
  }

  if (languageTriggerValue) {
    languageTriggerValue.textContent = normalizedLanguage === 'python' ? 'Python' : 'C';
  }
  languageOptions.forEach(option => {
    const selected = option.dataset.language === normalizedLanguage;
    option.classList.toggle('active', selected);
    option.setAttribute('aria-selected', selected ? 'true' : 'false');
  });

  if (persist) {
    const savePromise = window.electronAPI?.saveSettings?.({ targetLanguage: normalizedLanguage });
    savePromise?.catch(error => {
      showToast(
        `Failed to persist language: ${error instanceof Error ? error.message : String(error)}`
      );
    });
  }
};

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

languageTrigger?.addEventListener('click', () => {
  const isOpen = languageDropdown?.classList.contains('open');
  if (isOpen) {
    closeLanguageMenu();
  } else {
    closeTopMenus();
    openLanguageMenu();
  }
});

menuItems.forEach(button => {
  button.addEventListener('click', event => {
    event.stopPropagation();
    const menuKey = button.closest('.menu-group')?.getAttribute('data-menu');
    if (!menuKey) {
      return;
    }
    closeLanguageMenu();
    toggleTopMenu(menuKey);
  });
});

menuCommands.forEach(commandButton => {
  commandButton.addEventListener('click', () => {
    const command = commandButton.dataset.command;
    if (!command) {
      return;
    }
    handleMenuCommand(command);
    closeTopMenus();
  });
});

languageOptions.forEach(option => {
  option.addEventListener('click', () => {
    applyTargetLanguage(option.dataset.language ?? 'c', true);
    closeLanguageMenu();
  });
});

document.addEventListener('click', event => {
  const target = event.target as Node;
  const clickedDropdown = languageDropdown?.contains(target) ?? false;
  const clickedMenu = languageMenu?.contains(target) ?? false;
  if (!clickedDropdown && !clickedMenu) {
    closeLanguageMenu();
  }

  const clickedTopMenuGroup = menuGroups.some(group => group.contains(target));
  const clickedTopMenuDropdown = menuDropdowns.some(dropdown => dropdown.contains(target));
  if (!clickedTopMenuGroup && !clickedTopMenuDropdown) {
    closeTopMenus();
  }
});

// Render the language menu at the document level so it always sits above editor layers.
if (languageMenu && languageMenu.parentElement !== document.body) {
  document.body.appendChild(languageMenu);
}
// Render top menus at the document level so they are not clipped by editor/terminal layers.
menuDropdowns.forEach(dropdown => {
  if (dropdown.parentElement !== document.body) {
    document.body.appendChild(dropdown);
  }
});

window.addEventListener('resize', () => {
  if (languageDropdown?.classList.contains('open')) {
    positionLanguageMenu();
  }
  if (activeTopMenuKey) {
    positionTopMenu(activeTopMenuKey);
  }
});

windowMinimizeButton?.addEventListener('click', () => {
  window.electronAPI?.window?.minimize();
});
windowMaximizeButton?.addEventListener('click', () => {
  window.electronAPI?.window?.maximize();
});
windowCloseButton?.addEventListener('click', () => {
  window.electronAPI?.window?.close();
});

themeCycleButton?.addEventListener('click', () => {
  const order: UiTheme[] = ['light', 'dark', 'glass'];
  const idx = order.indexOf(currentTheme);
  const nextTheme = order[(idx + 1) % order.length];
  applyTheme(nextTheme, true);
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

const highlightLine = (
  tabId: string,
  lineNumber: number,
  variant: 'pending' | 'error' | 'success' = 'pending'
) => {
  if (tabId !== activeTabId) {
    return;
  }
  const className = variant === 'error' ? 'line-commit-error' : 'line-commit-decoration';
  transientHighlightDecorations = editor.deltaDecorations(transientHighlightDecorations, [
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
    transientHighlightDecorations = editor.deltaDecorations(transientHighlightDecorations, []);
  }, 1100);
};

const isIgnorableLine = (trimmed: string) =>
  !trimmed ||
  trimmed.startsWith('//') ||
  trimmed.startsWith('#') ||
  trimmed.startsWith('/*') ||
  trimmed.startsWith('*') ||
  trimmed.startsWith('*/') ||
  /^[{}()[\];,]+$/.test(trimmed);

const refreshUntranslatedDecorations = () => {
  const active = getActiveTab();
  if (!active) {
    untranslatedDecorations = editor.deltaDecorations(untranslatedDecorations, []);
    return;
  }
  const lineCount = active.model.getLineCount();
  const nextDecorations: monaco.editor.IModelDeltaDecoration[] = [];
  for (let lineNumber = 1; lineNumber <= lineCount; lineNumber += 1) {
    const trimmed = active.model.getLineContent(lineNumber).trim();
    if (isIgnorableLine(trimmed)) {
      continue;
    }
    if (active.translatedLines.has(lineNumber)) {
      continue;
    }
    nextDecorations.push({
      range: new monaco.Range(lineNumber, 1, lineNumber, 1),
      options: {
        linesDecorationsClassName: 'line-untranslated-gutter'
      }
    });
  }
  untranslatedDecorations = editor.deltaDecorations(untranslatedDecorations, nextDecorations);
};

const invalidateTranslatedLines = (
  tab: EditorTab,
  event: monaco.editor.IModelContentChangedEvent
) => {
  // Monaco can emit multiple changes in one event. Process from bottom to top
  // so line-number shifts are applied consistently.
  const orderedChanges = [...event.changes].sort(
    (a, b) => b.range.startLineNumber - a.range.startLineNumber
  );

  for (const change of orderedChanges) {
    const startLine = change.range.startLineNumber;
    const endLine = change.range.endLineNumber;
    const replacedLineCount = endLine - startLine;
    const insertedLineCount = (change.text.match(/\n/g) ?? []).length;
    const lineDelta = insertedLineCount - replacedLineCount;
    const preserveStartLine =
      change.range.startLineNumber === change.range.endLineNumber &&
      change.range.startColumn === change.range.endColumn &&
      change.text.startsWith('\n');

    const remapped = new Set<number>();
    tab.translatedLines.forEach(line => {
      if (line < startLine) {
        remapped.add(line);
        return;
      }
      if (line > endLine) {
        remapped.add(line + lineDelta);
        return;
      }
      if (preserveStartLine && line === startLine) {
        remapped.add(line);
      }
    });
    tab.translatedLines = remapped;
  }
};

const markTranslatedLineRange = (tab: EditorTab, startLine: number, totalLines: number) => {
  const finalLine = Math.min(tab.model.getLineCount(), startLine + Math.max(1, totalLines) - 1);
  for (let line = startLine; line <= finalLine; line += 1) {
    tab.translatedLines.add(line);
  }
};

createUntitledTab(welcomeSnippet);
loadWorkspaceTree();

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
  markTranslatedLineRange(targetTab, lineNumber, countSnippetLines(indentedSnippet));
  targetTab.dirty = true;
  renderTabs();
  refreshUntranslatedDecorations();
  highlightLine(tabId, lineNumber, 'success');

  // Position cursor appropriately after insertion
  if (tabId === activeTabId) {
    positionCursorAfterInsertion(lineNumber, indentedSnippet, indent);
  }
};

/**
 * Position cursor appropriately after code insertion.
 * - For block statements (if, for, while, switch): position inside the braces
 * - For regular statements: position at end of last line with proper indentation
 */
const positionCursorAfterInsertion = (startLine: number, snippet: string, baseIndent: string) => {
  const lines = snippet.split('\n');
  const model = editor.getModel();
  if (!model) return;
  
  // First, try to find a line ending with an opening brace and position inside it
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trimEnd();
    if (line.endsWith('{')) {
      // The cursor should go on the next line, inside the block
      const targetLine = startLine + i + 1;
      
      // Get the indent of the line with the brace to calculate block indent
      const braceLineIndent = lines[i].match(/^\s*/)?.[0] ?? '';
      const blockIndent = braceLineIndent + '    ';
      
      // Check if the next line exists
      if (targetLine <= model.getLineCount()) {
        const nextLineContent = model.getLineContent(targetLine);
        
        // If it's an empty/whitespace line or closing brace, position cursor there
        if (nextLineContent.trim() === '' || nextLineContent.trim() === '}') {
          editor.setPosition({ 
            lineNumber: targetLine, 
            column: blockIndent.length + 1 
          });
          editor.focus();
          return;
        }
        
        // If the next line has content (like i++ in while loops), 
        // we still want cursor before that content
        // Check if there's an empty line before the content line
        const lineContent = nextLineContent.trimStart();
        if (lineContent && !lineContent.startsWith('}')) {
          // Position at the beginning of this line with proper indent
          editor.setPosition({
            lineNumber: targetLine,
            column: blockIndent.length + 1
          });
          editor.focus();
          return;
        }
      }
    }
  }
  
  // No braces found - position cursor at the end of the inserted code
  // on a new line with the same base indentation
  const lastInsertedLine = startLine + lines.length - 1;
  
  // Move to the line after the last inserted line if it exists
  // Otherwise stay on the last inserted line
  if (lastInsertedLine + 1 <= model.getLineCount()) {
    const nextLine = lastInsertedLine + 1;
    const nextLineContent = model.getLineContent(nextLine);
    const nextLineIndent = nextLineContent.match(/^\s*/)?.[0] ?? '';
    
    // If the next line is empty or has less/equal indent, we can position there
    if (nextLineContent.trim() === '' || nextLineIndent.length <= baseIndent.length) {
      editor.setPosition({
        lineNumber: nextLine,
        column: baseIndent.length + 1
      });
      editor.focus();
      return;
    }
  }
  
  // Default: position at end of last inserted line
  const lastLineContent = model.getLineContent(lastInsertedLine);
  editor.setPosition({
    lineNumber: lastInsertedLine,
    column: lastLineContent.length + 1
  });
  editor.focus();
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
    provider: currentSettings?.ai.provider || undefined,
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
  if (activeTab) {
    invalidateTranslatedLines(activeTab, event);
    refreshUntranslatedDecorations();
  }
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
    applyTheme(normalizeTheme(uiPrefs.theme), false);
    applyEditorPreferences(settings.editor);

    if (autoToggle) {
      ignoreAutoToggleChange = true;
      autoToggle.checked = autoTranslate;
      ignoreAutoToggleChange = false;
    }
    applyTargetLanguage(settings.targetLanguage, false);
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

/**
 * Run the current file in a terminal
 */
const runCurrentFile = async () => {
  const tab = getActiveTab();
  if (!tab) {
    showToast('No file open to run');
    return;
  }

  // Save file first if dirty
  if (tab.dirty) {
    await saveActiveTab(false);
  }

  if (!tab.absolutePath) {
    showToast('Save the file first before running');
    return;
  }

  if (!terminalPanel) {
    showToast('Terminal not available');
    return;
  }

  // Show terminal if hidden
  if (!terminalPanel.visible) {
    terminalPanel.show();
  }

  const filePath = tab.absolutePath;
  const lang = tab.language;
  let command = '';

  if (lang === 'c' || lang === 'cpp') {
    // Compile and run C/C++
    const outputPath = filePath.replace(/\.(c|cpp)$/, '');
    const compiler = lang === 'cpp' ? 'g++' : 'gcc';
    command = `${compiler} "${filePath}" -o "${outputPath}" && "${outputPath}"`;
  } else if (lang === 'python') {
    command = `python3 "${filePath}"`;
  } else if (lang === 'javascript') {
    command = `node "${filePath}"`;
  } else if (lang === 'typescript') {
    command = `npx ts-node "${filePath}"`;
  } else {
    showToast(`Don't know how to run ${lang} files`);
    return;
  }

  await terminalPanel.runCommand(command, `Run: ${tab.title}`);
  pushStatus(`Running ${tab.title}`);
};

/**
 * Build the current file without running
 */
const buildCurrentFile = async () => {
  const tab = getActiveTab();
  if (!tab) {
    showToast('No file open to build');
    return;
  }

  // Save file first if dirty
  if (tab.dirty) {
    await saveActiveTab(false);
  }

  if (!tab.absolutePath) {
    showToast('Save the file first before building');
    return;
  }

  if (!terminalPanel) {
    showToast('Terminal not available');
    return;
  }

  // Show terminal if hidden
  if (!terminalPanel.visible) {
    terminalPanel.show();
  }

  const filePath = tab.absolutePath;
  const lang = tab.language;
  let command = '';

  if (lang === 'c' || lang === 'cpp') {
    const outputPath = filePath.replace(/\.(c|cpp)$/, '');
    const compiler = lang === 'cpp' ? 'g++' : 'gcc';
    command = `${compiler} "${filePath}" -o "${outputPath}"`;
  } else if (lang === 'typescript') {
    command = `npx tsc "${filePath}"`;
  } else {
    showToast(`No build step for ${lang} files`);
    return;
  }

  await terminalPanel.runCommand(command, `Build: ${tab.title}`);
  pushStatus(`Building ${tab.title}`);
};

// Add keyboard shortcut for terminal toggle (Ctrl+`)
document.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    closeLanguageMenu();
    closeTopMenus();
  }
  if (e.ctrlKey && e.key === '`') {
    e.preventDefault();
    terminalPanel?.toggle();
  }
});

