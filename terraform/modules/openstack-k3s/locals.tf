# OpenStack K3s Cluster Module - Local Values
locals {
  # Cluster naming
  resource_prefix = var.cluster_name
  # Network configuration  
  subnet_cidr = var.network_cidr
  # OpenStack-specific settings from auth object
  auth_url    = var.openstack_auth.auth_url
  username    = var.openstack_auth.username
  password    = var.openstack_auth.password
  tenant_name = var.openstack_auth.tenant_name
  region      = var.openstack_auth.region
  cacert_file = var.openstack_auth.cacert_file
  # Instance configuration
  image_name = var.image_name
  # Tags
  common_tags = merge(var.tags, {
    module       = "openstack-k3s"
    cluster_name = var.cluster_name
  })

  # Tailscale configuration
  tailscale_prefix = var.tailscale_hostname_prefix != "" ? var.tailscale_hostname_prefix : var.cluster_name

  # CSI/Operator replica count - use min of server count and 3 for HA controllers
  operator_replica = min(var.server_count, 3)
}
