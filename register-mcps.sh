#!/usr/bin/env bash
# Windows MCP registration for office-agent.
# Registers all 10 Rust MCPs + the Python mcp-cv-screen with the Hermes CLI.
# Idempotent: skips a server if already present in `hermes mcp list`.
#
# Run from the repo root after building:
#   optional-mcps/target/release/*.exe   (cargo build --workspace --release)
#   optional-mcps/mcp-cv-screen/.venv     (uv venv + uv pip install -e . --no-deps)
#
# Tokens left empty are placeholders — fill real secrets in
# ~/AppData/Local/hermes/config.yaml (mcp_servers.<name>.env) before use.

set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

HERMES="agent-core/.venv/Scripts/hermes.exe"
REL="optional-mcps/target/release"
CV_PY="$ROOT_DIR/optional-mcps/mcp-cv-screen/.venv/Scripts/python.exe"
DATA="$USERPROFILE/.hermes/data"

list="$( "$HERMES" mcp list 2>/dev/null || true )"

add() {  # add <name> <binary_or_cmd> [--args ...]   (env pairs via HERMES_ENV after)
  local name="$1"; shift
  local cmd="$1"; shift
  if echo "$list" | grep -q "$name"; then
    echo "skip  $name (already registered)"
    return 0
  fi
  local env_args=()
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --env) env_args+=( --env "$2" ); shift 2 ;;
      --args) shift; break ;;
      *) break ;;
    esac
  done
  echo "add   $name"
  # --args must be last per hermes mcp add syntax
  if [ "$#" -gt 0 ]; then
    echo "Y" | "$HERMES" mcp add "$name" --command "$cmd" "${env_args[@]}" --args "$@"
  else
    echo "Y" | "$HERMES" mcp add "$name" --command "$cmd" "${env_args[@]}"
  fi
}

# --- Rust MCPs (binaries) ---
add mcp-catalog        "$ROOT_DIR/$REL/mcp-catalog.exe"        --env "CATALOG_DB=$DATA/catalog.db"
add mcp-ledger         "$ROOT_DIR/$REL/mcp-ledger.exe"         --env "LEDGER_DB=$DATA/ledger.db"
add mcp-video-edit     "$ROOT_DIR/$REL/mcp-video-edit.exe"     --env "EDIT_WORKSPACE=$USERPROFILE/.hermes/work"
add mcp-hr-data        "$ROOT_DIR/$REL/mcp-hr-data.exe"       --env "HR_DB=$DATA/hr.db"
add mcp-schedule       "$ROOT_DIR/$REL/mcp-schedule.exe"       --env "CALCOM_API_URL=https://api.cal.com/v1"
add mcp-social-youtube "$ROOT_DIR/$REL/mcp-social-youtube.exe" \
  --env "YOUTUBE_CLIENT_ID=" --env "YOUTUBE_CLIENT_SECRET=" --env "YOUTUBE_REFRESH_TOKEN="
add mcp-social-meta    "$ROOT_DIR/$REL/mcp-social-meta.exe" \
  --env "META_APP_ID=" --env "META_APP_SECRET=" --env "META_PAGE_ACCESS_TOKEN=" --env "META_IG_USER_ID="
add mcp-social-tiktok  "$ROOT_DIR/$REL/mcp-social-tiktok.exe" \
  --env "TIKTOK_CLIENT_KEY=" --env "TIKTOK_CLIENT_SECRET=" --env "TIKTOK_ACCESS_TOKEN="
add mcp-trend-research "$ROOT_DIR/$REL/mcp-trend-research.exe"
add mcp-zalo-oa        "$ROOT_DIR/$REL/mcp-zalo-oa.exe" \
  --env "ZALO_OA_API_URL=https://openapi.zalo.me/v2.0/officialaccount" \
  --env "ZALO_OA_ACCESS_TOKEN=" --env "ZALO_OA_WEBHOOK_SECRET=" \
  --env "ZALO_OA_WEBHOOK_PORT=8080" --env "ZALO_OA_REPLY_MODE=suggest"

# --- Python MCP ---
add mcp-cv-screen "$CV_PY" --env "CV_CACHE_DIR=$DATA/cv_cache" --args -m mcp_cv_screen.server

echo
echo "==> Registered. Verify with:  $HERMES mcp list"