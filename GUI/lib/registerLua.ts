import { Monaco } from "@monaco-editor/react";

type IWordAtPosition = {
  word: string
};

function registerFsModule(word: IWordAtPosition | null) {
    if (!word) return;

    if (word.word === 'fs.read_file' || word.word === 'read_file') {
      return {
       contents: [
         { value: '**fs.read_file(path: string) -> string**' },
         { value: 'Reads the entire file into a string.' }
       ]
      };
    }

    if (word.word === 'fs.write_file' || word.word === 'write_file') {
      return {
       contents: [
         { value: '**fs.write_file(path: string, content: string)**' },
         { value: 'Creates or overwrites a file with content.' }
       ]
      };
    }

    if (word.word === 'fs.append_file' || word.word === 'append_file') {
      return {
       contents: [
         { value: '**fs.append_file(path: string, content: string)**' },
         { value: 'Appends to a file.' }
       ]
      };
    }

    if (word.word === 'fs.remove_file' || word.word === 'remove_file') {
      return {
       contents: [
         { value: '**fs.remove_file(path: string)**' },
         { value: 'Deletes the file..' }
       ]
      };
    }

    if (word.word === 'fs.create_dir' || word.word === 'create_dir') {
      return {
       contents: [
         { value: '**fs.create_dir(path: string)**' },
         { value: 'Creates a directory (recursive).' }
       ]
      };
    }

    if (word.word === 'fs.list_dir' || word.word === 'list_dir') {
      return {
       contents: [
         { value: '**fs.list_dir(path: string) -> { string }**' },
         { value: 'Returns a table of file/directory names.' }
       ]
      };
    }

    if (word.word === 'fs.remove_dir' || word.word === 'remove_dir') {
      return {
       contents: [
         { value: '**fs.remove_dir(path: string)**' },
         { value: 'Removes/deletes a directory' }
       ]
      };
    }

    if (word.word === 'fs.exists' || word.word === 'exists') {
      return {
       contents: [
         { value: '**fs.exists(path: string) -> boolean**' },
         { value: 'Returns true if file or directory exists.' }
       ]
      };
    }

    if (word.word === 'fs.is_dir' || word.word === 'is_dir') {
      return {
       contents: [
         { value: '**fs.is_dir(path: string) -> boolean**' },
         { value: 'Returns true if path is a directory.' }
       ]
      };
    }

    if (word.word === 'fs.is_file' || word.word === 'is_file') {
      return {
       contents: [
         { value: '**fs.is_file(path: string) -> boolean**' },
         { value: 'Returns true if path is a file.' }
       ]
      };
    }
}

function registerTimeModule(word: IWordAtPosition | null) {
    if (!word) return;

    if (word.word === 'time.month' || word.word === 'month') {
      return {
       contents: [
         { value: '**time.month() -> number**' },
         { value: 'Returns current month.' }
       ]
      };
    }

    if (word.word === 'time.year' || word.word === 'year') {
      return {
       contents: [
         { value: '**time.year() -> number**' },
         { value: 'Returns current year.' }
       ]
      };
    }

    if (word.word === 'time.day' || word.word === 'day') {
      return {
       contents: [
         { value: '**time.day() -> number**' },
         { value: 'Returns current day.' }
       ]
      };
    }

    if (word.word === 'time.hour' || word.word === 'hour') {
      return {
       contents: [
         { value: '**time.hour(format: boolean) -> number**' },
         { value: 'Returns current hour but either military time or AM/PM based on the format bool.' }
       ]
      };
    }

    if (word.word === 'time.minute' || word.word === 'minute') {
      return {
       contents: [
         { value: '**time.minute() -> number**' },
         { value: 'Returns current minute.' }
       ]
      };
    }

    if (word.word === 'time.now' || word.word === 'now') {
      return {
       contents: [
         { value: '**time.now(format: string) -> number**' },
         { value: 'Returns current time completely based on the format, e.g %d/%M/%Y-%h:%m:%s..' }
       ]
      };
    }

    if (word.word === 'time.sleep' || word.word === 'sleep') {
      return {
       contents: [
         { value: '**time.sleep(miliseconds: number) -> number**' },
         { value: 'Blocks for given miliseconds.' }
       ]
      };
    }
}

function registerJSONModule(word: IWordAtPosition | null) {
  if (!word) return;
  
  if (word.word === 'json.encode' || word.word === 'encode') 
    return {
     contents: [
       { value: '**json.encode(table) -> string**' },
       { value: 'Encodes Lua table to JSON string.' }
    ]
  };

  if (word.word === 'json.decode' || word.word === 'decode') 
    return {
     contents: [
       { value: '**json.decode(json_string: string) -> table**' },
       { value: 'Decodes JSON string to Lua table.' }
    ]
  };
}

