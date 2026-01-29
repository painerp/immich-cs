# Terraform Infrastructure

Terraform-Konfiguration für die Provisionierung eines hochverfügbaren K3s-Clusters auf OpenStack.

## Struktur

- **main.tf** - Haupt-Orchestrierungsdatei, ruft OpenStack K3s Modul auf
- **variables.tf** - Terraform-Variablendefinitionen
- **outputs.tf** - Output-Werte (Cluster-IPs, Kubeconfig, etc.)
- **providers.tf** - Provider-Konfiguration (OpenStack, Helm, Kubernetes)
- **terraform.tfvars.example** - Beispiel-Konfiguration für Variablen

### Unterordner

#### modules/
Wiederverwendbare Terraform-Module:
- **openstack-k3s/** - Hauptmodul für K3s-Cluster-Deployment auf OpenStack
  - Erstellt VMs, Netzwerke, Load Balancer
  - Konfiguriert K3s Server und Agents
  - Richtet Tailscale VPN ein

#### manifests/
Kubernetes-Manifest-Templates für die Infrastruktur-Komponenten:
- **cloud-controller-openstack.yaml.tpl** - OpenStack Cloud Controller
- **csi-cinder.yaml.tpl** - Cinder CSI-Driver für Block-Storage
- **longhorn.yaml.tpl** - Longhorn Storage-System
- **longhorn-backup-secret.yaml.tpl** - S3-Backup-Konfiguration

#### templates/
Cloud-init und Shell-Script-Templates für VM-Provisionierung:
- **k3s-server.tpl** - Initialisierung der K3s Server-Nodes
- **k3s-agent.tpl** - Initialisierung der K3s Worker-Nodes
- **argocd-install.tpl** - ArgoCD-Installation
- **tailscale-*.tpl** - Tailscale VPN-Konfigurationen
- **gpu-operator-install.tpl** - NVIDIA GPU Operator

