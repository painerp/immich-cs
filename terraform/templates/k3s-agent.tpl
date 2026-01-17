#cloud-config
package_reboot_if_required: true
packages:
  - htop
  - nfs-common
  - open-iscsi

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
%{ if enable_longhorn ~}
  # Setup Longhorn storage volume
  - echo "Setting up Longhorn storage volume..." >> /var/log/k3s-agent.log
  - |
    #!/bin/bash
    # Wait for block device to appear
    for i in {1..30}; do
      if [ -b /dev/vdb ]; then
        echo "Block device /dev/vdb found" >> /var/log/k3s-agent.log
        break
      fi
      echo "Waiting for /dev/vdb... ($i/30)" >> /var/log/k3s-agent.log
      sleep 2
    done

    # Check if volume already has a filesystem
    if blkid /dev/vdb; then
      echo "Existing filesystem detected on /dev/vdb - preserving data" >> /var/log/k3s-agent.log
      EXISTING_FS=$(blkid -s TYPE -o value /dev/vdb)
      echo "Filesystem type: $EXISTING_FS" >> /var/log/k3s-agent.log
    else
      echo "No filesystem found on /dev/vdb - formatting as ext4..." >> /var/log/k3s-agent.log
      mkfs.ext4 -F /dev/vdb >> /var/log/k3s-agent.log 2>&1
      echo "Format complete" >> /var/log/k3s-agent.log
    fi

    # Create mount point
    mkdir -p /var/lib/longhorn

    # Get UUID for fstab
    DISK_UUID=$(blkid -s UUID -o value /dev/vdb)

    # Add to fstab if not already present
    if ! grep -q "/var/lib/longhorn" /etc/fstab; then
      echo "UUID=$DISK_UUID /var/lib/longhorn ext4 defaults,nofail 0 2" >> /etc/fstab
      echo "Added Longhorn volume to fstab" >> /var/log/k3s-agent.log
    fi

    # Mount the volume
    mount /var/lib/longhorn
    echo "Mounted Longhorn storage at /var/lib/longhorn" >> /var/log/k3s-agent.log
%{ endif ~}
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

