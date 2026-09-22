#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

changelog_version=$(grep -m1 -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' CHANGELOG.md | tr -d '[]# ' || true)

tag_version="${GITHUB_REF_NAME:-}"
tag_version="${tag_version#v}"
tag_version="${tag_version#.}"

if [ -z "$changelog_version" ]; then
  echo "No versioned section found in CHANGELOG.md" >&2
  exit 1
fi

if [ "$tag_version" != "$changelog_version" ]; then
  echo "Tag ($tag_version) does not match CHANGELOG.md ($changelog_version)" >&2
  exit 1
fi

section=$(awk -v ver="$changelog_version" '
  $0 ~ "^## \\[" ver "\\]" { found = 1 }
  found && /^## / && $0 !~ "^## \\[" ver "\\]" { found = 0 }
  found { print }
' CHANGELOG.md)

if [ -z "$section" ]; then
  echo "Section [$changelog_version] not found in CHANGELOG.md" >&2
  exit 1
fi

short="v.$(echo "$changelog_version" | cut -d. -f1,2)"

cat <<EOF
<div align="center">
  <br><img src="https://raw.githubusercontent.com/Agzes/ToTray/refs/heads/main/assets/logo.png" width="128" alt="ToTray logo"/>
  <br><h1 align="center">&nbsp;&nbsp;&nbsp;&nbsp; \$\Huge{\textsf{ToTray}}\$ <sup><sup><kbd>${short}</kbd></sup></sup>
  <br></h1>
  <p><b>An automated application manager and tray utility for Hyprland.</b></p>
</div>

${section}

<h2></h2>
<kbd>With</kbd> <kbd>❤️</kbd> <kbd>by</kbd> <kbd>Agzes</kbd><br>
<kbd>pls ⭐ project!</kbd>
EOF
