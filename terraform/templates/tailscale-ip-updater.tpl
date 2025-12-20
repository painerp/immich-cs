  - path: /usr/local/bin/update-k3s-tailscale-ip.sh
    permissions: "0755"
    owner: root:root
    content: |
      #!/bin/bash
      # Update K3s node-external-ip annotation when Tailscale IP changes
      # This handles ephemeral node reconnections with new IPs

      LOGFILE="/var/log/tailscale-ip-updater.log"
      STATEFILE="/var/lib/tailscale/last-k3s-ip"
      mkdir -p /var/lib/tailscale

      CURRENT_IP=$(tailscale ip -4 2>/dev/null)

      if [ -z "$CURRENT_IP" ]; then
        echo "$(date): Tailscale not connected" >> $LOGFILE
        exit 0
      fi

      # Get last known IP
      LAST_IP=""
      if [ -f "$STATEFILE" ]; then
        LAST_IP=$(cat $STATEFILE)
      fi

      # Check if IP changed
      if [ "$CURRENT_IP" != "$LAST_IP" ]; then
        echo "$(date): Tailscale IP changed from $LAST_IP to $CURRENT_IP" >> $LOGFILE

        # Update K3s node annotation
        export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
        NODE_NAME=$(hostname)

        # Wait for K3s to be ready
        for i in $(seq 1 30); do
          if kubectl get node $NODE_NAME &>/dev/null; then
            break
          fi
          sleep 2
        done

        # Update node external IP annotation
        if kubectl annotate node $NODE_NAME --overwrite \
           "alpha.kubernetes.io/provided-node-ip=$CURRENT_IP" 2>> $LOGFILE; then
          echo "$(date): Successfully updated node annotation" >> $LOGFILE
          echo "$CURRENT_IP" > $STATEFILE
        else
          echo "$(date): Failed to update annotation, restarting ${k3s_service}" >> $LOGFILE
          systemctl restart ${k3s_service}
          echo "$CURRENT_IP" > $STATEFILE
        fi
      fi
  - path: /etc/systemd/system/tailscale-k3s-ip-updater.service
    content: |
      [Unit]
      Description=Update K3s node external IP when Tailscale IP changes
      After=tailscaled.service ${k3s_service}.service
      Wants=tailscaled.service ${k3s_service}.service

      [Service]
      Type=oneshot
      ExecStart=/usr/local/bin/update-k3s-tailscale-ip.sh

      [Install]
      WantedBy=multi-user.target
  - path: /etc/systemd/system/tailscale-k3s-ip-updater.timer
    content: |
      [Unit]
      Description=Check Tailscale IP changes every ${tailscale_ip_update_interval} seconds

      [Timer]
      OnBootSec=30
      OnUnitActiveSec=${tailscale_ip_update_interval}

      [Install]
      WantedBy=timers.target

