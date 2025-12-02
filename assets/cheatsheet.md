### fs module — Filesystem operations
| Function                   | Args             | Returns   | Description                                   |
| -------------------------- | ---------------- | --------- | --------------------------------------------- |
| `fs.read(path)`            | `string`         | `string`  | Read the contents of a file.                  |
| `fs.write(path, content)`  | `string, string` | `nil`     | Overwrite a file with content.                |
| `fs.append(path, content)` | `string, string` | `nil`     | Append content to a file.                     |
| `fs.remove(path)`          | `string`         | `nil`     | Delete a file.                                |
| `fs.create_dir(path)`      | `string`         | `nil`     | Create a directory.                           |
| `fs.read_dir(path)`        | `string`         | `table`   | Returns a table of file names in a directory. |
| `fs.exists(path)`          | `string`         | `boolean` | Check if a path exists.                       |
| `fs.is_dir(path)`          | `string`         | `boolean` | Check if path is a directory.                 |
| `fs.is_file(path)`         | `string`         | `boolean` | Check if path is a file.                      |

---

### env module — Environment variables & system info

| Property / Function   | Args             | Returns         | Description                                           |
| --------------------- | ---------------- | --------------- | ----------------------------------------------------- |
| `env.os`              | —                | `string`        | Operating system (`"windows"`, `"linux"`, `"macos"`). |
| `env.arch`            | —                | `string`        | CPU architecture.                                     |
| `env.get(key)`        | `string`         | `string or nil` | Get env variable.                                     |
| `env.set(key, value)` | `string, string` | `nil`           | Set env variable.                                     |
| `env.remove(key)`     | `string`         | `nil`           | Remove env variable.                                  |
| `env.vars()`          | —                | `table`         | Table of all env variables.                           |
| `env.cwd()`           | —                | `string`        | Current working directory.                            |
| `env.kill_pid(pid)`   | `number`         | `boolean`       | Kill process by PID.                                  |
| `env.kill_name(name)` | `string`         | `boolean`       | Kill process by name.                                 |

---

### shell module — Command execution

| Function                                 | Args                         | Returns   | Description                                                   |
| ---------------------------------------- | ---------------------------- | --------- | ------------------------------------------------------------- |
| `shell.exec(command, detached?, admin?)` | `string, boolean?, boolean?` | `table`   | Execute a shell command. Returns `{code, stdout, stderr}`.    |
| `shell.kill_pid(pid)`                    | `number`                     | `boolean` | Kill a detached process started with `shell.exec(..., true)`. |

>Notes:
> * `detached = true` → process runs in background, returns {pid = number}.
> * `admin = true` → run as administrator/root.

---

### time module — Date/time utilities

| Function              | Args      | Returns                                 | Description                                                           |
| --------------------- | --------- | --------------------------------------- | --------------------------------------------------------------------- |
| `time.year()`         | —         | `number`                                | Current year (UTC).                                                   |
| `time.month()`        | —         | `number`                                | Current month (1–12).                                                 |
| `time.day()`          | —         | `number`                                | Current day of month.                                                 |
| `time.hour(format?)`  | `boolean` | `(number, "AM"/"PM")` or `(number, "")` | Current hour, optionally 12-hour format.                              |
| `time.minute()`       | —         | `number`                                | Current minute.                                                       |
| `time.second()`       | —         | `number`                                | Current second.                                                       |
| `time.now(fmt)`       | `string`  | `string`                                | Formatted current UTC time. Example: `time.now("%Y-%m-%d %H:%M:%S")`. |
| `time.sleep(seconds)` | `number`  | `nil`                                   | Sleep for given seconds (supports fractions).                         |

---

### json module — JSON encode/decode

| Function                   | Args     | Returns  | Description                       |
| -------------------------- | -------- | -------- | --------------------------------- |
| `json.encode(table)`       | `table`  | `string` | Convert Lua table to JSON string. |
| `json.decode(json_string)` | `string` | `table`  | Parse JSON string into Lua table. |


---

### log module — Logging

| Function         | Args     | Description            |
| ---------------- | -------- | ---------------------- |
| `log.info(msg)`  | `string` | Info-level logging.    |
| `log.warn(msg)`  | `string` | Warning-level logging. |
| `log.error(msg)` | `string` | Error-level logging.   |
| `log.debug(msg)` | `string` | Debug-level logging.   |

---

### http module — (Stub/empty)

Currently empty, reserved for future HTTP functions.