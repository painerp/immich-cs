apiVersion: helm.cattle.io/v1
kind: HelmChart
metadata:
  name: longhorn
  namespace: kube-system
spec:
  chart: longhorn
  repo: https://charts.longhorn.io
  version: 1.10.1
  targetNamespace: longhorn-system
  createNamespace: true
  bootstrap: true
  valuesContent: |-
    persistence:
      defaultClass: true
      defaultFsType: ext4
      defaultClassReplicaCount: ${replica_count}
      defaultDataPath: /var/lib/longhorn
      reclaimPolicy: Retain

    defaultSettings:
      defaultReplicaCount: ${replica_count}
      guaranteedInstanceManagerCpu: 5
      createDefaultDiskLabeledNodes: false
      defaultDataPath: /var/lib/longhorn
      defaultLonghornStaticStorageClass: longhorn
      replicaSoftAntiAffinity: true
      storageOverProvisioningPercentage: 200
      storageMinimalAvailablePercentage: 10
      upgradeChecker: false
      defaultDataLocality: disabled
      nodeDownPodDeletionPolicy: delete-both-statefulset-and-deployment-pod
%{ if backup_enabled ~}

    defaultBackupStore:
      backupTarget: ${backup_target}
      backupTargetCredentialSecret: longhorn-backup-s3-secret
      pollInterval: 300
%{ endif ~}

    ingress:
      enabled: false

    longhornManager:
      resources:
        requests:
          cpu: 50m
          memory: 128Mi
        limits:
          memory: 512Mi

    longhornDriver:
      resources:
        requests:
          cpu: 10m
          memory: 32Mi
        limits:
          memory: 128Mi

    longhornUI:
      resources:
        requests:
          cpu: 10m
          memory: 32Mi
        limits:
          memory: 128Mi

