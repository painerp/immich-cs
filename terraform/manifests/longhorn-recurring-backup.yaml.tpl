apiVersion: longhorn.io/v1beta2
kind: RecurringJob
metadata:
  name: backup-daily
  namespace: longhorn-system
spec:
  cron: "${backup_cron}"
  task: backup
  groups:
  - default
  retain: ${backup_retain}
  concurrency: ${backup_concurrency}
  labels:
    recurring-job-group: default
---
apiVersion: longhorn.io/v1beta2
kind: RecurringJob
metadata:
  name: snapshot-hourly
  namespace: longhorn-system
spec:
  cron: "0 * * * *"
  task: snapshot
  groups:
  - default
  retain: 24
  concurrency: 2
  labels:
    recurring-job-group: default

