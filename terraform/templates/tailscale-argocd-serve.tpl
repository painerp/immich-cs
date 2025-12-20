#!/bin/bash
set -e

LOG_FILE="/var/log/tailscale-argocd-serve.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "Setting up Tailscale Serve for ArgoCD..."
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

log "Waiting for K3s to be ready..."
timeout 300 bash -c 'until kubectl get nodes &>/dev/null; do sleep 5; done' || {
    log "ERROR: K3s did not become ready in time"
    exit 1
}

log "Waiting for ArgoCD namespace..."
timeout ${argocd_wait_timeout} bash -c 'until kubectl get namespace argocd &>/dev/null; do sleep 10; done' || {
    log "ERROR: ArgoCD namespace not found in time"
    exit 1
}

log "Waiting for ArgoCD server service to be ready..."
timeout ${argocd_wait_timeout} bash -c 'until kubectl get svc -n argocd argocd-server &>/dev/null; do sleep 10; done' || {
    log "ERROR: ArgoCD server service not found"
    exit 1
}

log "Waiting for ArgoCD pods to be running..."
timeout ${argocd_wait_timeout} bash -c 'until [ $(kubectl get pods -n argocd -l app.kubernetes.io/name=argocd-server --no-headers 2>/dev/null | grep Running | wc -l) -gt 0 ]; do sleep 10; done' || {
    log "WARNING: ArgoCD server pods not running yet, continuing anyway"
}

log "Getting cluster IP of ArgoCD server service..."
ARGOCD_CLUSTER_IP=$(kubectl get svc -n argocd argocd-server -o jsonpath='{.spec.clusterIP}')
log "ArgoCD service cluster IP: $ARGOCD_CLUSTER_IP"

log "Configuring Tailscale Serve to expose ArgoCD..."
tailscale serve --service=svc:argocd --https=443 "http://$ARGOCD_CLUSTER_IP:80" >> "$LOG_FILE" 2>&1 || {
    log "ERROR: Failed to configure Tailscale Serve"
    exit 1
}

log "===================================================================="
log "Tailscale Serve configured successfully for ArgoCD"
log "Access via Tailscale: https://argocd.$(tailscale status --json | jq -r '.MagicDNSSuffix')"
log "Username: admin"
%{ if admin_password == "" ~}
log "Password: $(kubectl -n argocd get secret argocd-initial-admin-secret -o jsonpath="{.data.password}" 2>/dev/null | base64 -d)"
%{ else ~}
log "Password: <configured via terraform variable>"
%{ endif ~}
log "===================================================================="
