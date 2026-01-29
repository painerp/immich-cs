apiVersion: v1
kind: Secret
metadata:
  name: cloudflare-tunnel-credentials
  namespace: cloudflare
type: Opaque
stringData:
  credentials.json: |
    {
      "AccountTag": "${account_id}",
      "TunnelID": "${tunnel_id}",
      "TunnelSecret": "${tunnel_secret}"
    }
