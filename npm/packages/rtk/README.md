# @terrain-ai/rtk

npm wrapper for the [RTK](https://github.com/rtk-ai/rtk) CLI — compresses verbose shell output before it reaches an LLM.

## Install / run

```bash
bunx @terrain-ai/rtk git status
npx @terrain-ai/rtk cargo test
```

Prefer the Terrain-managed copy when available: `~/.terrain/bin/rtk`.

## Platform packages

Native binaries ship in optional platform packages (e.g. `@terrain-ai/rtk-darwin-arm64`). If your OS/arch is unsupported, install [Terrain](https://github.com/sopaco/terrain) desktop or use env integration.

## Publish (maintainers)

From `npm/` after staging binaries:

```bash
npm run prepare
npm publish -w @terrain-ai/rtk-darwin-arm64 --access public
npm publish -w @terrain-ai/rtk --access public
```
