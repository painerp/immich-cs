# Immich Cloud Services

Automatisiertes Deployment von [Immich](https://immich.app/) auf einem hochverfügbaren K3s-Cluster in OpenStack mit GitOps-Ansatz.

## Überblick

Dieses Projekt stellt eine vollständige Infrastructure-as-Code (IaC) Lösung bereit, um Immich auf einem Kubernetes-Cluster zu betreiben. Die Infrastruktur wird mit Terraform provisioniert und die Anwendungen werden mit ArgoCD verwaltet.

## Komponenten

- **terraform/** - Infrastructure-as-Code für K3s-Cluster auf OpenStack
- **apps/** - Kubernetes-Manifeste für Immich und zugehörige Services (ArgoCD-verwaltet)
- **im-deploy/** - CLI-Tool (Rust) zur Vereinfachung des Deployments
- **im-deploy.sh** - Wrapper-Script zum Bauen und Ausführen des CLI-Tools

## Schnellstart

1. Terraform-Variablen anpassen:
   ```bash
   cd terraform
   cp terraform.tfvars.example terraform.tfvars
   # terraform.tfvars bearbeiten
   ```

2. Infrastruktur deployen:
   ```bash
   terraform init
   terraform apply
   ```

3. ArgoCD-Anwendungen deployen (werden automatisch synchronisiert)

## Voraussetzungen

- Terraform >= 1.0
- OpenStack-Zugangsdaten
- SSH-Schlüsselpaar
- (Optional) Rust für das im-deploy CLI-Tool

## Architektur

- **K3s** als leichtgewichtige Kubernetes-Distribution
- **ArgoCD** für GitOps-basiertes Deployment
- **PostgreSQL** (CNPG Operator) für Immich-Datenbank
- **Valkey** (Redis-Fork) für Caching
- **Longhorn** für persistenten Storage
- **Prometheus** für Monitoring
