# Ninja Lua API & DSL Cheatsheet

## Lua API Reference

### fs (Filesystem Module)

```lua
-- Read file contents
content = fs.read(path)

-- Write content to file
fs.write(path, content)

-- Append content to file
fs.append(path, content)

-- Remove a file
fs.remove(path)

-- Create directory (recursive)
fs.create_dir(path)

-- List directory contents
entries = fs.read_dir(path)  -- returns array of filenames

-- Check if path exists
exists = fs.exists(path)  -- returns boolean

-- Check if path is directory
is_dir = fs.is_dir(path)  -- returns boolean

-- Check if path is file
is_file = fs.is_file(path)  -- returns boolean
```

### env (Environment Module)

```lua
-- System information
os = env.os      -- Operating system (e.g., "windows", "linux", "macos")
arch = env.arch  -- Architecture (e.g., "x86_64", "aarch64")

-- Get environment variable
value = env.get(key)  -- returns string or nil

-- Set environment variable (unsafe)
env.set(key, value)

-- Remove environment variable (unsafe)
env.remove(key)

-- Get all environment variables
vars = env.vars()  -- returns table

-- Get current working directory
cwd = env.cwd()  -- returns string
```

### shell (Shell Module)

```lua
-- Execute shell command
result = shell.exec(command, admin)
-- command: string - shell command to execute
-- admin: boolean (optional) - run with elevated privileges

-- Returns table with:
-- result.code   - exit code (number)
-- result.stdout - standard output (string)
-- result.stderr - standard error (string)
```

### proc (Process Module)

```lua
-- Spawn a new process
result = proc.spawn(command)
-- Returns table with:
-- result.pid - process ID (number)

-- Kill process by PID
success = proc.kill_pid(pid)  -- returns boolean

-- Kill process by name
success = proc.kill_name(name)  -- returns boolean

-- Process list (currently empty)
processes = proc.list
```

### time (Time Module)

```lua
-- Get current year
year = time.year()

-- Get current month (1-12)
month = time.month()

-- Get current day (1-31)
day = time.day()

-- Get current hour
hour = time.hour(false)  -- 24-hour format, returns number
hour, period = time.hour(true)  -- 12-hour format, returns (number, "AM"/"PM")

-- Get current minute (0-59)
minute = time.minute()

-- Get current second (0-59)
second = time.second()

-- Get formatted timestamp
timestamp = time.now(format)
-- format: string - chrono format string
-- Example: time.now("%Y-%m-%d %H:%M:%S")

-- Sleep for specified seconds
time.sleep(seconds)  -- seconds: number (supports decimals)
```

### json (JSON Module)

```lua
-- Encode table to JSON string
json_string = json.encode(table)

-- Decode JSON string to table
table = json.decode(json_string)
```

### log (Logging Module)

```lua
-- Log at different levels
log.info(message)
log.warn(message)
log.error(message)
log.debug(message)
```

---

## DSL Reference

### Shuriken Management Commands

```bash
# List all shurikens
list

# List shurikens with their states
list state

# Select a shuriken for operations
select <name>

# Start the selected shuriken
start

# Stop the selected shuriken
stop

# Install a new shuriken from file
install <path>

# Deselect current shuriken
exit
```

### Configuration Commands

```bash
# Generate configuration for selected shuriken
configure

# Configure with inline assignments
configure {
  key1 = value1
  key2 = "string value"
  key3 = 123
  key4 = true
}

# Configure multiline block
configure {
  option1 = value1;
  option2 = value2;
  option3 = value3
}

# Set a single configuration value
set <key> <value>

# Get a configuration value
get <key>

# Toggle a boolean configuration value
toggle <key>
```

### Script Execution

```bash
# Execute a Ninja script file
execute <script_path>
```

### HTTP Server

```bash
# Start HTTP API server
http <port>
# Example: http 8080
```

### Help

```bash
# Display help message
help
```

---

## Configuration Value Types

The DSL automatically detects value types:

```bash
# String (with quotes)
set name "value"
set path 'another value'

# String (without quotes)
set name value

# Boolean
set enabled true
set disabled false

# Number (integer)
set port 8080
set count 42
```

---

## Comments

Both single-line comment styles are supported:

```bash
# This is a comment
// This is also a comment

configure {
  option1 = value1  # inline comment
  option2 = value2  // also inline
}
```

---

## Example Scripts

### Lua Script Example

```lua
-- Create a directory and write a file
fs.create_dir("logs")
local timestamp = time.now("%Y-%m-%d_%H-%M-%S")
local logfile = "logs/app_" .. timestamp .. ".log"

fs.write(logfile, "Application started\n")
log.info("Log file created: " .. logfile)

-- Execute system command
local result = shell.exec("echo Hello from shell")
if result.code == 0 then
    log.info("Command output: " .. result.stdout)
end

-- Check environment
local os_name = env.os
log.info("Running on: " .. os_name)
```

### DSL Script Example

```bash
# Install and configure a shuriken
install ./my-shuriken-linux-x86_64.shuriken

select my-shuriken

configure {
  port = 8080
  host = "localhost"
  debug = true
  workers = 4
}

# Or set individual values
set timeout 30
set log_level "info"

# Start the shuriken
start

# Check status
list state
```

### Combined Example

```bash
# Main configuration
select webserver

configure {
  port = 8080
  ssl = true
  workers = 4
}

# Execute custom Lua setup script
execute ./setup.ns

# Start the server
start
```

---

## Notes

- All filesystem paths are resolved relative to the configured working directory
- Path-like commands in `proc.spawn` are resolved properly on both Windows and Unix
- Windows: Uses PowerShell for shell commands
- Unix/Linux: Uses `$SHELL` or defaults to `sh`
- Admin/elevated privileges:
  - Windows: Uses `-Verb RunAs` (PowerShell)
  - Unix: Uses `pkexec` with `--keep-cwd`