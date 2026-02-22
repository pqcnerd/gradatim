/**
 * TerminalPanel - Main terminal panel with tabs
 */

import { TerminalInstance } from './TerminalInstance';

export interface TerminalTab {
  id: string;
  name: string;
  instance: TerminalInstance;
  element: HTMLElement;
}

export class TerminalPanel {
  private container: HTMLElement;
  private tabsContainer: HTMLElement;
  private terminalsContainer: HTMLElement;
  private tabs: Map<string, TerminalTab> = new Map();
  private activeTabId: string | null = null;
  private tabCounter: number = 0;
  private isVisible: boolean = true;
  private panelHeight: number = 300;
  private minHeight: number = 100;
  private maxHeightRatio: number = 0.7;
  private _workspacePath: string | null = null;
  private currentTheme: 'light' | 'dark' | 'glass' = 'light';

  constructor(container: HTMLElement) {
    this.container = container;
    this.container.className = 'terminal-panel';
    
    // Create panel structure
    this.container.innerHTML = `
      <div class="terminal-panel-header">
        <div class="terminal-panel-resize-handle"></div>
        <div class="terminal-tabs"></div>
        <div class="terminal-panel-actions">
          <button class="terminal-btn terminal-btn-new" title="New Terminal">+</button>
          <button class="terminal-btn terminal-btn-toggle" title="Toggle Terminal">▼</button>
        </div>
      </div>
      <div class="terminal-panel-content">
        <div class="terminal-instances"></div>
      </div>
    `;

    this.tabsContainer = this.container.querySelector('.terminal-tabs')!;
    this.terminalsContainer = this.container.querySelector('.terminal-instances')!;

    // Set up event listeners
    this.setupEventListeners();

    // Set initial height
    this.setHeight(this.panelHeight);
  }

  private setupEventListeners(): void {
    // New terminal button
    const newBtn = this.container.querySelector('.terminal-btn-new');
    newBtn?.addEventListener('click', () => this.createTerminal());

    // Toggle button
    const toggleBtn = this.container.querySelector('.terminal-btn-toggle');
    toggleBtn?.addEventListener('click', () => this.toggle());

    // Resize handle
    const resizeHandle = this.container.querySelector('.terminal-panel-resize-handle');
    if (resizeHandle) {
      this.setupResize(resizeHandle as HTMLElement);
    }
  }

  private setupResize(handle: HTMLElement): void {
    let startY = 0;
    let startHeight = 0;

    const onMouseMove = (e: MouseEvent) => {
      const deltaY = startY - e.clientY;
      const newHeight = Math.min(
        Math.max(startHeight + deltaY, this.minHeight),
        window.innerHeight * this.maxHeightRatio
      );
      this.setHeight(newHeight);
      this.fitAllTerminals();
    };

    const onMouseUp = () => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };

