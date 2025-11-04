# Scotty Frontend

SvelteKit-based web interface for Scotty, providing a modern UI for managing Docker Compose applications.

## Technology Stack

- **SvelteKit 2**: Single-page application (SPA) with static adapter
- **TypeScript**: Type-safe development with types generated from Rust
- **TailwindCSS 4.x**: Utility-first CSS framework
- **DaisyUI 5**: Component library built on Tailwind
- **Iconify**: Icon system with Phosphor icon set
- **WebSocket**: Real-time communication for logs, shell sessions, and task output

## Key Features

- **Type-Safe API Integration**: TypeScript types automatically generated from Rust backend types (via ts-rs)
- **Real-Time Updates**: WebSocket support for live logs, shell sessions, and task output streaming
- **OAuth 2.0 Authentication**: Device flow and web flow support
- **Dashboard**: Application overview and management
- **Task Monitoring**: Real-time task execution tracking
- **Tight Backend Coupling**: No API versioning - frontend and backend evolve together

## Development

### Prerequisites

1. Node.js (v20 or higher recommended)
2. Scotty backend running (see root README.md)
3. Traefik running for local development (see `apps/traefik`)

### Install Dependencies

```bash
npm install
# or
bun install
```

### Development Server

```bash
npm run dev
```

The dev server runs on `http://localhost:5173` and proxies API requests to the Scotty backend at `http://localhost:21342`:

- `/api/*` → Backend REST API
- `/ws/*` → Backend WebSocket endpoints
- `/rapidoc` → OpenAPI documentation

### Type Checking

```bash
npm run check

# Watch mode
npm run check:watch
```

### Linting and Formatting

```bash
# Check formatting and lint
npm run lint

# Fix formatting and lint issues
npm run lint+fix

# Format code
npm run format
```

## Building for Production

```bash
npm run build
```

This creates a static build in the `build/` directory that can be served by any static file server or embedded directly in the Scotty binary.

### Preview Production Build

```bash
npm run preview
```

## Project Structure

```
src/
├── routes/              # SvelteKit routes
│   ├── dashboard/       # Application dashboard
│   ├── login/           # Login page
│   ├── oauth/           # OAuth callback handler
│   └── tasks/           # Task monitoring
├── stores/              # Svelte stores
│   ├── webSocketStore.ts  # WebSocket connection management
│   └── userStore.ts       # User authentication state
├── generated/           # Auto-generated TypeScript types from Rust
│   ├── index.ts
│   └── *.ts             # Generated type definitions
└── lib/                 # Shared components and utilities
```

## Generated Types

TypeScript types are automatically generated from Rust types using `ts-rs`. To regenerate types after backend changes:

```bash
# In the root directory
cargo run --bin ts-generator
```

This updates all files in `src/generated/`.

## Configuration

### Vite Proxy Configuration

The development server proxies requests to the backend. See `vite.config.ts` for proxy configuration.

### Backend Integration

The frontend expects the Scotty backend to be running at `http://localhost:21342` during development. Configure this in `vite.config.ts` if your backend runs on a different port.

## Deployment

The frontend builds to a static SPA with fallback routing (`index.html` serves all routes). Deploy options:

1. **Standalone**: Serve the `build/` directory with any static file server
2. **Embedded**: The Scotty binary can serve the frontend directly (built files copied to backend)

## Development Notes

- The frontend is **tightly coupled** with the backend API - breaking changes are acceptable
- No API versioning or backwards compatibility required
- All authentication state is managed in `userStore.ts`
- WebSocket connections are managed in `webSocketStore.ts`
- Uses SvelteKit's `$app` modules for routing and environment access

## Browser Support

Modern browsers with ES2020+ support. The build targets:

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

## Contributing

When making changes:

1. Ensure types are up to date: `cargo run --bin ts-generator`
2. Run type checking: `npm run check`
3. Run linting: `npm run lint`
4. Test in development mode: `npm run dev`
5. Verify production build: `npm run build && npm run preview`
