#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

changelog_version=$(grep -m1 -oE '^## \[[0-9]+\.[0-9]+\.[0-9]+\]' CHANGELOG.md | tr -d '[]# ' || true)

tag_version="${1:-${GITHUB_REF_NAME:-}}"
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
  found && $0 !~ /^---$/ { print }
' CHANGELOG.md)

if [ -z "$section" ]; then
  echo "Section [$changelog_version] not found in CHANGELOG.md" >&2
  exit 1
fi

cat <<EOF
<div align="center">
  <p>
    <a href="https://aur.archlinux.org/packages/totray"><kbd>AUR package</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/ToTray#-build-from-source"><kbd>Build from source</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/ToTray/issues/new?template=bug_report.md"><kbd>Report a bug</kbd></a>
    &nbsp;·&nbsp;
    <a href="https://github.com/Agzes/ToTray/issues/new?template=feature_request.md"><kbd>Request a feature</kbd></a>
  </p>
  <h1><img height="24" alt="ToTray logo" src="https://raw.githubusercontent.com/Agzes/ToTray/refs/heads/main/assets/logo.png"/> ToTray <sup><kbd>v.${changelog_version}</kbd></sup></h1>
  <p>An automated application manager and tray utility for Hyprland.</p>
</div>

---

${section}

---

<kbd>With</kbd> <kbd>❤️</kbd> <kbd>by</kbd> <kbd>Agzes</kbd><br>
<kbd>pls ⭐ project!</kbd>
EOF
