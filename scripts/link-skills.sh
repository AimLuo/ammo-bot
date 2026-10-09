#!/usr/bin/env bash
set -euo pipefail

# Links repo-root my-* skills into the local skill directories used by each
# agent harness:
#   - ~/.claude/skills: Claude Code
#   - ~/.agents/skills: Codex and other Agent Skills-compatible harnesses
#   - ~/.cursor/skills: Cursor
# Each entry is a symlink into this repo, so edits here show up immediately.

REPO="$(cd "$(dirname "$0")/.." && pwd)"
DESTS=("$HOME/.claude/skills" "$HOME/.agents/skills" "$HOME/.cursor/skills")

names=()
srcs=()
shopt -s nullglob
for src in "$REPO"/my-*/; do
  [ -f "${src}SKILL.md" ] || continue
  src="${src%/}"
  names+=("$(basename "$src")")
  srcs+=("$src")
done
shopt -u nullglob

if [ "${#names[@]}" -eq 0 ]; then
  echo "error: no my-* skills found under $REPO" >&2
  exit 1
fi

for DEST in "${DESTS[@]}"; do
  # If $DEST is a symlink that resolves into this repo, we'd end up writing the
  # per-skill symlinks back into the repo. Detect and bail out instead of
  # polluting the working copy.
  if [ -L "$DEST" ]; then
    if resolved="$(readlink -f "$DEST" 2>/dev/null || readlink "$DEST")"; then
      case "$resolved" in
        "$REPO"|"$REPO"/*)
          echo "error: $DEST is a symlink into this repo ($resolved)." >&2
          echo "Remove it (rm \"$DEST\") and re-run; the script will recreate it as a real dir." >&2
          exit 1
          ;;
      esac
    fi
  fi

  mkdir -p "$DEST"

  for i in "${!names[@]}"; do
    name="${names[$i]}"
    src="${srcs[$i]}"
    target="$DEST/$name"

    if [ -e "$target" ] && [ ! -L "$target" ]; then
      rm -rf "$target"
    fi

    ln -sfn "$src" "$target"
    echo "linked $name -> $src ($DEST)"
  done
done
