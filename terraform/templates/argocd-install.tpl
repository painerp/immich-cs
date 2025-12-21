#!/bin/bash
set -e

LOG_FILE="/var/log/argocd-install.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "ArgoCD Installation"
log "Setting up kubeconfig..."
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
export PATH=$PATH:/usr/local/bin

log "Waiting for K3s to be ready..."
timeout 300 bash -c 'until kubectl get nodes &>/dev/null; do sleep 5; done' || {
    log "ERROR: K3s did not become ready in time"
    exit 1
}

log "Waiting for all nodes to be Ready..."
timeout 600 bash -c 'until [ $(kubectl get nodes --no-headers 2>/dev/null | grep -v NotReady | wc -l) -eq ${total_nodes} ]; do sleep 10; done' || {
    log "WARNING: Not all nodes became ready in time, continuing anyway..."
}

log "Creating argocd namespace..."
kubectl create namespace argocd --dry-run=client -o yaml | kubectl apply -f - >> "$LOG_FILE" 2>&1

log "Adding ArgoCD helm repository..."
helm repo add argo https://argoproj.github.io/argo-helm >> "$LOG_FILE" 2>&1
helm repo update >> "$LOG_FILE" 2>&1

log "Creating values file for ArgoCD..."
cat > /tmp/argocd-values.yaml <<'EOFVALUES'
global:
  domain: argocd.local

configs:
  params:
    server.insecure: "true"  # We'll use Tailscale for secure access, no need for TLS termination
  secret:
    createSecret: true
%{ if admin_password != "" ~}
    argocdServerAdminPassword: "${admin_password}"
%{ endif ~}

server:
  service:
    type: ClusterIP
  ingress:
    enabled: false

controller:
  metrics:
    enabled: true

dex:
  enabled: false

applicationSet:
  enabled: true
EOFVALUES

log "Installing ArgoCD..."
helm install argocd argo/argo-cd \
  -n argocd \
  -f /tmp/argocd-values.yaml \
  --wait \
  --timeout 20m >> "$LOG_FILE" 2>&1 || {
    log "ERROR: ArgoCD installation failed"
    log "Checking ArgoCD logs..."
    kubectl logs -n argocd -l app.kubernetes.io/name=argocd-server --tail=50 >> "$LOG_FILE" 2>&1 || true
    exit 1
}

log "Waiting for ArgoCD pods to be ready..."
timeout 600 bash -c 'until [ $(kubectl get pods -n argocd --no-headers 2>/dev/null | grep -v Completed | grep -v Running | wc -l) -eq 0 ]; do sleep 10; done' || {
    log "WARNING: Not all ArgoCD pods became ready in time"
    kubectl get pods -n argocd >> "$LOG_FILE" 2>&1
}

%{ if repo_url != "" ~}
log "Adding GitHub repository to ArgoCD..."
cat > /tmp/argocd-repo.yaml <<'EOFREPO'
apiVersion: v1
kind: Secret
metadata:
  name: apps-repo
  namespace: argocd
  labels:
    argocd.argoproj.io/secret-type: repository
stringData:
  type: git
  url: ${repo_url}
EOFREPO

kubectl apply -f /tmp/argocd-repo.yaml >> "$LOG_FILE" 2>&1 || {
    log "WARNING: Failed to add repository to ArgoCD"
}
log "Repository added successfully"

log "Waiting for ArgoCD to recognize the repository..."
sleep 10

log "Creating root Application (App of Apps)..."
cat > /tmp/argocd-root-app.yaml <<'EOFAPP'
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: root-apps
  namespace: argocd
  finalizers:
    - resources-finalizer.argocd.argoproj.io
spec:
  project: default
  source:
    repoURL: ${repo_url}
    targetRevision: ${repo_branch}
    path: apps
    directory:
      recurse: true
      jsonnet: {}
  destination:
    server: https://kubernetes.default.svc
    namespace: argocd
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
      allowEmpty: false
    syncOptions:
    - CreateNamespace=true
EOFAPP

kubectl apply -f /tmp/argocd-root-app.yaml >> "$LOG_FILE" 2>&1 || {
    log "WARNING: Failed to create root Application"
}
log "Root Application created successfully"
%{ endif ~}

log "ArgoCD installation complete!"
