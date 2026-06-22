---
name: sdd-workflow-skill
description: Terrain SDD standardized workflow — requirement clarification, technical design, code generation, and code review.
---

# SDD Standardized Workflow

Software Design Driven (SDD) workflow for Terrain. Execute one phase at a time and persist outputs under the local SDD store (`~/.terrain/sdd/{project}/sessions/{id}/outputs/`).

## Environment

| Variable | Purpose |
|----------|---------|
| `TERRAIN_SDD_SKILL` | This skill directory |
| `TERRAIN_SDD_WORKSPACE` | Intermediate workspace (local session dir under `~/.terrain/sdd/`) |
| `TERRAIN_SDD_OUTPUT_DIR` | Phase outputs (`outputs/`) — **not versioned in Git** |
| `TERRAIN_HUMAN_OUTPUT_DIR` | Litho human docs for context |

## Phases

### Phase 1 — Requirement clarification (`1.requirements.md`)

- Clarify goals, scope, user stories, acceptance criteria
- List constraints, dependencies, and open questions
- Do not start technical design in this phase

### Phase 2 — Technical design (`2.tech-design.md`)

- Read `1.requirements.md` and project knowledge (human docs, agent pack)
- Produce architecture decisions, module boundaries, APIs, data model
- Include rollout plan and risks

### Phase 3 — Code generation (`3.implementation.md` + repo changes)

- Implement the approved technical design in the repository
- Follow existing project conventions
- Write implementation notes to `3.implementation.md`

### Phase 4 — Code review (`4.code-review.md`)

- Compare implementation against requirements and technical design
- Review changed files in the repository
- Report findings: Critical / Major / Minor / Suggestions

## Output contract

Write each phase artifact to `TERRAIN_SDD_OUTPUT_DIR` using the filenames above. Use absolute paths when writing outside the repository root.
