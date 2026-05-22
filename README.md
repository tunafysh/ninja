# Ninja: Service Management Evolved

Ninja is a modern, composable service management system that trades configuration files for something far more elegant, **Shurikens**. Whether you're orchestrating microservices, managing system utilities, or deploying complex applications, Ninja provides a unified, intuitive platform to handle it all.

## The Core Philosophy

Instead of wrestling with disparate tools and configuration formats, Ninja introduces **Shurikens**, self-contained, portable packages that represent services and applications. Each Shuriken knows how to manage itself: how to start, how to stop, where to log, and what to configure. This shifts the burden from you to the service itself.

It's service oriented architecture without the ceremony.

## What You Get

* **CLI**: A command-line interface for managing shurikens and orchestrating your services
* **GUI**: A full-featured desktop application built with Next.js and Tauri for those who prefer a visual approach
* **Core Library**: The engine that powers everything, written in Rust for reliability and performance
* **HTTP/GraphQL API**: Integrate Ninja into your CI/CD pipelines and automation workflows
* **Scripting Support**: Embed Lua scripts within shurikens to handle dynamic logic
* **Backup & Recovery**: Built-in mechanisms to snapshot and restore service states
* **FFI Bindings**: Call Ninja from C/C++ or other languages
* **Model Context Protocol Support**: Seamlessly integrate with AI tools and agents

## Quick Start

### Installation

The recommended way to get started depends on your platform and use case:

```bash
# Via cargo (for CLI and core tools)
cargo install --path ./CLI

# Via npm (for the GUI)
cd GUI && pnpm install && pnpm build
```

For detailed setup instructions and platform-specific builds, see the [CLI README](./CLI/README.md), [Core README](./core/README.md), or [GUI README](./GUI/README.md).

### Your First Shuriken

A Shuriken is defined as a `.shuriken` file, a precisely formatted package containing your service definition, startup/shutdown logic, and optional configuration templates and tools.

For comprehensive details on creating and managing Shurikens, check out the [documentation](./assets/docs.md) and [cheatsheet](./assets/cheatsheet.md).

## Project Structure

```
ninja/
├── CLI/              # Command-line interface
├── core/             # Core service management engine
├── GUI/              # Desktop GUI (Next.js + Tauri)
├── HTTP/             # HTTP/GraphQL API server
├── FFI/              # C/C++ bindings
├── MCP/              # Model Context Protocol support
├── tests/            # Integration and unit tests
└── assets/           # Documentation and SDK tools
```

## Architecture Highlights

**Modular Design**: Each component (CLI, GUI, HTTP, FFI) is independent and can be used standalone or in combination.

**Rust Core**: The core service management engine is written in Rust, ensuring memory safety and blazing-fast performance.

**Scriptable with Lua**: Script your shurikens with Lua, a fast, embeddable scripting engine, infused with custom modules by ninja.

**Cross-Platform**: Works on Linux, macOS, and Windows with platform-specific optimizations where needed. (FreeBSD support is on the works.)

**Extensible**: The scripting system and tool ecosystem allow you to customize behavior to match your infrastructure.

## Contributing

We welcome contributions of all shapes and sizes. Whether you're fixing a bug, adding a feature, or improving documentation, your work helps Ninja become more powerful.

Start by checking out the existing work in [CHANGELOG.md](./CHANGELOG.md) to see what's been done and what's in progress.

## License

This project is licensed under the GNU Affero General Public License v3.0. See [LICENSE.md](./LICENSE.md) for details.

## Get Involved

Have questions? Want to show Ninja in action? Found a bug? Head over to the issues or discussions and i'll look into it.

made with love by me❤️
