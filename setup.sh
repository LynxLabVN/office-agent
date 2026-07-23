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
register_mcp() {
    local name="$1"
    local env_key="$2"
    local env_default="$3"
    if ! .venv/bin/hermes mcp list | grep -q "$name"; then
        echo "Y" | .venv/bin/hermes mcp add "$name" \
            --command "$ROOT_DIR/optional-mcps/target/release/$name" \
            --env "$env_key=$env_default"
    fi
}
register_mcp mcp-catalog CATALOG_DB "$HOME/.hermes/data/catalog.db"
register_mcp mcp-ledger LEDGER_DB "$HOME/.hermes/data/ledger.db"
register_mcp mcp-video-edit EDIT_WORKSPACE "$HOME/.hermes/work"

echo "==> Setup complete. Next steps (manual):"
echo "    1. Configure API keys in ~/.hermes/.env"
echo "    2. Run: npm run setup:zalo   # scan QR with a secondary Zalo account"
echo "    3. Run: .venv/bin/hermes gateway"
echo "    4. Start agent: .venv/bin/hermes"
