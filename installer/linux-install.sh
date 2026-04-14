#!/usr/bin/env bash
# Indexarr Linux installer
# Usage: sudo bash linux-install.sh [--version v0.1.0]
# Installs the indexarr binary, PostgreSQL, and a systemd service.

set -euo pipefail

VERSION="${1:-}"
INDEXARR_USER="indexarr"
INDEXARR_GROUP="indexarr"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/indexarr"
CONF_DIR="/etc/indexarr"
LOG_DIR="/var/log/indexarr"
SERVICE_FILE="/etc/systemd/system/indexarr.service"
PG_DB="indexarr"
PG_ROLE="indexarr"
PG_PASS="indexarr"
DOWNLOAD_BASE="https://indexarr.net/downloads"

# ── Helpers ───────────────────────────────────────────────────────────────────
info()  { echo -e "\033[1;32m[indexarr]\033[0m $*"; }
warn()  { echo -e "\033[1;33m[indexarr]\033[0m $*"; }
abort() { echo -e "\033[1;31m[indexarr]\033[0m $*" >&2; exit 1; }

require_root() {
  [[ $EUID -eq 0 ]] || abort "Run with sudo or as root."
}

detect_distro() {
  if command -v apt-get &>/dev/null; then
    echo "apt"
  elif command -v dnf &>/dev/null; then
    echo "dnf"
  elif command -v yum &>/dev/null; then
    echo "yum"
  else
    abort "Unsupported package manager. Install PostgreSQL 16+ manually and re-run."
  fi
}

# ── PostgreSQL install ────────────────────────────────────────────────────────
install_postgres_apt() {
  info "Installing PostgreSQL via apt..."
  apt-get update -qq
  apt-get install -y --no-install-recommends postgresql postgresql-client
}

install_postgres_dnf() {
  info "Installing PostgreSQL via dnf..."
  dnf install -y postgresql-server postgresql
  postgresql-setup --initdb
  systemctl enable --now postgresql
}

install_postgres_yum() {
  info "Installing PostgreSQL via yum..."
  yum install -y postgresql-server postgresql
  postgresql-setup initdb
  systemctl enable --now postgresql
}

ensure_postgres() {
  if pg_isready -q 2>/dev/null; then
    info "PostgreSQL is already running."
    return
  fi

  local pkg_mgr
  pkg_mgr=$(detect_distro)

  case "$pkg_mgr" in
    apt) install_postgres_apt ;;
    dnf) install_postgres_dnf ;;
    yum) install_postgres_yum ;;
  esac

  # On Debian/Ubuntu postgresql service starts automatically; ensure it's up.
  if systemctl is-active --quiet postgresql 2>/dev/null || \
     systemctl is-active --quiet "postgresql@*" 2>/dev/null; then
    info "PostgreSQL service started."
  else
    systemctl start postgresql 2>/dev/null || true
  fi

  # Wait up to 30s for PostgreSQL to become ready
  local i=0
  while ! pg_isready -q 2>/dev/null; do
    sleep 1
    i=$((i+1))
    [[ $i -lt 30 ]] || abort "PostgreSQL did not start within 30 seconds."
  done
  info "PostgreSQL is ready."
}

# ── Database setup ────────────────────────────────────────────────────────────
setup_database() {
  info "Setting up Indexarr database..."

  # Create role if it doesn't exist
  if ! sudo -u postgres psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='${PG_ROLE}'" | grep -q 1; then
    sudo -u postgres psql -c "CREATE USER ${PG_ROLE} WITH PASSWORD '${PG_PASS}';"
    info "Created PostgreSQL role '${PG_ROLE}'."
  else
    warn "PostgreSQL role '${PG_ROLE}' already exists, skipping."
  fi

  # Create database if it doesn't exist
  if ! sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='${PG_DB}'" | grep -q 1; then
    sudo -u postgres psql -c "CREATE DATABASE ${PG_DB} OWNER ${PG_ROLE};"
    info "Created PostgreSQL database '${PG_DB}'."
  else
    warn "PostgreSQL database '${PG_DB}' already exists, skipping."
  fi
}

# ── Binary install ────────────────────────────────────────────────────────────
install_binary() {
  info "Installing Indexarr binary..."

  local arch
  arch=$(uname -m)
  [[ "$arch" == "x86_64" ]] || abort "Only x86_64 is currently supported (got $arch)."

  local bin_name="indexarr-linux-x86_64"
  local tmp_bin
  tmp_bin=$(mktemp)

  if [[ -n "${VERSION}" ]]; then
    local url="${DOWNLOAD_BASE}/${VERSION}/${bin_name}"
  else
    local url="${DOWNLOAD_BASE}/latest/${bin_name}"
  fi

  info "Downloading from ${url}..."
  if command -v curl &>/dev/null; then
    curl -fsSL "$url" -o "$tmp_bin"
  elif command -v wget &>/dev/null; then
    wget -q "$url" -O "$tmp_bin"
  else
    abort "Neither curl nor wget found. Install one and retry."
  fi

  chmod +x "$tmp_bin"
  mv "$tmp_bin" "${INSTALL_DIR}/indexarr"
  info "Installed to ${INSTALL_DIR}/indexarr."
}

# ── System user + directories ─────────────────────────────────────────────────
setup_system() {
  if ! id -u "${INDEXARR_USER}" &>/dev/null; then
    useradd --system --no-create-home --shell /usr/sbin/nologin "${INDEXARR_USER}"
    info "Created system user '${INDEXARR_USER}'."
  fi

  mkdir -p "${DATA_DIR}" "${CONF_DIR}" "${LOG_DIR}"
  chown "${INDEXARR_USER}:${INDEXARR_GROUP}" "${DATA_DIR}" "${LOG_DIR}"
  chmod 750 "${DATA_DIR}"
}

# ── Config file ───────────────────────────────────────────────────────────────
write_config() {
  if [[ -f "${CONF_DIR}/indexarr.env" ]]; then
    warn "Config already exists at ${CONF_DIR}/indexarr.env — not overwriting."
    return
  fi

  cat > "${CONF_DIR}/indexarr.env" <<EOF
INDEXARR_DB_URL=postgres://${PG_ROLE}:${PG_PASS}@127.0.0.1:5432/${PG_DB}
INDEXARR_DATA_DIR=${DATA_DIR}
EOF
  chmod 640 "${CONF_DIR}/indexarr.env"
  chown root:"${INDEXARR_GROUP}" "${CONF_DIR}/indexarr.env"
  info "Wrote ${CONF_DIR}/indexarr.env."
}

# ── systemd service ───────────────────────────────────────────────────────────
install_service() {
  cat > "${SERVICE_FILE}" <<EOF
[Unit]
Description=Indexarr — decentralized torrent indexer
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=${INDEXARR_USER}
Group=${INDEXARR_GROUP}
WorkingDirectory=${DATA_DIR}
EnvironmentFile=${CONF_DIR}/indexarr.env
ExecStart=${INSTALL_DIR}/indexarr --all
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=indexarr
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=${DATA_DIR} ${LOG_DIR}

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  systemctl enable indexarr
  systemctl restart indexarr
  info "Indexarr service installed and started."
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
  require_root

  info "=== Indexarr installer ==="

  ensure_postgres
  setup_database
  install_binary
  setup_system
  write_config
  install_service

  info ""
  info "Indexarr is running!"
  info "  Web UI:  http://localhost:8080"
  info "  Logs:    journalctl -u indexarr -f"
  info "  Config:  ${CONF_DIR}/indexarr.env"
  info "  Data:    ${DATA_DIR}"
}

main "$@"
