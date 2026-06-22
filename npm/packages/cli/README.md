# @terrain-ai/cli

npm wrapper for the **Terrain CLI** — repository scanning, `.terrain/` knowledge assets, and `terrain tools` for ACP integrators.

## Install / run

```bash
bunx @terrain-ai/cli --help
bunx @terrain-ai/cli tools list-projects
bunx @terrain-ai/cli assets pack-agent .
```

Prefer the Terrain-managed copy when available: `~/.terrain/bin/terrain`.

## Platform packages

Native binaries ship in optional platform packages (e.g. `@terrain-ai/cli-darwin-arm64`).

## Publish (maintainers)

From `npm/` after building the Rust CLI:

```bash
npm run prepare
npm publish -w @terrain-ai/cli-darwin-arm64 --access public
npm publish -w @terrain-ai/cli --access public
```
