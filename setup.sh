#!/usr/bin/env bash
set -euo pipefail

# One-time workspace setup for office-agent.
# Run from the repository root.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

echo "==> Install uv (local)"
if [[ ! -d "$ROOT_DIR/.uv" ]]; then
    curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR="$ROOT_DIR/.uv" sh
fi
UV="$ROOT_DIR/.uv/uv"

export PATH="$ROOT_DIR/.uv:$PATH"

echo "==> Install Python 3.13 via uv"
"$UV" python install 3.13 --quiet

echo "==> Set up agent-core venv"
cd "$ROOT_DIR/agent-core"
"$UV" venv --python 3.13 .venv
"$UV" pip install -e '.[mcp,acp,dev]'
"$UV" pip install -e '../optional-mcps/mcp-cv-screen'

echo "==> Build web dashboard"
cd "$ROOT_DIR/agent-core/web"
npm install
npm run build

echo "==> Build Rust MCP workspace"
cd "$ROOT_DIR/optional-mcps"
cargo build --workspace --release

echo "==> Install local npm deps (Zalo plugin)"
cd "$ROOT_DIR"
npm install

echo "==> Register Rust MCPs with Hermes"
cd "$ROOT_DIR/agent-core"
# register_mcp <name> <KEY=VALUE> [<KEY=VALUE> ...]
# Idempotent: skips if already listed. Empty values (KEY=) register the var
# as a placeholder; fill real secrets in ~/.hermes/config.yaml (mcp_servers.<name>.env)
# before using tools that call the upstream API.
register_mcp() {
    local name="$1"; shift
    if ! .venv/bin/hermes mcp list | grep -q "$name"; then
        local env_args=()
        local kv
        for kv in "$@"; do env_args+=( --env "$kv" ); done
        echo "Y" | .venv/bin/hermes mcp add "$name" \
            --command "$ROOT_DIR/optional-mcps/target/release/$name" \
            "${env_args[@]}"
    fi
}
register_mcp mcp-catalog "CATALOG_DB=$HOME/.hermes/data/catalog.db"
register_mcp mcp-ledger "LEDGER_DB=$HOME/.hermes/data/ledger.db"
register_mcp mcp-video-edit "EDIT_WORKSPACE=$HOME/.hermes/work"
# Social MCPs: registered with empty secret placeholders. Fill tokens in
# ~/.hermes/config.yaml under mcp_servers.<name>.env before publishing/replying.
register_mcp mcp-social-youtube \
    "YOUTUBE_CLIENT_ID=" "YOUTUBE_CLIENT_SECRET=" "YOUTUBE_REFRESH_TOKEN="
register_mcp mcp-social-meta \
    "META_APP_ID=" "META_APP_SECRET=" "META_PAGE_ACCESS_TOKEN=" "META_IG_USER_ID="
register_mcp mcp-social-tiktok \
    "TIKTOK_CLIENT_KEY=" "TIKTOK_CLIENT_SECRET=" "TIKTOK_ACCESS_TOKEN="

echo "==> Setup complete. Next steps (manual):"
echo "    1. Configure API keys in ~/.hermes/.env"
echo "    2. Fill social MCP tokens in ~/.hermes/config.yaml (mcp_servers.mcp-social-*.env)"
echo "    3. Run: npm run setup:zalo   # scan QR with a secondary Zalo account"
echo "    4. Run: .venv/bin/hermes gateway"
echo "    5. Start agent: .venv/bin/hermes"
