# CLONE_SPEC — Git panel mirroring GitHub Desktop (behavior only, clean-room)

Source studied: https://github.com/desktop/desktop (MIT, public source) via the
REAP 04 software-and-feature-cloning method: extract *behavior* with evidence,
rebuild with Terrain's own architecture (Rust gitops + Svelte). No code, assets,
or branding copied.

## Evidence (desktop/desktop @ master)

| # | Behavior | Evidence |
|---|----------|----------|
| E1 | Checkboxes are Desktop's **own include set**, not the git index. Committing = `unstageAll` → `stageFiles(repository, selectedFiles)` → `git commit -F <stdin>` | `app/src/lib/git/commit.ts` `createCommit()` — "Clear the staging area, our diffs reflect the difference between the working directory and the last commit" |
| E2 | Per-file working diffs come from `git diff --no-color` (plus `--cached` variants for index-side views) | `app/src/lib/git/diff.ts` (`getWorkingDirectoryDiff`) |
| E3 | Status is parsed from porcelain output into a model with per-file status (Added/Modified/Deleted/Renamed/Conflicted/Untracked) + branch/upstream/ahead-behind + merge/rebase state | `app/src/lib/git/status.ts` + `app/src/lib/status-parser.ts`, `app/src/lib/models/status.ts` |
| E4 | The commit message splits into **summary** (first line, recommended ≤ 72 chars → warns above 50) + **description** (rest) | `app/src/ui/changes/commit-message.tsx` (summary char counters/validation) |
| E5 | History view lists commits (summary, author, relative time) with per-commit changed-file lists and patch views | `app/src/ui/history/{commit-list-item,expandable-commit-summary,file-list}.tsx`, `app/src/lib/git/log.ts` |
| E6 | Toolbar "Fetch origin" button shows ↑ahead / ↓behind counts on the current branch | `app/src/ui/repository.tsx` (fetch component), `app/src/models/branch.ts` `IAheadBehind` |
| E7 | Branch dropdown switches branches; merging is a separate flow targeting the current branch | `app/src/ui/branches/branches-menu.tsx`, `app/src/ui/branches/merge-branch-dialog.tsx` |
| E8 | Conflicted state shows a prominent banner instructing the user to resolve, then re-commit | `app/src/ui/changes/conflicted-files.tsx` + `app/src/lib/git/status.ts` `mergeHeadFound` |

## Requirements derived for Terrain (own architecture)

1. **Status model** — `gitops::status()` (porcelain v2) returns branch, upstream,
   ahead/behind, per-file entry {path, status code, untracked, conflict},
   `merging`. Already implemented (Rust, `porcelain=v2 --branch`).
2. **Include-set commit flow** — the UI owns checked files; backend
   `commit_files(repo, summary, description, files)` does:
   `reset --mixed → add -- <files> → commit` (E1). No persistent staging area.
3. **Diffs** — per-file unified patch (`diff_file`, `commit_diff`), capped at
   512 KB with `truncated` flag (E2).
4. **History** — `history(repo, limit)` newest-first with author/time/summary;
   `commit_file_list` name-status per commit (E5).
5. **Sync** — `fetch` + existing `push` (auto `--set-upstream origin`) +
   `pull --ff-only`; ahead/behind from status (E6).
6. **Branches** — dropdown lists local branches; selection switches via
   `checkout`; merge targets the current branch (E7).
7. **Conflicts** — banner listing conflicted paths when status reports them;
   resolution happens in the user's editor, then include + commit (E8).

## UI contract (Terrain tokens)

- Toolbar: branch dropdown · view pills (Changes |
  History) · Fetch origin (↑↓ badges) · Push · Pull.
- Changes = two-pane: left file list with per-row checkboxes + include-all in
  the header with a count; right pane renders the selected file's unified diff
  (+/- line coloring, hunk headers accented); bottom-left commit box (summary +
  description + "Commit to {branch}").
- History = two-pane: commit list left; selected commit's file list + patch right.
- Empty repo state, binary/untracked placeholder ("no textual diff").

## UNKNOWN / out of scope for v1

- GitHub auth flows, LFS, template repos, proxy config (Desktop-only).
- Rebase/squash/stash/amend/cherry-pick flows (Desktop has them; v1 keeps
  commit/merge/push/pull/checkout only).
- Side-by-side diff (v1 is unified).
- Partial (line-level) staging with hunk/drag selection — v1 commits whole files.