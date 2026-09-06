#!/usr/bin/env bash
# Generate conformance vectors: for every theme on this machine, record the
# palette as resolved by the *real* omarchy-theme-color oracle. Bindings in
# any language should validate their resolver against these JSON files; the
# Rust and Odin conformance tests in this repo do exactly that.
#
#   ./conformance/generate.sh [output-dir]
#
# Each vector: conformance/vectors/<slug>.json
#   {
#     "theme": "<slug>",
#     "colors_file": "<source colors.toml>",   # informational
#     "light_mode_marker": true|false,          # light.mode beside colors.toml
#     "resolved": { "<key>": "<value>", ... }   # oracle `--all` output
#   }
#
# Notes:
# * `--all` prints every map key, including ones resolved to empty; those are
#   dropped here and consumers should treat empty as absent (matching the
#   bash `[[ ${map[k]} ]]` truthiness the oracle itself uses).
# * Only the *source* colors.toml is passed; the light.mode sidecar is
#   recorded because mode auto-detection consults it.

set -euo pipefail

OUT="${1:-$(dirname "$0")/vectors}"
ORACLE=$(command -v omarchy-theme-color || echo /usr/share/omarchy/bin/omarchy-theme-color)
[[ -x $ORACLE ]] || { echo "omarchy-theme-color not found; run on an Omarchy system" >&2; exit 1; }

mkdir -p "$OUT"
rm -f "$OUT"/*.json

emit() {
  local slug="$1" file="$2"
  local json="$OUT/$slug.json"
  local marker=false
  [[ -f $(dirname "$file")/light.mode ]] && marker=true

  {
    printf '{\n  "theme": "%s",\n' "$slug"
    printf '  "colors_file": "%s",\n' "$file"
    printf '  "light_mode_marker": %s,\n' "$marker"
    printf '  "resolved": {\n'
    # --all: key<TAB>value, LC_ALL=C sorted; skip empty values
    "$ORACLE" --file "$file" --all | awk -F'\t' '$2 != "" {
      gsub(/\\/, "\\\\", $2); gsub(/"/, "\\\"", $2)
      printf "    \"%s\": \"%s\",\n", $1, $2
    }' | sed '$ s/,$//'
    printf '  }\n}\n'
  } > "$json"
  echo "  $slug"
}

echo "themes:"
for dir in /usr/share/omarchy/themes/*/; do
  slug=$(basename "$dir")
  [[ -f $dir/colors.toml ]] || continue
  emit "$slug" "$dir/colors.toml"
done

# user themes override same-slug stock vectors and add their own
for dir in "$HOME/.config/omarchy/themes"/*/; do
  [[ -d $dir ]] || continue
  slug=$(basename "$dir")
  [[ -f $dir/colors.toml ]] || continue
  emit "$slug" "$dir/colors.toml"
done

# the staged current theme (post-overlay merge view)
STAGED="$HOME/.local/state/omarchy/current/theme/colors.toml"
if [[ -f $STAGED ]]; then
  name=$(cat "$HOME/.local/state/omarchy/current/theme.name" 2>/dev/null || echo current-staged)
  emit "staged-$name" "$STAGED"
fi

echo "wrote $(ls "$OUT"/*.json | wc -l) vectors to $OUT"