function registerLogModule(word: IWordAtPosition | null) {
  if (!word) return;
  
  if (word.word === 'log.info' || word.word === 'info') 
    return {
     contents: [
       { value: '**log.info(message: string) -> string**' },
       { value: 'Logs content at info level.' }
    ]
  };

  if (word.word === 'log.warn' || word.word === 'warn') 
    return {
     contents: [
       { value: '**log.warn(message: string) -> table**' },
       { value: 'Logs content at warn level.' }
    ]
  };

  if (word.word === 'log.error' || word.word === 'error') 
    return {
     contents: [
       { value: '**log.error(message: string) -> table**' },
       { value: 'Logs content at error level.' }
    ]
  };

  if (word.word === 'log.debug' || word.word === 'debug') 
    return {
     contents: [
       { value: '**log.debug(message: string) -> table**' },
       { value: 'Logs content at debug level.' }
    ]
  };
}

function registerHttpModule(word: IWordAtPosition | null) {
  if (!word) return;
  
  if (word.word === 'http.fetch' || word.word === 'fetch') 
    return {
     contents: [
       { value: '**http.fetch(url: string, headers?: table, method: string) -> { status: integer, body: string, headers: table }**' },
       { value: 'Performs HTTP request.' }
    ]
  };
}

function registerShellModule(word: IWordAtPosition | null) {
    if (!word) return;

    if (word.word === 'shell.exec' || word.word === 'exec') {
      return {
       contents: [
         { value: '**shell.exec(command: string) -> { code: integer, stdout: string, stderr: string }**' },
         { value: 'Executes shell command via `sh -c` (Unix) or `cmd /C` (Windows).\nReturns exit code, stdout, and stderr.' }
       ]
      };
    }
}

function registerEnvModule(word: IWordAtPosition | null) {
    if (!word) return;

    if (word.word === 'env.os' || word.word === 'os') {
      return {
       contents: [
         { value: '**env.os: string**' },
         { value: 'The name of the current operating system.' }
       ]
      };
    }

    if (word.word === 'env.arch' || word.word === 'arch') {
      return {
       contents: [
         { value: '**env.arch: string**' },
         { value: 'The name of the current architecture.' }
       ]
      };
    }

    if (word.word === 'env.get' || word.word === 'get') {
      return {
       contents: [
         { value: '**env.get(key: string) -> string|nil**' },
         { value: 'Returns value of environment variable or nil.' }
       ]
      };
    }

    if (word.word === 'env.set' || word.word === 'set') {
      return {
       contents: [
         { value: '**env.set(key: string, value: string)**' },
         { value: 'Sets an environment variable.' }
       ]
      };
    }

    if (word.word === 'env.remove' || word.word === 'remove') {
      return {
       contents: [
         { value: '**env.remove(key: string)**' },
         { value: 'Removes an environment variable..' }
       ]
      };
    }

    if (word.word === 'env.vars' || word.word === 'vars') {
      return {
       contents: [
         { value: '**env.vars() -> { { key: string, value:string }, ... }**' },
         { value: 'Returns a table of all env variables.' }
       ]
      };
    }
}

export default function RegisterLua(monaco: Monaco){

    monaco.languages.register({ id: 'ninjascript' });
    
      // Syntax highlighting
      monaco.languages.setMonarchTokensProvider('ninjascript', {
        tokenizer: {
          root: [
            [/[a-zA-Z_]\w*/, {
              cases: {
                'function|end|if|then|else|elseif|for|in|do|while|repeat|until|local|return|break|and|or|not': 'keyword',
                '@default': 'identifier'
              }
            }],
            [/\d+/, 'number'],
            [/"([^"\\]|\\.)*$/, 'string.invalid'],
            [/"/, { token: 'string.quote', bracket: '@open', next: '@string' }],
            [/[{}()\[\]]/, '@brackets'],
            [/--.*/, 'comment'],
            [/[=+\-*/%^#<>~:.]/, 'operator'],
          ],
          string: [
            [/[^\\"]+/, 'string'],
            [/\\./, 'string.escape'],
            [/"/, { token: 'string.quote', bracket: '@close', next: '@pop' }]
          ]
        }
      });

      // Hover documentation
      monaco.languages.registerHoverProvider('ninjascript', {
        provideHover(model, position) {
          const word = model.getWordAtPosition(position);
          if (!word) return;
          
          registerFsModule(word);
          registerEnvModule(word);
          registerShellModule(word);
          registerTimeModule(word);
          registerJSONModule(word);
          registerHttpModule(word);
          registerLogModule(word);

          if (word.word === 'print') {
            return {
              contents: [
                { value: '**print(value)**' },
                { value: 'Prints a value to the console output.' }
              ]
            };
          }
        }
      });
}