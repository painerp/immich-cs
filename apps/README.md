# Kubernetes Applications

Kubernetes-Manifeste für Immich und zugehörige Services. Diese Anwendungen werden von **ArgoCD** überwacht und automatisch synchronisiert.

## Struktur

### root/
ArgoCD Application-Definitionen (Meta-Layer):
- **immich-application.yaml** - ArgoCD App für Immich-Deployment
- **cnpg-application.yaml** - ArgoCD App für CloudNativePG Operator
- **prometheus-application.yaml** - ArgoCD App für Monitoring

Diese Dateien definieren, welche Git-Repositories ArgoCD überwachen soll.

### immich/
Kubernetes-Manifeste für die Immich-Anwendung:

- **namespace.yaml** - Namespace-Definition
- **database-cluster.yaml** - PostgreSQL-Cluster (CNPG)
- **prometheus-ingress.yaml** - Monitoring-Ingress

#### immich/server/
Immich Backend-Server:
- **deployment.yaml** - Haupt-Deployment
- **service.yaml** - Kubernetes Service
- **pvc.yaml** - Persistent Volume Claim für Uploads
- **monitor.yaml** - ServiceMonitor für Prometheus

#### immich/machine-learning/
Immich ML-Service für Gesichtserkennung und Objekterkennung:
- **deployment.yaml** - ML-Deployment
- **service.yaml** - Kubernetes Service
- **pvc.yaml** - Model-Cache Storage

#### immich/valkey/
Valkey (Redis-Fork) für Caching:
- **deployment.yaml** - Valkey-Deployment
- **service.yaml** - Kubernetes Service
- **pvc.yaml** - Persistent Storage

## GitOps-Workflow

Alle Änderungen an den Manifesten in diesem Ordner werden automatisch von ArgoCD erkannt und auf den Cluster angewendet:

1. Manifeste im Git-Repository ändern
2. Änderungen committen und pushen
3. ArgoCD synchronisiert automatisch (oder manuell triggern)
4. Änderungen werden auf den Cluster deployed

## Sync-Einstellungen

- **Auto-Prune**: Aktiviert - Ressourcen werden gelöscht, wenn sie aus Git entfernt werden
- **Self-Heal**: Aktiviert - Manuelle Änderungen am Cluster werden automatisch zurückgesetzt
- **Server-Side Apply**: Aktiviert - Reduziert Konflikte bei gleichzeitigen Änderungen
