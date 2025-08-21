import React, { useEffect, useRef } from 'react';
import { Editor, useMonaco } from '@monaco-editor/react';
import { Button } from '../ui/button';
import { PlayIcon } from 'lucide-react';
import { open, save } from '@tauri-apps/plugin-dialog';
import RegisterLua from '@/lib/registerLua';
import { Loading } from '../ui/loading';
// import RegisterLua from "@/lib/registerLua"

export default function Scripting() {
  const editorRef = useRef(null);
  const [value, setValue] = React.useState<string>('');
  const [mounted, setMounted] = React.useState(false);
  const luaRegistered = useRef(false);
  const monaco = useMonaco();
  const monacoColorScheme = {
  "base": "vs-dark",
  "inherit": true,
  "rules": [
    {
      "token": "keyword",
      "foreground": "a855f7"
    }
  ],
  "colors": {
    "editor.background": "#101012", // zinc-900
    "editor.foreground": "#f4f4f5", // zinc-100
    "editor.lineHighlightBackground": "#27272a", // zinc-800
    "editorCursor.foreground": "#f4f4f5", // zinc-100
    "editor.selectionBackground": "#3f3f46", // zinc-700
    "editor.inactiveSelectionBackground": "#3f3f4655", // zinc-700 with opacity
    "editor.lineHighlightBorder": "#27272a", // zinc-800
    "editorGutter.background": "#101012", // zinc-900
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

  useEffect(() => {
    if (mounted && monaco && !luaRegistered.current) {
      monaco.editor.addKeybindingRule({
        keybinding: monaco.KeyCode.F1,
        command: "none"
      });

      RegisterLua(monaco);
      luaRegistered.current = true;
    }
  }, [mounted, monaco]);



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

  return (
    <div className='h-full flex flex-col align-center justify-center gap-4'>
      <div className='w-full flex justify-center'>

      <div className='w-[95%] flex justify-between'>
        <div>
        <Button onClick={ () => save({ filters: [{ name: "Ninjascript file", extensions: ["ns"]}, {name:"All files", extensions: ["*"]}]})} variant={"outline"} className='mr-4'>
          Save script as a file
        </Button>
        <Button onClick={ () => open({filters: [ {name: "Ninjascript file", extensions: ["ns"]}]})} variant={"outline"}>
          Open script file
        </Button>
        </div>
        <Button>
          <PlayIcon className='h-4 w-4'/>
          Run script
        </Button>
      </div>
      </div>

      <div className='h-full w-full flex justify-center'>
      <Editor
        className='rounded-md overflow-hidden border-0.5 border-foreground/50 '
        height="90%"
        width="95%"
        language="ninjascript"
        value={value}
        beforeMount={handleThemeChange}
        theme="zinc-dark"
        loading={<Loading className=''/>}
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
    </div>
  );
}