    handle.addEventListener('mousedown', (e: MouseEvent) => {
      e.preventDefault();
      startY = e.clientY;
      startHeight = this.panelHeight;
      document.body.style.cursor = 'ns-resize';
      document.body.style.userSelect = 'none';
      document.addEventListener('mousemove', onMouseMove);
      document.addEventListener('mouseup', onMouseUp);
    });
  }

  /**
   * Create a new terminal tab
   */
  async createTerminal(name?: string, cwd?: string): Promise<string | null> {
    const tabId = `tab-${++this.tabCounter}`;
    const tabName = name || `Terminal ${this.tabCounter}`;

    // Create tab element
    const tabElement = document.createElement('div');
    tabElement.className = 'terminal-tab';
    tabElement.dataset.tabId = tabId;
    tabElement.innerHTML = `
      <span class="terminal-tab-name">${tabName}</span>
      <button class="terminal-tab-close" title="Close">×</button>
    `;

    // Tab click handler
    tabElement.addEventListener('click', (e) => {
      if (!(e.target as HTMLElement).classList.contains('terminal-tab-close')) {
        this.activateTab(tabId);
      }
    });

    // Close button handler
    const closeBtn = tabElement.querySelector('.terminal-tab-close');
    closeBtn?.addEventListener('click', (e) => {
      e.stopPropagation();
      this.closeTab(tabId);
    });

    this.tabsContainer.appendChild(tabElement);

    // Create terminal container
    const terminalElement = document.createElement('div');
    terminalElement.className = 'terminal-instance';
    terminalElement.dataset.tabId = tabId;
    this.terminalsContainer.appendChild(terminalElement);

    // Create terminal instance with workspace path
    const workingDir = cwd || this._workspacePath || undefined;
    const instance = new TerminalInstance({
      id: tabId,
      container: terminalElement,
      cwd: workingDir,
      theme: this.currentTheme,
      onClose: (id) => this.handleTerminalExit(id),
    });

    // Store tab
    const tab: TerminalTab = {
      id: tabId,
      name: tabName,
      instance,
      element: tabElement,
    };
    this.tabs.set(tabId, tab);

    // Activate this tab
    this.activateTab(tabId);

    // Connect to PTY
    const connected = await instance.connect();
    if (!connected) {
      // Keep tab open but show error
      instance.write('\r\n\x1b[33mPress any key to retry connection...\x1b[0m');
    }

    // Ensure panel is visible
    if (!this.isVisible) {
      this.show();
    }

    return connected ? instance.id : null;
  }

  /**
   * Activate a tab
   */
  activateTab(tabId: string): void {
    const tab = this.tabs.get(tabId);
    if (!tab) return;

    // Deactivate current tab
    if (this.activeTabId) {
      const currentTab = this.tabs.get(this.activeTabId);
      if (currentTab) {
        currentTab.element.classList.remove('active');
        const currentTerminal = this.terminalsContainer.querySelector(
          `.terminal-instance[data-tab-id="${this.activeTabId}"]`
        ) as HTMLElement;
        if (currentTerminal) {
          currentTerminal.style.display = 'none';
        }
      }
    }

    // Activate new tab
    tab.element.classList.add('active');
    const terminalElement = this.terminalsContainer.querySelector(
      `.terminal-instance[data-tab-id="${tabId}"]`
    ) as HTMLElement;
    if (terminalElement) {
      terminalElement.style.display = 'block';
    }

    this.activeTabId = tabId;
    
    // Fit and focus
    setTimeout(() => {
      tab.instance.fit();
      tab.instance.focus();
    }, 0);
  }

  /**
   * Close a tab
   */
  closeTab(tabId: string): void {
    const tab = this.tabs.get(tabId);
    if (!tab) return;

    // Dispose terminal
    tab.instance.dispose();

    // Remove elements
    tab.element.remove();
    const terminalElement = this.terminalsContainer.querySelector(
      `.terminal-instance[data-tab-id="${tabId}"]`
    );
    terminalElement?.remove();

    // Remove from map
    this.tabs.delete(tabId);

    // If this was the active tab, activate another
    if (this.activeTabId === tabId) {
      this.activeTabId = null;
      const remaining = Array.from(this.tabs.keys());
      if (remaining.length > 0) {
        this.activateTab(remaining[remaining.length - 1]);
      }
    }
  }

  /**
   * Handle terminal exit
   */
  private handleTerminalExit(id: string): void {
    // Terminal exited, but keep tab open
    const tab = this.tabs.get(id);
    if (tab) {
      tab.element.classList.add('exited');
    }
  }

  /**
   * Toggle panel visibility
   */
  toggle(): void {
    if (this.isVisible) {
      this.hide();
    } else {
      this.show();
    }
  }

  /**
   * Show panel
   */
  show(): void {
    this.isVisible = true;
    this.container.classList.remove('hidden');
    this.setHeight(this.panelHeight);
    
    // Create a terminal if none exist
    if (this.tabs.size === 0) {
      this.createTerminal();
    } else {
      this.fitAllTerminals();
      const activeTab = this.activeTabId ? this.tabs.get(this.activeTabId) : null;
      activeTab?.instance.focus();
    }

    // Update toggle button
    const toggleBtn = this.container.querySelector('.terminal-btn-toggle');
    if (toggleBtn) {
      toggleBtn.textContent = '▼';
      toggleBtn.setAttribute('title', 'Hide Terminal');
    }

    // Dispatch event for layout update
    window.dispatchEvent(new CustomEvent('terminal-panel-resize'));
  }

  /**
   * Hide panel
   */
  hide(): void {
    this.isVisible = false;
    this.container.classList.add('hidden');
    this.container.style.height = '0';

    // Update toggle button
    const toggleBtn = this.container.querySelector('.terminal-btn-toggle');
    if (toggleBtn) {
      toggleBtn.textContent = '▲';
      toggleBtn.setAttribute('title', 'Show Terminal');
    }

    // Dispatch event for layout update
    window.dispatchEvent(new CustomEvent('terminal-panel-resize'));
  }

  /**
   * Set panel height
   */
  private setHeight(height: number): void {
    this.panelHeight = height;
    this.container.style.height = `${height}px`;
  }

  /**
   * Fit all terminals
   */
  private fitAllTerminals(): void {
    for (const tab of this.tabs.values()) {
      tab.instance.fit();
    }
  }

  /**
   * Run a command in a new or existing terminal
   */
  async runCommand(command: string, name?: string): Promise<void> {
    // Create new terminal for the command
    const terminalId = await this.createTerminal(name || 'Output');
    if (terminalId) {
      // The command will be executed when the terminal connects
      // For now, we wait a bit and then send the command
      setTimeout(() => {
        const tab = Array.from(this.tabs.values()).find(t => t.instance.id === terminalId);
        if (tab?.instance.connected) {
          window.electronAPI?.terminal?.write(terminalId, command + '\r');
        }
      }, 500);
    }
  }

  /**
   * Get visibility state
   */
  get visible(): boolean {
    return this.isVisible;
  }

  /**
   * Get active terminal instance
   */
  get activeTerminal(): TerminalInstance | null {
    if (!this.activeTabId) return null;
    return this.tabs.get(this.activeTabId)?.instance || null;
  }

  /**
   * Dispose all terminals
   */
  dispose(): void {
    for (const tab of this.tabs.values()) {
      tab.instance.dispose();
    }
    this.tabs.clear();
  }

  /**
   * Set the workspace path for new terminals
   */
  set workspacePath(path: string | null) {
    this._workspacePath = path;
  }

  /**
   * Get the current workspace path
   */
  get workspacePath(): string | null {
    return this._workspacePath;
  }

  setTheme(theme: 'light' | 'dark' | 'glass'): void {
    this.currentTheme = theme;
    for (const tab of this.tabs.values()) {
      tab.instance.setTheme(theme);
    }
  }
}

