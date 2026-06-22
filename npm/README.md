# Terrain npm distribution

Optional **npm / bunx / npx** channel for agent-facing CLIs when `~/.terrain/bin/` is not available.

| Package | Binary | Platform (v0.1) |
|---------|--------|-----------------|
| `@terrain-ai/rtk` | `rtk` | via `@terrain-ai/rtk-darwin-arm64` |
| `@terrain-ai/cli` | `terrain` | via `@terrain-ai/cli-darwin-arm64` |

Upstream RTK: [rtk-ai/rtk](https://github.com/rtk-ai/rtk). Terrain CLI: `crates/terrain-cli`.

## Layout

```
npm/
  packages/
    rtk/                  # meta package — Node shim → platform binary
    rtk-darwin-arm64/     # native rtk binary (gitignored, staged before publish)
    cli/
    cli-darwin-arm64/
  scripts/
    prepare-binaries.mjs  # copy from packages/rtk + cargo-built terrain-cli
    write-shims.mjs
    sync-version.mjs      # align versions with Cargo workspace
```

## Maintainer workflow

```bash
cd npm
npm install
npm run prepare          # write shims + build terrain-cli + stage binaries
npm run version:sync     # after bumping Cargo workspace version

# First-time: create @terrain-ai scope on npmjs.com, npm login

npm publish -w @terrain-ai/rtk-darwin-arm64 --access public
npm publish -w @terrain-ai/rtk --access public
npm publish -w @terrain-ai/cli-darwin-arm64 --access public
npm publish -w @terrain-ai/cli --access public
```

Publish **platform packages before** meta packages so `optionalDependencies` resolve.

## Local smoke test (without publishing)

```bash
cd npm
npm run prepare
node packages/rtk/bin/rtk.js --version
node packages/cli/bin/terrain.js --help
```

Or link into PATH:

```bash
npm link -w @terrain-ai/rtk
rtk gain
```

## Adding platforms

1. Add `npm/packages/rtk-<platform>/` and `cli-<platform>/` with `os` / `cpu` in `package.json`.
2. Cross-compile binaries into `packages/rtk/<platform>/` and `packages/terrain/<platform>/`.
3. Extend `prepare-binaries.mjs` copy map.
4. Add optionalDependency entries on meta packages.

## Agent skill fallback

Skills reference:

- `bunx @terrain-ai/rtk` when `~/.terrain/bin/rtk` is missing
- `bunx @terrain-ai/cli` when `~/.terrain/bin/terrain` is missing
