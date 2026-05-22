# Ninja GUI: Visual Service Management

A modern, full-featured desktop application for managing shurikens visually. Built with Next.js for the interface, Tauri for the desktop runtime, and backed by the Ninja core engine.

## What's Inside

A responsive web-based UI that brings the full power of Ninja to your desktop without the terminal. Manage services, configure applications, view logs, and orchestrate complex workflows through an intuitive interface.

## Tech Stack

- **Frontend**: Next.js with React and TypeScript
- **Desktop Runtime**: Tauri for lightweight, cross-platform packaging (also bc it's easier to integrate with rust)
- **State Management**: Zustand. I'm not in the mood to deal with Redux's bull****
- **UI Components**: shadcn. wouldn't have it otherwise
- **Package Manager**: pnpm because using npm should be a crime

## Features

- Dashboard view of all installed shurikens and their states
- One-click service start/stop/restart controls
- Configuration management with visual editors
- Log viewer with real-time updates (hopefully)
- Backup and restore functionality (pretty primitive ik im working on it)
- Armory view for browsing and installing packages
- Export and import capabilities

## Quick Start

### Prerequisites

* Node.js 18+ (latest preferred)
* pnpm
* Rust (duh)

### Installation

```bash
cd GUI
pnpm install
```

### Development

Run the development server with hot reload:

```bash
pnpm dev
```

This starts the Next.js development server on `http://localhost:3000` (when running the web version).

For the bundled Tauri app in development mode:

```bash
pnpm tauri dev
```
or 

```bash
cargo tauri dev
```

if you installed tauri with cargo

### Build

Compile the desktop application for your platform:

```bash
# Build optimized Next.js production bundle
pnpm build

# Create Tauri desktop application
pnpm tauri build # like in the dev part you can use cargo tauri build if you installed it with cargo.
```

The resulting application bundle will be in `../target/release/bundle/` if not cross-compiled, else it will be outputted to `../target/[target-triple]/release/bundle`.

## Project Structure

```
GUI/
├── app/                 # Next.js app directory and pages
├── components/
│   ├── pages/          # Full-page components (Dashboard, Config, etc.)
│   └── ui/             # Reusable UI components
├── hooks/              # Custom React hooks like useShuriken and useConfig
├── lib/                # Utilities, types, and helpers
├── public/             # Static assets (why is this here again?)
└── src-tauri/          # Tauri application configuration and build settings
```

## Component Organization

### Pages

Located in `components/pages/`:
* **Dashboard**: Overview of all services and their current states
* **Armory**: Browse, search, and install shuriken packages
* **Config**: System configuration and settings
* **Logs**: Real-time log viewer with filtering
* **Backup**: Backup creation and restoration
* **Tools**: Development tools for package creation
* **Developer**: Advanced development interface

### UI Components (everything that displays something is dumped here)

Located in `components/ui/`:
* Form components: Input, Select, Switch, Button
* Layout components: Card, Dialog, Accordion
* Data display: Table, Badge, Progress
* Navigation: Tabs, Menubar, Context menu
* Specialized: ArrayEditor, ArmoryCard, AnimatedTabs

## Hooks

Custom hooks for common functionality:

* `useShuriken()` - Core Ninja API access
* `useMobile()` - Responsive design detection (shadcn ui)
* `useOutsideClick()` - Click-outside detection for dropdowns (shadcn ui)
* `useConfig()` - Configuration management

## Configuration

The application configuration is centralized in:
- `lib/config.ts` - Runtime configuration
- `lib/types.ts` - TypeScript type definitions
- `src-tauri/tauri.conf.json` - Tauri application settings

## Styling

The GUI uses Tailwind CSS with a custom configuration:

```bash
# Configuration: tailwind.config.ts
# PostCSS setup: postcss.config.mjs
```

Custom colors, spacing, and component styles are defined in `app/globals.css`.

## Tauri Integration

The `src-tauri/` directory contains the native Rust backend:

* **build.rs**: Build script for native compilation
* **tauri.conf.json**: Application metadata and capabilities
* **capabilities/**: Permission declarations
* **src/**: Rust backend code for Tauri commands

### Available Tauri Commands

Custom commands bridge the frontend to the Ninja core:

- `invoke_ninja_command()` - Execute Ninja CLI commands
- `get_shuriken_status()` - Query service status
- `start_shuriken()` - Start a service
- `stop_shuriken()` - Stop a service
- And more...

## Development Tips

### Hot Reload

The development build automatically reloads when you edit JavaScript/TypeScript files. For Rust changes, the Tauri dev server will restart.

### TypeScript

The project is fully typed. Run type checking:

```bash
# Through Next.js
pnpm build
```

### Debugging

* Browser DevTools: Press `F12` in development mode
* Tauri logs: Check the console output or system logs

## Performance

- Next.js handles static optimization and code splitting
- Tauri provides a lightweight runtime compared to full Electron apps
- UI components use React.memo for preventing unnecessary re-renders

## Cross-Platform Support

The application targets:
* Linux (x86_64, ARM64)
* macOS (x86_64, Apple Silicon)
* Windows (x86_64)
* FreeBSD (in another timeline maybe. just make a flux capacitor if you want it so badly)

Platform-specific bindings in Tauri handle system calls appropriately.

## Building for Distribution

```bash
# Create optimized production build
pnpm build

# Bundle as desktop application
pnpm tauri build

# Result in src-tauri/target/release/bundle/:
# - macOS: .dmg installer and .app bundle
# - Linux: .appimage, .deb and .rpm
# - Windows: .msi or .exe installer. wix/nsis respectively
```

## Testing

```bash
# Run linting
pnpm lint

# Type checking is integrated into the build. i believe
```

## Documentation

- [Next.js Documentation](https://nextjs.org/docs)
- [Tauri Documentation](https://tauri.app/docs/)
- [Radix UI Documentation](https://www.radix-ui.com/docs/primitives/overview/introduction)

## Contributing

The GUI welcomes contributions. When working on new features:
1. Keep components reusable and well-documented
2. Maintain TypeScript strictness
3. Follow the existing styling conventions
4. Test across different screen sizes

Thanks for helping make Ninja GUI better. luv u ❤️