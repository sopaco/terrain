---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

I have enough information to compose the complete agent-facing context.md file now. Let me synthesize all findings:**From Macro Preload:**- Project overview: AI coding assistant's engineering environment management platform, dual-purpose for humans and agents via ACP protocol- Architecture design: C4 Level1/2 diagrams showing four-layer container structure (Desktop App → CLI Tools → Core Library → AI Agent Layer)- Module map: Repository structure with multiple crates (terrain-core, terrain-agent, terrain-cli, etc.), npm packages, Svelte components**From Architecture Doc:**- Detailed container views at C4 Level2/3- Three primary workflows: Project Initialization, Litho Documentation Generation, DeepWiki Q&A- Key architectural patterns: Agent mode, Pipeline mode, File-system-as-database- Tech stack details: Rust + Tokio async runtime, SvelteKit frontend, Node.js tooling**From Directory Structure:**- Multiple Cargo.toml files indicating workspace structure- Skill configurations in preset_skills/- Cross-platform packaging setup via npm packagesNow I'll compose the complete document with all7 required sections, keeping it under14000 characters: