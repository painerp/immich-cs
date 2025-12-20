#cloud-config
#packages_update: true
#packages_upgrade: true
packages:
  - htop

write_files:
  - path: /run/cloud-config
    content: |
      [Global]
      #docu: https://github.com/kubernetes/cloud-provider-openstack
      #source: https://raw.githubusercontent.com/kubernetes/cloud-provider-openstack/master/manifests/controller-manager/cloud-config
      auth-url=${cloud_config_auth_url}
      username=${cloud_config_username}
      password=${cloud_config_password}
      region=${cloud_config_region}
      tenant-id=
      domain-id=

      [LoadBalancer]
      subnet-id=${cloud_config_subnet_id}
      floating-network-id=${cloud_config_floating_net}
%{ if enable_tailscale ~}
  - path: /usr/local/bin/setup-tailscale.sh
    permissions: "0755"
    owner: root:root
    content: |
      ${indent(6, tailscale_script)}
${tailscale_ip_updater_files}
%{ endif ~}
%{ for filename, content in manifests ~}
  - path: /var/lib/rancher/k3s/server/manifests/${filename}
    permissions: "0600"
    owner: root:root
    encoding: gz+b64
    content: ${content}
%{ endfor ~}

bootcmd:
  - [ cloud-init-per, once, first-boot, touch, /run/first-boot-timestamp ]

runcmd:
  # Wait for network to be fully configured
  - echo "Waiting for network to be ready..." >> /var/log/k3s-server.log
  - |
    #!/bin/bash
    for i in $(seq 1 30); do
      if ping -c 1 8.8.8.8 &>/dev/null && nslookup get.k3s.io &>/dev/null; then
        echo "Network is ready" >> /var/log/k3s-server.log
        break
      fi
      echo "Waiting for network (attempt $i/30)..." >> /var/log/k3s-server.log
      sleep 5
    done
%{ if enable_tailscale ~}
  # Setup Tailscale VPN
  - echo "Setting up Tailscale..." >> /var/log/k3s-server.log
  - /usr/local/bin/setup-tailscale.sh
%{ endif ~}
  # ${is_first_server ? "Initialize first k3s server with cluster-init flag for embedded etcd HA" : "Join k3s server to existing cluster"}
  - echo "${is_first_server ? "Initializing first k3s server" : "Joining k3s server to cluster"}..." >> /var/log/k3s-server.log
%{ if !is_first_server ~}
  - 'echo "First server IP: ${first_server_ip}" >> /var/log/k3s-server.log'
%{ endif ~}
  - |
    #!/bin/bash
    # Get the server's IP address
    SERVER_IP=$(hostname -I | awk '{print $1}')
    echo "Server IP: $SERVER_IP" >> /var/log/k3s-server.log
%{ if enable_tailscale ~}
    # Get Tailscale IP for external node IP
    TAILSCALE_IP=$(tailscale ip -4)
    echo "Tailscale IP: $TAILSCALE_IP" >> /var/log/k3s-server.log
%{ endif ~}
%{ if !is_first_server ~}
    # Wait for the first server to be available directly (not via load balancer)
    echo "Checking first server availability..." >> /var/log/k3s-server.log
    for i in $(seq 1 90); do
      if curl -k https://${first_server_ip}:6443/cacerts --connect-timeout 5 2>&1 | grep -q "BEGIN CERTIFICATE"; then
        echo "First server is reachable" >> /var/log/k3s-server.log
        break
      fi
      echo "Waiting for first server to be available (attempt $i/90)..." >> /var/log/k3s-server.log
      sleep 10
    done
%{ endif ~}
    # Install k3s with proper configuration
    curl -sfL https://get.k3s.io | K3S_TOKEN="${token}" sh -s - server \
%{ if is_first_server ~}
      --cluster-init \
%{ else ~}
      --server https://${first_server_ip}:6443 \
%{ endif ~}
      --disable-cloud-controller \
      --disable servicelb \
      --write-kubeconfig-mode 644 \
      --kubelet-arg="cloud-provider=external" \
%{ if is_first_server ~}
      --tls-san ${floating_ip} \
%{ endif ~}
      --tls-san ${lb_vip} \
      --advertise-address=$SERVER_IP \
      --node-ip=$SERVER_IP \
%{ if is_first_server ~}
      --bind-address=$SERVER_IP \
%{ endif ~}
%{ if enable_tailscale ~}
      --node-external-ip=$TAILSCALE_IP \
%{ endif ~}
%{ if is_first_server ~}
      --etcd-expose-metrics=true \
%{ endif ~}
      >> /var/log/k3s-server.log 2>&1
  # Wait for k3s to be ready
  - |
    #!/bin/bash
    echo "Waiting for k3s to be ready..." >> /var/log/k3s-server.log
    for i in $(seq 1 30); do
      if [ -f /etc/rancher/k3s/k3s.yaml ]%{ if is_first_server } && systemctl is-active --quiet k3s%{ endif }; then
        echo "k3s is ready" >> /var/log/k3s-server.log
        break
      fi
      echo "Waiting for k3s to be ready (attempt $i/30)..." >> /var/log/k3s-server.log
      sleep 5
    done
  - mkdir -p /home/ubuntu/.kube
  - |
    #!/bin/bash
    if cp /etc/rancher/k3s/k3s.yaml /home/ubuntu/.kube/config 2>> /var/log/k3s-server.log; then
      echo "Kubeconfig copied successfully" >> /var/log/k3s-server.log
    else
      echo "Failed to copy kubeconfig" >> /var/log/k3s-server.log
    fi
  - chown ubuntu:ubuntu /home/ubuntu/.kube/config
  - curl -fsSL -o /tmp/get_helm.sh https://raw.githubusercontent.com/helm/helm/master/scripts/get-helm-3
  - chmod 700 /tmp/get_helm.sh
  - /tmp/get_helm.sh
%{ if enable_tailscale ~}
  # Enable Tailscale IP updater
  - systemctl daemon-reload
  - systemctl enable tailscale-k3s-ip-updater.timer
  - systemctl start tailscale-k3s-ip-updater.timer
%{ endif ~}
  - echo "k3s server ${is_first_server ? "initialization" : "join"} complete" >> /var/log/k3s-server.log
%{ if is_first_server && enable_nvidia_gpu_operator ~}
  - echo "Installing NVIDIA GPU Operator..." >> /var/log/k3s-server.log
  - |
    ${indent(4, gpu_operator_script)}
%{ endif ~}
%{ if is_first_server && enable_argocd_with_tailscale ~}
  - echo "Installing ArgoCD..." >> /var/log/k3s-server.log
  - |
    ${indent(4, argocd_script)}
%{ endif ~}
%{ if enable_argocd_with_tailscale ~}
  - echo "Setting up Tailscale Serve for ArgoCD..." >> /var/log/k3s-server.log
  - |
    ${indent(4, tailscale_argocd_serve_script)}
%{ endif ~}
  - echo "Cloud-init runcmd complete" >> /var/log/k3s-server.log

