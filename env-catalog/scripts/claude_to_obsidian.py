#!/usr/bin/env python3
"""
Pipeline: Claude chats → Obsidian vault

Adds frontmatter, automatic tags, and wikilinks to existing notes.

Usage:
    python3 claude_to_obsidian.py \\
        --export-dir ~/claude-exports \\
        --vault-dir ~/ObsidianVault \\
        --move

Options:
    --dry-run         Show what would happen without modifying anything
    --move            Delete originals after copying (default: copy only)
    --origin          Force origin: code, web, or auto (default: auto)
    --no-wikilinks    Disable wikilink insertion

Customize KEYWORD_TAG_MAP below to match your stack and projects.
"""

import argparse
import re
from datetime import datetime
from pathlib import Path

# ============================================================================
# CONFIGURATION — adapt KEYWORD_TAG_MAP to your stack and projects
# ============================================================================
# Maps keywords found in chat content → tags applied to the note.
# Add/remove entries based on what you work with.

KEYWORD_TAG_MAP = {
    # Languages / Frameworks
    "python": "python",
    "javascript": "javascript",
    "typescript": "typescript",
    "react": "react",
    "vue": "vue",
    "angular": "angular",
    "fastapi": "fastapi",
    "django": "django",
    "flask": "flask",
    "nodejs": "nodejs",
    "rust": "rust",
    "go ": "golang",
    "golang": "golang",
    "sql": "sql",

    # AI / ML
    "machine learning": "machine-learning",
    "deep learning": "deep-learning",
    "neural network": "neural-network",
    "transformer": "transformers",
    "llm": "llm",
    "gpt": "llm",
    "claude": "llm",
    "langchain": "langchain",
    "embedding": "embeddings",
    "fine-tun": "fine-tuning",
    "rag": "rag",
    "prompt": "prompt-engineering",
    "computer vision": "computer-vision",
    "nlp": "nlp",
    "pytorch": "pytorch",
    "tensorflow": "tensorflow",
    "hugging face": "huggingface",
    "huggingface": "huggingface",

    # Automation
    "automation": "automation",
    "selenium": "automation",
    "scrapy": "web-scraping",
    "scraping": "web-scraping",
    "cron": "automation",
    "pipeline": "pipeline",

    # Infra / DevOps
    "docker": "docker",
    "kubernetes": "kubernetes",
    "aws": "aws",
    "gcp": "gcp",
    "azure": "azure",
    "linux": "linux",
    "git": "git",
    "api": "api",
    "rest": "api",
    "graphql": "graphql",

    # Backend services
    "supabase": "supabase",
    "firebase": "firebase",
    "postgres": "database",
    "mongodb": "database",
    "redis": "redis",
    "database": "database",

    # Knowledge management
    "obsidian": "obsidian",
    "vault": "obsidian",
    "zettelkasten": "pkm",
    "graphify": "graphify",

    # Generic
    "debug": "debugging",
    "error": "debugging",
    "refactor": "refactoring",
    "test": "testing",
    "deploy": "deploy",

    # Add your own project-specific keywords here, e.g.:
    # "my-app": "my-app",
    # "client-name": "client-work",
}

# Short keywords that should match only as whole words (avoid false positives)
SHORT_KEYWORDS = {"sql", "llm", "gpt", "rag", "nlp", "git", "api", "rest", "aws", "gcp"}

# ============================================================================
# CORE LOGIC — usually no need to edit below this line
# ============================================================================


def detect_origin(filepath: Path, content: str) -> str:
    """Detect whether the chat is from Claude Code or Claude Web."""
    path_str = str(filepath).lower()
    if any(k in path_str for k in ("code", "claude-code", ".claude/projects")):
        return "code"

    code_indicators = ["```bash", "$ claude", "terminal", "command line"]
    hits = sum(1 for ind in code_indicators if ind.lower() in content.lower())
    if hits >= 2:
        return "code"

    return "web"


def extract_tags(content: str) -> list[str]:
    """Extract tags via keyword matching in the content."""
    content_lower = content.lower()
    found: set[str] = set()

    for keyword, tag in KEYWORD_TAG_MAP.items():
        if keyword in SHORT_KEYWORDS:
            if re.search(rf"\b{re.escape(keyword)}\b", content_lower):
                found.add(tag)
        else:
            if keyword in content_lower:
                found.add(tag)

    return sorted(found)


def strip_existing_frontmatter(content: str) -> tuple[dict[str, str], str]:
    """Remove existing frontmatter and return parsed fields + body."""
    existing: dict[str, str] = {}
    body = content

    if content.startswith("---\n"):
        end = content.find("\n---\n", 4)
        if end != -1:
            fm_block = content[4:end]
            body = content[end + 5:]
            for line in fm_block.split("\n"):
                if ":" in line and not line.startswith("  ") and not line.startswith("-"):
                    key, _, val = line.partition(":")
                    existing[key.strip()] = val.strip()

    return existing, body


def build_frontmatter(
    title: str,
    tags: list[str],
    origin: str,
    created: str,
) -> str:
    """Build the YAML frontmatter block."""
    now = datetime.now().strftime("%Y-%m-%d %H:%M")
    all_tags = ["chat-import"] + [t for t in tags if t != "chat-import"]
    tags_yaml = "\n".join(f"  - {t}" for t in all_tags)

    return f"""---
title: "{title}"
tags:
{tags_yaml}
source: claude
origin: {origin}
created: {created}
processed: {now}
status: imported
type: chat
---

"""


