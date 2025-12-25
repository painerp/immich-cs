#!/bin/bash
set -e

LOG_FILE="/var/log/tailscale-longhorn-serve.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "Setting up Tailscale Serve for Longhorn..."
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

log "Waiting for K3s to be ready..."
timeout 300 bash -c 'until kubectl get nodes &>/dev/null; do sleep 5; done' || {
    log "ERROR: K3s did not become ready in time"
    exit 1
}

log "Waiting for Longhorn namespace..."
timeout ${longhorn_wait_timeout} bash -c 'until kubectl get namespace longhorn-system &>/dev/null; do sleep 10; done' || {
    log "ERROR: Longhorn namespace not found in time"
    exit 1
}

log "Waiting for Longhorn frontend service to be ready..."
timeout ${longhorn_wait_timeout} bash -c 'until kubectl get svc -n longhorn-system longhorn-frontend &>/dev/null; do sleep 10; done' || {
    log "ERROR: Longhorn frontend service not found"
    exit 1
}

log "Waiting for Longhorn UI pods to be running..."
timeout ${longhorn_wait_timeout} bash -c 'until [ $(kubectl get pods -n longhorn-system -l app=longhorn-ui --no-headers 2>/dev/null | grep Running | wc -l) -gt 0 ]; do sleep 10; done' || {
    log "WARNING: Longhorn UI pods not running yet, continuing anyway"
}

log "Getting cluster IP of Longhorn frontend service..."
LONGHORN_CLUSTER_IP=$(kubectl get svc -n longhorn-system longhorn-frontend -o jsonpath='{.spec.clusterIP}')
log "Longhorn frontend service cluster IP: $LONGHORN_CLUSTER_IP"

log "Configuring Tailscale Serve to expose Longhorn UI..."
tailscale serve --service=svc:longhorn --https=443 "http://$LONGHORN_CLUSTER_IP:80" >> "$LOG_FILE" 2>&1 || {
    log "ERROR: Failed to configure Tailscale Serve"
    exit 1
}

log "===================================================================="
log "Tailscale Serve configured successfully for Longhorn UI"
log "Access via Tailscale: https://longhorn.$(tailscale status --json | jq -r '.MagicDNSSuffix')"
log "===================================================================="

