/**
 * TerminalInstance - Wrapper for xterm.js terminal
 */

import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import 'xterm/css/xterm.css';
import type { TerminalDataEvent, TerminalExitEvent } from '../types/electron';

export interface TerminalInstanceOptions {
  id: string;
  container: HTMLElement;
  cwd?: string;
  onClose?: (id: string) => void;
}

export class TerminalInstance {
  public readonly id: string;
  private terminal: Terminal;
  private fitAddon: FitAddon;
  private container: HTMLElement;
  private cwd?: string;
  private isConnected: boolean = false;
  private dataListener?: () => void;
  private exitListener?: () => void;
  private onClose?: (id: string) => void;

  constructor(options: TerminalInstanceOptions) {
    this.id = options.id;
    this.container = options.container;
    this.cwd = options.cwd;
    this.onClose = options.onClose;

    // Create xterm.js terminal with dark theme
    this.terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      fontSize: 14,
      fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", Consolas, monospace',
      lineHeight: 1.2,
      theme: {
        background: '#1a1b26',
        foreground: '#a9b1d6',
        cursor: '#c0caf5',
        cursorAccent: '#1a1b26',
        selectionBackground: '#33467c',
        selectionForeground: '#c0caf5',
        black: '#32344a',
        red: '#f7768e',
        green: '#9ece6a',
        yellow: '#e0af68',
        blue: '#7aa2f7',
        magenta: '#ad8ee6',
        cyan: '#449dab',
        white: '#787c99',
        brightBlack: '#444b6a',
        brightRed: '#ff7a93',
        brightGreen: '#b9f27c',
        brightYellow: '#ff9e64',
        brightBlue: '#7da6ff',
        brightMagenta: '#bb9af7',
        brightCyan: '#0db9d7',
        brightWhite: '#acb0d0',
      },
      allowProposedApi: true,
    });

    // Create fit addon for auto-resizing
    this.fitAddon = new FitAddon();
    this.terminal.loadAddon(this.fitAddon);

    // Load web links addon for clickable URLs
    const webLinksAddon = new WebLinksAddon();
    this.terminal.loadAddon(webLinksAddon);

    // Open terminal in container
    this.terminal.open(this.container);
    
    // Initial fit
    setTimeout(() => this.fit(), 0);

    // Handle resize
    const resizeObserver = new ResizeObserver(() => this.fit());
    resizeObserver.observe(this.container);

    // Handle user input
    this.terminal.onData((data) => {
      if (this.isConnected) {
        window.electronAPI?.terminal?.write(this.id, data);
      }
    });
  }

  /**
   * Connect to a PTY session
   */
  async connect(): Promise<boolean> {
    const api = window.electronAPI?.terminal;
    if (!api) {
      this.terminal.writeln('\r\n\x1b[31mTerminal API not available\x1b[0m');
      return false;
    }

    // Create PTY session with workspace directory
    const result = await api.create({
      cols: this.terminal.cols,
      rows: this.terminal.rows,
      cwd: this.cwd,
    });

    if (!result.success || !result.id) {
      this.terminal.writeln(`\r\n\x1b[31mFailed to create terminal: ${result.error}\x1b[0m`);
      return false;
    }

    // Update ID to match backend
    (this as { id: string }).id = result.id;
    this.isConnected = true;

    // Listen for data from PTY
    this.dataListener = api.onData((event: TerminalDataEvent) => {
      if (event.id === this.id) {
        this.terminal.write(event.data);
      }
    });

    // Listen for exit
    this.exitListener = api.onExit((event: TerminalExitEvent) => {
      if (event.id === this.id) {
        this.isConnected = false;
        this.terminal.writeln(`\r\n\x1b[33mTerminal exited with code ${event.exitCode}\x1b[0m`);
        this.onClose?.(this.id);
      }
    });

    return true;
  }

  /**
   * Write data to the terminal display
   */
  write(data: string): void {
    this.terminal.write(data);
  }

  /**
   * Fit terminal to container
   */
  fit(): void {
    try {
      this.fitAddon.fit();
      if (this.isConnected) {
        window.electronAPI?.terminal?.resize(this.id, this.terminal.cols, this.terminal.rows);
      }
    } catch (e) {
      // Ignore fit errors (container might not be visible)
    }
  }

  /**
   * Focus the terminal
   */
  focus(): void {
    this.terminal.focus();
  }

  /**
   * Clear the terminal
   */
  clear(): void {
    this.terminal.clear();
  }

  /**
   * Dispose of the terminal
   */
  dispose(): void {
    this.dataListener?.();
    this.exitListener?.();
    
    if (this.isConnected) {
      window.electronAPI?.terminal?.close(this.id);
    }
    
    this.terminal.dispose();
  }

  /**
   * Get the terminal dimensions
   */
  getDimensions(): { cols: number; rows: number } {
    return {
      cols: this.terminal.cols,
      rows: this.terminal.rows,
    };
  }

  /**
   * Check if connected
   */
  get connected(): boolean {
    return this.isConnected;
  }
}

