#!/usr/bin/env bash
# ==============================================================================
# SITREP // DJP OSINT Console — Rust Binary Cutover Script (Proxmox LXC CT 104)
# ==============================================================================
# Usage:
#   chmod +x rust/cutover.sh
#   ./rust/cutover.sh [--check | --deploy | --rollback]
#
# Rules:
#   - Never executes against live environment without explicit authorization (ADR-006).
#   - 100% drop-in compatibility with .env, config.json, geo/*.json, data/* (ADR-005).
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN_PATH="${PROJECT_ROOT}/rust/app/target/release/djp-sitrep"
PORT="${PORT:-8080}"
SERVICE_NAME="djp-sitrep"

echo "=== SITREP // DJP Cutover Manager ==="
echo "Target Binary: ${BIN_PATH}"
echo "Project Root:  ${PROJECT_ROOT}"

check_prerequisites() {
  echo "[1/4] Checking prerequisites..."
  if [[ ! -f "${BIN_PATH}" ]]; then
    echo "ERROR: Release binary not found at ${BIN_PATH}. Run 'cargo build --release --manifest-path rust/app/Cargo.toml' first."
    exit 1
  fi

  if [[ ! -f "${PROJECT_ROOT}/.env" ]]; then
    echo "WARN: .env file not found in ${PROJECT_ROOT}. Using system environment variables."
  fi

  if [[ ! -d "${PROJECT_ROOT}/public" ]]; then
    echo "ERROR: public/ directory not found in ${PROJECT_ROOT}."
    exit 1
  fi

  if command -v ffmpeg >/dev/null 2>&1; then
    echo "  [OK] ffmpeg found: $(command -v ffmpeg)"
  else
    echo "  [WARN] ffmpeg not found in PATH; RTSP snapshots will use direct JPEG bypass."
  fi

  if command -v python3 >/dev/null 2>&1; then
    echo "  [OK] python3 found: $(command -v python3)"
  else
    echo "  [WARN] python3 not found in PATH; Threads scraper worker will be disabled."
  fi

  echo "  [OK] All prerequisites satisfied."
}

test_binary_smoke() {
  echo "[2/4] Testing binary startup in background test mode..."
  local TEST_PORT=18080
  PORT=${TEST_PORT} "${BIN_PATH}" &
  local BIN_PID=$!

  sleep 1

  local RETRIES=5
  local SUCCESS=0
  for i in $(seq 1 ${RETRIES}); do
    if curl -s "http://127.0.0.1:${TEST_PORT}/api/ping" | grep -q '"ok":true'; then
      echo "  [OK] Health check passed on port ${TEST_PORT} (attempt ${i})."
      SUCCESS=1
      break
    fi
    sleep 0.5
  done

  kill -TERM "${BIN_PID}" 2>/dev/null || true
  wait "${BIN_PID}" 2>/dev/null || true

  if [[ ${SUCCESS} -ne 1 ]]; then
    echo "ERROR: Binary failed smoke test on port ${TEST_PORT}."
    exit 1
  fi
}

create_systemd_unit() {
  echo "[3/4] Generating systemd unit definition..."
  local UNIT_CONTENT="[Unit]
Description=DJP SITREP OSINT Console (Pure Rust)
After=network.target

[Service]
Type=simple
User=${USER}
WorkingDirectory=${PROJECT_ROOT}
ExecStart=${BIN_PATH}
Restart=always
RestartSec=3
LimitNOFILE=65535
EnvironmentFile=-${PROJECT_ROOT}/.env
Environment=PORT=8080

[Install]
WantedBy=multi-user.target
"
  echo "${UNIT_CONTENT}" > "${SCRIPT_DIR}/djp-sitrep.service"
  echo "  [OK] Unit file written to ${SCRIPT_DIR}/djp-sitrep.service."
}

show_instructions() {
  echo "[4/4] Cutover instructions for Proxmox LXC CT 104:"
  echo "----------------------------------------------------------------------"
  echo "To switch from Node.js to pure Rust:"
  echo "  1. sudo cp ${SCRIPT_DIR}/djp-sitrep.service /etc/systemd/system/"
  echo "  2. sudo systemctl daemon-reload"
  echo "  3. pm2 stop all 2>/dev/null || true   # Stop existing Node.js server"
  echo "  4. sudo systemctl enable --now djp-sitrep"
  echo "  5. curl -i http://127.0.0.1:8080/api/ping"
  echo ""
  echo "To rollback to Node.js if needed:"
  echo "  1. sudo systemctl stop djp-sitrep"
  echo "  2. pm2 start server.js --name djp-sitrep"
  echo "----------------------------------------------------------------------"
}

check_prerequisites
test_binary_smoke
create_systemd_unit
show_instructions

echo "=== Pre-flight checks completed successfully ==="
