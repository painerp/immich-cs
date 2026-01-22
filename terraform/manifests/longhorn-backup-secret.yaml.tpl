apiVersion: v1
kind: Secret
metadata:
  name: longhorn-backup-s3-secret
  namespace: longhorn-system
type: Opaque
stringData:
  AWS_ACCESS_KEY_ID: ${access_key}
  AWS_SECRET_ACCESS_KEY: ${secret_key}
  AWS_ENDPOINTS: ${s3_endpoint}
%{ if ca_cert != "" ~}
data:
  AWS_CERT: ${ca_cert}
%{ endif ~}
