#!/bin/bash
# Sync Claude → Obsidian
# Run manually or via cron.
#
# Setup:
#   1. Edit VAULT_DIR below to point to your Obsidian vault
#   2. Make executable: chmod +x sync_claude_obsidian.sh
#   3. Test once manually: ./sync_claude_obsidian.sh
#   4. (Optional) Add to cron to run daily at 10pm:
#      (crontab -l 2>/dev/null; echo "0 22 * * * $HOME/scripts/sync_claude_obsidian.sh") | crontab -

# ============================================================================
# CONFIGURATION — edit these paths
# ============================================================================

VAULT_DIR="$HOME/ObsidianVault"          # path to your Obsidian vault
EXPORT_DIR="$HOME/claude-exports"        # staging area for exported chats
SCRIPT_DIR="$HOME/scripts"               # where this script and the .py live
LOG="$SCRIPT_DIR/claude_obsidian_sync.log"

# ============================================================================

mkdir -p "$EXPORT_DIR/code" "$EXPORT_DIR/web"

echo "[$(date)] Starting sync..." >> "$LOG"

# 1. Export Claude Code chats (requires claude-conversation-extractor)
if command -v claude-extract &> /dev/null; then
    claude-extract --all --output "$EXPORT_DIR/code" 2>> "$LOG"
    echo "[$(date)] Claude Code chats exported" >> "$LOG"
else
    echo "[$(date)] claude-extract not found — install with:" >> "$LOG"
    echo "[$(date)]   pip install claude-conversation-extractor" >> "$LOG"
    echo "[$(date)] Skipping Code export" >> "$LOG"
fi

# 2. Process and send to vault
#    Web chats should be manually exported via browser extension
#    and dropped into $EXPORT_DIR/web/ — they'll be processed too.
python3 "$SCRIPT_DIR/claude_to_obsidian.py" \
    --export-dir "$EXPORT_DIR" \
    --vault-dir "$VAULT_DIR" \
    --move 2>> "$LOG"

echo "[$(date)] Sync completed" >> "$LOG"
echo "" >> "$LOG"
