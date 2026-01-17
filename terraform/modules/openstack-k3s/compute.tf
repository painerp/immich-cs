# Compute Instance Resources
###############################################################################
# K3s Server Instances (Control Plane)
###############################################################################
resource "openstack_compute_instance_v2" "k3s_server" {
  count           = var.server_count
  name            = "${local.resource_prefix}-server-${count.index}"
  image_name      = local.image_name
  flavor_name     = var.server_flavor
  key_pair        = openstack_compute_keypair_v2.keypair.name
  security_groups = [openstack_networking_secgroup_v2.server.name]
  depends_on      = [openstack_networking_subnet_v2.subnet]

  network {
    port = openstack_networking_port_v2.server_port[count.index].id
  }

  user_data = templatefile("${path.root}/templates/k3s-server.tpl", {
    is_first_server              = count.index == 0
    token                        = var.k3s_token
    first_server_ip              = count.index == 0 ? "" : openstack_networking_port_v2.server_port[0].all_fixed_ips[0]
    floating_ip                  = var.enable_load_balancer ? openstack_networking_floatingip_v2.fip_lb[0].address : ""
    lb_vip                       = var.enable_load_balancer ? openstack_lb_loadbalancer_v2.k3s_lb[0].vip_address : ""
    cloud_config_auth_url        = local.auth_url
    cloud_config_username        = local.username
    cloud_config_password        = local.password
    cloud_config_region          = local.region
    cloud_config_subnet_id       = openstack_networking_subnet_v2.subnet.id
    cloud_config_floating_net    = data.openstack_networking_network_v2.fip_network.id
    enable_tailscale             = var.enable_tailscale
    manifests                    = local.manifests_encoded
    enable_nvidia_gpu_operator   = var.enable_nvidia_gpu_operator
    gpu_operator_script          = local.gpu_operator_install_script
    enable_argocd_with_tailscale = local.enable_argocd_with_tailscale
    argocd_script                = local.argocd_install_script
    enable_longhorn_with_tailscale = local.enable_longhorn_with_tailscale
    tailscale_argocd_serve_script = local.enable_argocd_with_tailscale ? templatefile("${path.root}/templates/tailscale-argocd-serve.tpl", {
      argocd_wait_timeout = count.index == 0 ? 120 : 1200
      admin_password      = var.argocd_admin_password
    }) : ""
    tailscale_longhorn_serve_script = local.enable_longhorn_with_tailscale ? templatefile("${path.root}/templates/tailscale-longhorn-serve.tpl", {
      longhorn_wait_timeout = count.index == 0 ? 120 : 1200
    }) : ""
    tailscale_script = var.enable_tailscale ? templatefile("${path.root}/templates/tailscale-install.tpl", {
      tailscale_auth_key = tailscale_tailnet_key.server[count.index].key
      tailscale_hostname = "${local.tailscale_prefix}-server-${count.index}"
    }) : ""
    tailscale_ip_updater_files = var.enable_tailscale ? templatefile("${path.root}/templates/tailscale-ip-updater.tpl", {
      tailscale_ip_update_interval = var.tailscale_ip_update_interval
      k3s_service                  = "k3s"
    }) : ""
  })
}
###############################################################################
# K3s Agent Instances (Workers)
###############################################################################
resource "openstack_compute_instance_v2" "k3s_agent" {
  count           = var.agent_count
  name            = "${local.resource_prefix}-agent-${count.index}"
  image_name      = local.image_name
  flavor_name     = var.agent_flavor
  key_pair        = openstack_compute_keypair_v2.keypair.name
  security_groups = [openstack_networking_secgroup_v2.agent.name]
  depends_on      = [openstack_networking_subnet_v2.subnet]

  network {
    uuid = openstack_networking_network_v2.network.id
  }

  user_data = templatefile("${path.root}/templates/k3s-agent.tpl", {
    k3s_token                    = var.k3s_token
    k3s_url                      = "https://${var.enable_load_balancer ? openstack_lb_loadbalancer_v2.k3s_lb[0].vip_address : openstack_compute_instance_v2.k3s_server[0].access_ip_v4}:6443"
    enable_longhorn              = var.enable_longhorn
    enable_tailscale             = var.enable_tailscale
    tailscale_ip_update_interval = var.tailscale_ip_update_interval
    tailscale_script = var.enable_tailscale ? templatefile("${path.root}/templates/tailscale-install.tpl", {
      tailscale_auth_key = tailscale_tailnet_key.agent[count.index].key
      tailscale_hostname = "${local.tailscale_prefix}-agent-${count.index}"
    }) : ""
    tailscale_ip_updater_files = var.enable_tailscale ? templatefile("${path.root}/templates/tailscale-ip-updater.tpl", {
      tailscale_ip_update_interval = var.tailscale_ip_update_interval
      k3s_service                  = "k3s-agent"
    }) : ""
  })
}
###############################################################################
# Bastion Host
###############################################################################
resource "openstack_compute_instance_v2" "bastion" {
  count           = var.enable_bastion ? 1 : 0
  name            = "${local.resource_prefix}-bastion"
  image_name      = local.image_name
  flavor_name     = var.bastion_flavor
  key_pair        = openstack_compute_keypair_v2.keypair.name
  security_groups = [openstack_networking_secgroup_v2.bastion[0].name]
  network {
    port = openstack_networking_port_v2.bastion_port[0].id
  }
}
