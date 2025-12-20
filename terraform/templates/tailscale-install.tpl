#!/bin/bash
set -e

LOG_FILE="/var/log/tailscale-setup.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "Installing Tailscale..."
curl -fsSL https://tailscale.com/install.sh | sh >> /var/log/tailscale-setup.log 2>&1

log "Waiting for tailscaled service..."
for i in $(seq 1 30); do
  if systemctl is-active --quiet tailscaled; then
    log "tailscaled service is active"
    break
  fi
  log "Waiting for tailscaled to start (attempt $i/30)..."
  sleep 2
done

log "Connecting to Tailscale network..."
tailscale up \
  --authkey="${tailscale_auth_key}" \
  --hostname="${tailscale_hostname}" \
  --accept-routes=true \
  --ssh >> /var/log/tailscale-setup.log 2>&1

log "Waiting for Tailscale connection..."
for i in $(seq 1 60); do
  if tailscale status --json 2>/dev/null | grep -q '"BackendState": "Running"'; then
    log "Tailscale is connected and running"
    TAILSCALE_IP=$(tailscale ip -4)
    log "Tailscale IPv4: $TAILSCALE_IP"
    break
  fi
  log "Waiting for Tailscale to connect (attempt $i/60)..."
  sleep 2
done

