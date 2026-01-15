#!/bin/bash
set -e

LOG_FILE="/var/log/tailscale-grafana-serve.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "Setting up Tailscale Serve for Grafana..."
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

log "Waiting for K3s to be ready..."
timeout 300 bash -c 'until kubectl get nodes &>/dev/null; do sleep 5; done' || {
    log "ERROR: K3s did not become ready in time"
    exit 1
}

log "Waiting for Grafana service..."
timeout 600 bash -c 'until kubectl get svc -n prometheus-system prometheus-grafana &>/dev/null; do sleep 10; done' || {
    log "ERROR: Grafana service not found in time"
    exit 1
}

log "Getting cluster IP of Grafana service..."
GRAFANA_CLUSTER_IP=$(kubectl get svc -n prometheus-system prometheus-grafana -o jsonpath='{.spec.clusterIP}')
log "Grafana service cluster IP: $GRAFANA_CLUSTER_IP"

# Extract the Tailscale hostname/domain suffix
TAILSCALE_HOSTNAME="${tailscale_hostname}"
TAILSCALE_DOMAIN=$(tailscale status --json 2>/dev/null | jq -r '.MagicDNSSuffix' | sed 's/^\.//g')

log "Configuring Tailscale Serve to expose Grafana..."
tailscale serve --service=svc:grafana --https=443 "http://$GRAFANA_CLUSTER_IP:80" >> "$LOG_FILE" 2>&1 || {
    log "ERROR: Failed to configure Tailscale Serve for Grafana"
    exit 1
}

log "===================================================================="
log "Tailscale Serve configured successfully for Grafana"
log "Access Grafana via: https://grafana.$TAILSCALE_DOMAIN"
log "Default credentials: admin/admin (or check Grafana documentation)"
log "Or via Tailscale IP: https://$(tailscale ip -4):443"
log "===================================================================="