def collect_vault_notes(vault_dir: Path) -> list[str]:
    """Collect names of all .md notes in the vault (without extension)."""
    notes: list[str] = []
    for md in vault_dir.rglob("*.md"):
        rel = md.relative_to(vault_dir)
        if any(p.startswith(".") for p in rel.parts):
            continue
        name = md.stem
        if len(name) >= 4:
            notes.append(name)

    # Sort by length descending so longer names match first
    notes.sort(key=lambda n: -len(n))
    return notes


def insert_wikilinks(body: str, vault_notes: list[str]) -> str:
    """Insert [[wikilinks]] for vault notes on first occurrence."""
    # Split body into segments: inside vs outside code blocks
    parts = re.split(r"(```[\s\S]*?```|`[^`\n]+`)", body)
    linked: set[str] = set()

    for i, part in enumerate(parts):
        # Skip code blocks
        if part.startswith("`"):
            continue

        for note in vault_notes:
            if note in linked:
                continue

            # Case-insensitive search with word boundary
            pattern = rf"(?<!\[\[)\b({re.escape(note)})\b(?!\]\])"
            match = re.search(pattern, part, re.IGNORECASE)
            if match:
                parts[i] = (
                    part[: match.start()]
                    + f"[[{note}]]"
                    + part[match.end():]
                )
                part = parts[i]
                linked.add(note)

    return "".join(parts)


def process_file(
    filepath: Path,
    vault_dir: Path,
    vault_notes: list[str],
    origin_override: str | None,
    no_wikilinks: bool,
    dry_run: bool,
    move: bool,
) -> dict:
    """Process a single exported .md file."""
    content = filepath.read_text(encoding="utf-8", errors="replace")
    _, body = strip_existing_frontmatter(content)

    origin = origin_override or detect_origin(filepath, content)
    tags = extract_tags(content)
    title = filepath.stem

    # Created date = original file's modification date
    mtime = datetime.fromtimestamp(filepath.stat().st_mtime)
    created = mtime.strftime("%Y-%m-%d")

    # Wikilinks
    if not no_wikilinks:
        body = insert_wikilinks(body, vault_notes)

    # Build output
    frontmatter = build_frontmatter(title, tags, origin, created)
    output = frontmatter + body

    dest_dir = vault_dir / "chats" / origin
    dest = dest_dir / filepath.name

    result = {
        "source": str(filepath),
        "dest": str(dest),
        "origin": origin,
        "tags": tags,
        "title": title,
    }

    if dry_run:
        return result

    dest_dir.mkdir(parents=True, exist_ok=True)
    dest.write_text(output, encoding="utf-8")

    if move:
        filepath.unlink()

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Import Claude chats into an Obsidian vault"
    )
    parser.add_argument(
        "--export-dir", required=True, type=Path,
        help="Directory containing exported .md files"
    )
    parser.add_argument(
        "--vault-dir", required=True, type=Path,
        help="Path to the Obsidian vault"
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Show what would happen without modifying anything"
    )
    parser.add_argument(
        "--move", action="store_true",
        help="Delete originals after copying (default: copy only)"
    )
    parser.add_argument(
        "--origin", choices=["code", "web", "auto"], default="auto",
        help="Force origin or auto-detect (default: auto)"
    )
    parser.add_argument(
        "--no-wikilinks", action="store_true",
        help="Disable wikilink insertion"
    )

    args = parser.parse_args()

    if not args.export_dir.exists():
        print(f"ERROR: Export directory not found: {args.export_dir}")
        return

    if not args.vault_dir.exists():
        print(f"ERROR: Vault not found: {args.vault_dir}")
        return

    # Collect vault notes
    vault_notes = collect_vault_notes(args.vault_dir)
    print(f"Vault notes found: {len(vault_notes)}")

    # Find .md files in export directory
    md_files = sorted(args.export_dir.rglob("*.md"))
    if not md_files:
        print("No .md files found in export directory.")
        return

    print(f"Files to process: {len(md_files)}")
    if args.dry_run:
        print("=== DRY RUN — nothing will be modified ===\n")

    results = []
    origin_override = args.origin if args.origin != "auto" else None

    for f in md_files:
        result = process_file(
            f, args.vault_dir, vault_notes,
            origin_override, args.no_wikilinks,
            args.dry_run, args.move,
        )
        results.append(result)

        tags_str = ", ".join(result["tags"]) if result["tags"] else "(no tags)"
        prefix = "[DRY] " if args.dry_run else ""
        print(f'{prefix}✓ {result["title"]}')
        print(f'  Origin: {result["origin"]} | Tags: {tags_str}')
        print(f'  → {result["dest"]}')

    # Summary
    code_count = sum(1 for r in results if r["origin"] == "code")
    web_count = sum(1 for r in results if r["origin"] == "web")
    all_tags: set[str] = set()
    for r in results:
        all_tags.update(r["tags"])

    print(f"\n{'═' * 50}")
    print(f"SUMMARY")
    print(f"{'═' * 50}")
    print(f"Total processed: {len(results)}")
    print(f"  Code: {code_count}")
    print(f"  Web:  {web_count}")
    print(f"Unique tags found: {len(all_tags)}")
    if all_tags:
        print(f"  {', '.join(sorted(all_tags))}")
    if args.dry_run:
        print("\n⚠ Dry run — no files were modified.")


if __name__ == "__main__":
    main()
