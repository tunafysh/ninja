"use client"

import { useRef, useState, useEffect } from "react"
import type { Terminal as XTerminal } from "xterm"
import type { FitAddon as XTermFitAddon } from "xterm-addon-fit"
import { JSX } from 'react/jsx-runtime';
import { isTauriAvailable } from "@/lib/tauri-api";

// Define proper types for the terminal themes
interface TerminalTheme {
  background: string
  foreground: string
  cursor: string
  selection: string
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
  brightBlack: string
  brightRed: string
  brightGreen: string
  brightYellow: string
  brightBlue: string
  brightMagenta: string
  brightCyan: string
  brightWhite: string
}

// Terminal themes with proper typing
const terminalThemes: Record<'dark' | 'light', TerminalTheme> = {
  dark: {
    background: '#282c34',
    foreground: '#abb2bf',
    cursor: '#528bff',
    selection: '#3E4451',
    black: '#282c34',
    red: '#e06c75',
    green: '#98c379',
    yellow: '#e5c07b',
    blue: '#61afef',
    magenta: '#c678dd',
    cyan: '#56b6c2',
    white: '#abb2bf',
    brightBlack: '#5c6370',
    brightRed: '#e06c75',
    brightGreen: '#98c379',
    brightYellow: '#e5c07b',
    brightBlue: '#61afef',
    brightMagenta: '#c678dd',
    brightCyan: '#56b6c2',
    brightWhite: '#ffffff',
  },
  light: {
    background: '#ffffff',
    foreground: '#24292e',
    cursor: '#044289',
    selection: '#c8e1ff',
    black: '#24292e',
    red: '#d73a49',
    green: '#22863a',
    yellow: '#e36209',
    blue: '#005cc5',
    magenta: '#6f42c1',
    cyan: '#1b7c83',
    white: '#6a737d',
    brightBlack: '#959da5',
    brightRed: '#cb2431',
    brightGreen: '#28a745',
    brightYellow: '#f66a0a',
    brightBlue: '#2188ff',
    brightMagenta: '#8a63d2',
    brightCyan: '#3192aa',
    brightWhite: '#d1d5da',
  }
};

// Define props interface
interface TerminalComponentProps {
  isDarkMode: boolean;
}

// Define window augmentation for global types
declare global {
  interface Window {
    Terminal?: typeof XTerminal;
    FitAddon?: typeof XTermFitAddon;
  }
}

// Command handler type for built-in commands
type CommandHandler = (args: string[], term: XTerminal) => Promise<void>;

// Available built-in commands
const builtInCommands: Record<string, CommandHandler> = {
  clear: async (_, term) => {
    term.clear();
  },
  help: async (_, term) => {
    term.writeln('Available commands:');
    term.writeln('  clear - Clear the terminal screen');
    term.writeln('  help - Show this help message');
    term.writeln('  Any shell command - Executed via Tauri');
    term.writeln('');
    term.writeln('Examples:');
    term.writeln('  ls -la');
    term.writeln('  cd /path/to/directory');
    term.writeln('  echo "Hello World"');
  }
};

