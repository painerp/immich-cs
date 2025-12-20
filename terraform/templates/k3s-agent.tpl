#cloud-config
package_reboot_if_required: true
packages:
  - htop

write_files:
%{ if enable_tailscale ~}
  - path: /usr/local/bin/setup-tailscale.sh
    permissions: "0755"
    owner: root:root
    content: |
      ${indent(6, tailscale_script)}
${tailscale_ip_updater_files}
%{ endif ~}

bootcmd:
  - [ cloud-init-per, once, first-boot, touch, /run/first-boot-timestamp ]

runcmd:
%{ if enable_tailscale ~}
  # Setup Tailscale VPN
  - echo "Setting up Tailscale..." >> /var/log/k3s-agent.log
  - /usr/local/bin/setup-tailscale.sh
  # Join K3s cluster as agent
  - echo "Joining K3s cluster as agent..." >> /var/log/k3s-agent.log
  - |
    #!/bin/bash
    # Get Tailscale IP for external node IP
    TAILSCALE_IP=$(tailscale ip -4)
    echo "Tailscale IP: $TAILSCALE_IP" >> /var/log/k3s-agent.log
    # Install k3s agent
    curl -sfL https://get.k3s.io | K3S_URL=${k3s_url} K3S_TOKEN="${k3s_token}" sh -s - agent \
      --node-external-ip=$TAILSCALE_IP >> /var/log/k3s-agent.log 2>&1
  # Enable Tailscale IP updater
  - systemctl daemon-reload
  - systemctl enable tailscale-k3s-ip-updater.timer
  - systemctl start tailscale-k3s-ip-updater.timer
  - echo "K3s agent setup complete" >> /var/log/k3s-agent.log
%{ else ~}
  - curl -sfL https://get.k3s.io | K3S_URL=${k3s_url} K3S_TOKEN="${k3s_token}" sh -
%{ endif ~}

