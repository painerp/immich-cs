apiVersion: v1
kind: Secret
metadata:
  name: operator-oauth
  namespace: tailscale
type: Opaque
data:
  client_id: ${client_id_b64}
  client_secret: ${client_secret_b64}