export default function TerminalComponent({ isDarkMode }: TerminalComponentProps): JSX.Element {
  const terminalRef = useRef<HTMLDivElement>(null);
  const [isTerminalReady, setIsTerminalReady] = useState<boolean>(false);
  const [terminal, setTerminal] = useState<XTerminal | null>(null);
  const fitAddonRef = useRef<XTermFitAddon | null>(null);
  const commandBufferRef = useRef<string>('');
  const [currentDir, setCurrentDir] = useState<string>('');
  const [isTauriReady, setIsTauriReady] = useState<boolean>(false);

  // Check if Tauri is available
  useEffect(() => {
    const checkTauri = async () => {
      if (isTauriAvailable()) {
        setIsTauriReady(true);
        try {
          const dir = await window.__TAURI__.core.invoke('get_current_dir');
          setCurrentDir(dir);
        } catch (error) {
          console.error('Failed to get current directory:', error);
        }
      }
    };
    
    checkTauri();
  }, []);

  // Load xterm scripts
  useEffect(() => {
    // Check if we're in the browser
    if (typeof window === 'undefined') return;

    // Load xterm dynamically
    const loadXterm = async (): Promise<void> => {
      try {
        // Import the modules
        const xtermModule = await import('xterm');
        const fitAddonModule = await import('xterm-addon-fit');
        
        // Import CSS
        await import('xterm/css/xterm.css');
        
        // Set global references
        window.Terminal = xtermModule.Terminal;
        window.FitAddon = fitAddonModule.FitAddon;
        
        setIsTerminalReady(true);
      } catch (error) {
        console.error('Failed to load xterm:', error);
      }
    };

    loadXterm();
  }, []);

  // Process command
  const processCommand = async (command: string, term: XTerminal): Promise<void> => {
    const trimmedCommand = command.trim();
    if (!trimmedCommand) return;

    const parts = trimmedCommand.split(' ');
    const cmd = parts[0].toLowerCase();
    const args = parts.slice(1);

    // Check if it's a built-in command
    if (builtInCommands[cmd]) {
      await builtInCommands[cmd](args, term);
      return;
    }

    // Execute via Tauri if available
    if (isTauriReady && window.__TAURI__) {
      try {
        const result = await window.__TAURI__.core.invoke('execute_command', { command: trimmedCommand });
        if (result) {
          term.writeln(result);
        }
        
        // Update current directory after command execution
        const newDir = await window.__TAURI__.core.invoke('get_current_dir');
        setCurrentDir(newDir);
      } catch (error) {
        term.writeln(`\x1b[31mError: ${error}\x1b[0m`);
      }
    } else {
      // Fallback for when Tauri is not available
      term.writeln(`Command '${cmd}' executed in browser environment.`);
      term.writeln('Note: Real command execution requires Tauri desktop app.');
      
      // Simulate some common commands for demo purposes
      if (cmd === 'ls') {
        term.writeln('index.html');
        term.writeln('styles.css');
        term.writeln('app.js');
        term.writeln('README.md');
      } else if (cmd === 'pwd') {
        term.writeln('/home/user/projects');
      } else if (cmd === 'echo') {
        term.writeln(args.join(' '));
      } else {
        term.writeln(`Command not found: ${cmd}`);
      }
    }
  };

  // Initialize terminal after scripts are loaded
  useEffect(() => {
    if (!isTerminalReady || !terminalRef.current || !window.Terminal || !window.FitAddon) return;

    try {
      const Terminal = window.Terminal;
      const FitAddon = window.FitAddon;

      // Initialize terminal with proper typing
      const term = new Terminal({
        theme: isDarkMode ? terminalThemes.dark : terminalThemes.light,
        fontFamily: 'Menlo, Monaco, "Courier New", monospace',
        fontSize: 14,
        cursorBlink: true,
      });

      // Add fit addon
      const fitAddon = new FitAddon();
      fitAddonRef.current = fitAddon;
      term.loadAddon(fitAddon);

      // Open terminal
      term.open(terminalRef.current);
      
      // Fit terminal after a short delay to ensure DOM is ready
      setTimeout(() => {
        try {
          if (fitAddon && typeof fitAddon.fit === 'function') {
            fitAddon.fit();
          }
        } catch (e) {
          console.error('Error fitting terminal:', e);
        }
      }, 100);

      // Write welcome message
      term.writeln('Apache Configuration Terminal');
      if (isTauriReady) {
        term.writeln('Tauri integration active - commands will be executed on your system');
      } else {
        term.writeln('Running in browser mode - limited command functionality');
      }
      term.writeln('Type "help" for available commands');
      
      // Show prompt
      const writePrompt = () => {
        const dir = currentDir || '/home/user';
        const shortDir = dir.split('/').pop() || dir;
        term.write(`\r\n\x1b[32muser@apache\x1b[0m:\x1b[34m${shortDir}\x1b[0m$ `);
      };
      
      writePrompt();

      // Handle input with proper typing
      term.onData((data: string) => {
        if (data === '\r') { // Enter key
          term.write('\r\n');
          
          // Process the command
          const command = commandBufferRef.current;
          commandBufferRef.current = '';
          
          // Execute the command asynchronously
          processCommand(command, term).then(writePrompt);
        } else if (data === '\u007f') { // Backspace
          if (commandBufferRef.current.length > 0) {
            commandBufferRef.current = commandBufferRef.current.slice(0, -1);
            // Move cursor backward, write space, move cursor backward again
            term.write('\b \b');
          }
        } else if (data >= ' ' && data <= '~') { // Printable characters
          term.write(data);
          commandBufferRef.current += data;
        }
      });

      setTerminal(term);

      // Resize handler
      const handleResize = (): void => {
        try {
          if (fitAddonRef.current && typeof fitAddonRef.current.fit === 'function') {
            fitAddonRef.current.fit();
          }
        } catch (e) {
          console.error('Error during resize:', e);
        }
      };

      window.addEventListener('resize', handleResize);

      // Return cleanup function
      return () => {
        window.removeEventListener('resize', handleResize);
        if (term) {
          try {
            term.dispose();
          } catch (e) {
            console.error('Error disposing terminal:', e);
          }
        }
      };
    } catch (error) {
      console.error('Error initializing terminal:', error);
      return undefined;
    }
  }, [isTerminalReady, isDarkMode, isTauriReady, currentDir]);

  // Update theme when mode changes
  useEffect(() => {
    if (terminal && isTerminalReady) {
      try {
        terminal.options.theme = isDarkMode ? terminalThemes.dark : terminalThemes.light;
      } catch (e) {
        console.error('Error updating terminal theme:', e);
      }
    }
  }, [isDarkMode, terminal, isTerminalReady]);

  return (
    <>
      <div ref={terminalRef} className="h-full w-full" />
      {!isTerminalReady && (
        <div className="flex items-center justify-center h-full">
          <div>Loading terminal...</div>
        </div>
      )}
    </>
  );
}
