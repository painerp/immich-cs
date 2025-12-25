#!/bin/bash
set -e

LOG_FILE="/var/log/gpu-operator-install.log"
log() {
    echo "$@" | tee -a "$LOG_FILE"
}

log "NVIDIA GPU Operator Installation"
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

log "Adding NVIDIA helm repository..."
helm repo add nvidia https://helm.ngc.nvidia.com/nvidia >> "$LOG_FILE" 2>&1
helm repo update >> "$LOG_FILE" 2>&1

log "Installing NVIDIA GPU Operator..."
kubectl create namespace gpu-operator || true
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: time-slicing-config
  namespace: gpu-operator
data:
  any: |-
    version: v1
    sharing:
      timeSlicing:
        resources:
          - name: nvidia.com/gpu
            replicas: 4
EOF

helm install --wait gpu-operator-1 \
  -n gpu-operator --create-namespace \
  nvidia/gpu-operator \
  --set operator.defaultRuntime=containerd \
  --set toolkit.env[0].name=CONTAINERD_CONFIG \
  --set toolkit.env[0].value=/var/lib/rancher/k3s/agent/etc/containerd/config.toml \
  --set toolkit.env[1].name=CONTAINERD_SOCKET \
  --set toolkit.env[1].value=/run/k3s/containerd/containerd.sock \
  --set toolkit.env[2].name=CONTAINERD_RUNTIME_CLASS \
  --set toolkit.env[2].value=nvidia \
  --set toolkit.env[3].name=CONTAINERD_SET_AS_DEFAULT \
  --set-string toolkit.env[3].value=true \
  --set devicePlugin.config.name=time-slicing-config \
  --set devicePlugin.config.default=any \
  --timeout 20m >> "$LOG_FILE" 2>&1 || {
    log "ERROR: GPU Operator installation failed"
    log "Checking operator logs..."
    kubectl logs -n gpu-operator -l app=gpu-operator --tail=50 >> "$LOG_FILE" 2>&1 || true
    exit 1
}

log "Waiting for GPU Operator pods to be ready..."
timeout 900 bash -c 'until [ $(kubectl get pods -n gpu-operator --no-headers 2>/dev/null | grep -v Completed | grep -v Running | wc -l) -eq 0 ]; do sleep 10; done' || {
    log "WARNING: Not all GPU Operator pods became ready in time"
    kubectl get pods -n gpu-operator >> "$LOG_FILE" 2>&1
}

log "Verifying GPU resources..."
sleep 30
GPU_COUNT=$(kubectl get nodes -o json | jq '[.items[] | select(.status.capacity."nvidia.com/gpu" != null)] | length')
log "Nodes with GPU resources: $GPU_COUNT"

if [ "$GPU_COUNT" -gt 0 ]; then
    log "GPU Operator installation complete!"
    kubectl get nodes -o json | jq -r '.items[] | select(.status.capacity."nvidia.com/gpu" != null) | "\(.metadata.name): \(.status.capacity."nvidia.com/gpu") GPU(s)"' >> "$LOG_FILE" 2>&1
else
    log "GPU Operator installed but no GPU resources detected yet"
    log "This may take a few more minutes for GPUs to be discovered"
fi

