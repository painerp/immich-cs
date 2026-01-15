#!/bin/bash
set -e

LOG_FILE="/var/log/tailscale-prometheus-serve.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "Setting up Tailscale Serve for Prometheus..."
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

log "Waiting for K3s to be ready..."
timeout 300 bash -c 'until kubectl get nodes &>/dev/null; do sleep 5; done' || {
    log "ERROR: K3s did not become ready in time"
    exit 1
}

log "Waiting for Prometheus namespace and service..."
timeout 600 bash -c 'until kubectl get svc -n prometheus-system prometheus-kube-prometheus-prometheus &>/dev/null; do sleep 10; done' || {
    log "ERROR: Prometheus service not found in time"
    exit 1
}

log "Getting cluster IP of Prometheus service..."
PROMETHEUS_CLUSTER_IP=$(kubectl get svc -n prometheus-system prometheus-kube-prometheus-prometheus -o jsonpath='{.spec.clusterIP}')
log "Prometheus service cluster IP: $PROMETHEUS_CLUSTER_IP"

# Extract the Tailscale hostname/domain suffix
TAILSCALE_HOSTNAME="${tailscale_hostname}"
TAILSCALE_DOMAIN=$(tailscale status --json 2>/dev/null | jq -r '.MagicDNSSuffix' | sed 's/^\.//g')

log "Configuring Tailscale Serve to expose Prometheus..."
tailscale serve --service=svc:prometheus --https=443 "http://$PROMETHEUS_CLUSTER_IP:9090" >> "$LOG_FILE" 2>&1 || {
    log "ERROR: Failed to configure Tailscale Serve for Prometheus"
    exit 1
}

log "===================================================================="
log "Tailscale Serve configured successfully for Prometheus"
log "Access Prometheus via: https://prometheus.$TAILSCALE_DOMAIN"
log "Or via Tailscale IP: https://$(tailscale ip -4):443"
log "===================================================================="
