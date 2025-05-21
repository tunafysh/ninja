import React, { useEffect, useRef } from 'react';
import { Editor, useMonaco } from '@monaco-editor/react';

export default function Scripting() {
  const editorRef = useRef(null);
  const [value, setValue] = React.useState<string>('');
  const [mounted, setMounted] = React.useState(false);
  const monaco = useMonaco();
  const monacoColorScheme = {
  "base": "vs-dark",
  "inherit": true,
  "rules": [],
  "colors": {
    "editor.background": "#18181b", // zinc-900
    "editor.foreground": "#f4f4f5", // zinc-100
    "editor.lineHighlightBackground": "#27272a", // zinc-800
    "editorCursor.foreground": "#f4f4f5", // zinc-100
    "editor.selectionBackground": "#3f3f46", // zinc-700
    "editor.inactiveSelectionBackground": "#3f3f4655", // zinc-700 with opacity
    "editor.lineHighlightBorder": "#27272a", // zinc-800
    "editorGutter.background": "#18181b", // zinc-900
    "editorGutter.modifiedBackground": "#facc15", // yellow-400 (for modified)
    "editorGutter.addedBackground": "#4ade80", // green-400 (for added)
    "editorGutter.deletedBackground": "#f87171", // red-400 (for deleted)
    "editorBracketMatch.background": "#3f3f46", // zinc-700
    "editorBracketMatch.border": "#71717a", // zinc-500
    "editorIndentGuide.background": "#27272a", // zinc-800
    "editorIndentGuide.activeBackground": "#3f3f46", // zinc-700
    "editorWhitespace.foreground": "#3f3f46" // zinc-700
  }
}

  const handleThemeChange = (monaco: any) => {
    monaco.editor.defineTheme('zinc-dark', monacoColorScheme);
  }

  const handleEditorDidMount = (editor: any) => {
    setMounted(true);
    editorRef.current = editor;
  };
  const handleChange = (value: string | undefined) => {
    if (mounted && value !== undefined) {
        setValue(value);
    }
  };

  useEffect(() => {
    if(monaco) {
      monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
        target: monaco.languages.typescript.ScriptTarget.ES2020,
        allowNonTsExtensions: true,
        moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
        module: monaco.languages.typescript.ModuleKind.CommonJS,
        noEmit: true,
        // Only include ECMAScript library, no DOM
        lib: ['es2020'],
      });

      monaco.editor.addKeybindingRule({
        keybinding: monaco.KeyCode.F1,
        command: "none"
      })

      // 2. Clear existing default libraries and type definitions
      monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: false,
        noSyntaxValidation: false,
      });

      // 3. Disable all built-in libraries
      // This is the key step - we're overriding the default libraries with our own minimal set
      const jsDefaults = monaco.languages.typescript.javascriptDefaults;
      
      // First, remove any existing extra libraries
      // (This is a workaround as there's no direct API to remove all libraries)
      const existingLibs = jsDefaults.getExtraLibs();
      Object.keys(existingLibs).forEach(lib => {
        // Unfortunately, there's no direct API to remove libs, but we can override them
        jsDefaults.addExtraLib('', lib);
      });

      // 4. Add only the minimal JavaScript functionality we want to allow
      // This defines a very minimal subset of JavaScript globals
      const minimalLib = `
        // Only allow these basic JavaScript constructs
        
        // Basic console for debugging (optional - remove if you want to disable console too)
        declare const console: {
          log(...args: any[]): void;
          error(...args: any[]): void;
          warn(...args: any[]): void;
          info(...args: any[]): void;
        };
        
        // Basic JavaScript types and functions
        declare const Math: Math;
        declare const JSON: JSON;
        declare const Array: ArrayConstructor;
        declare const Object: ObjectConstructor;
        declare const String: StringConstructor;
        declare const Number: NumberConstructor;
        declare const Boolean: BooleanConstructor;
        declare const RegExp: RegExpConstructor;
        declare const Date: DateConstructor;
        declare const Error: ErrorConstructor;
        declare const Map: MapConstructor;
        declare const Set: SetConstructor;
        declare const WeakMap: WeakMapConstructor;
        declare const WeakSet: WeakSetConstructor;
        declare const Promise: PromiseConstructor;
        
        // Explicitly declare that common browser globals don't exist
        // This helps prevent accidental usage
        declare const window: undefined;
        declare const document: undefined;
        declare const navigator: undefined;
        declare const location: undefined;
        declare const localStorage: undefined;
        declare const sessionStorage: undefined;
        declare const fetch: undefined;
        declare const XMLHttpRequest: undefined;
        declare const alert: undefined;
        declare const confirm: undefined;
        declare const prompt: undefined;
        declare const setTimeout: undefined;
        declare const setInterval: undefined;
        declare const clearTimeout: undefined;
        declare const clearInterval: undefined;
        declare const addEventListener: undefined;
        declare const removeEventListener: undefined;
      `;

      // Add our minimal library
      jsDefaults.addExtraLib(minimalLib, 'minimal-js-env.d.ts');
    }
  });

  return (
    <div className='h-full '>
      <Editor
        height="100%"
        width="100%"
        language="javascript"
        value={value}
        beforeMount={handleThemeChange}
        theme="zinc-dark"
        onMount={handleEditorDidMount}
        onChange={handleChange}
        options={{
          minimap: { enabled: false },
          scrollBeyondLastLine: false,
          fontSize: 14,
          automaticLayout: true,
          scrollbar: { horizontalScrollbarSize: 0, verticalScrollbarSize: 0 },
        }}
      />
      </div>
  );
}