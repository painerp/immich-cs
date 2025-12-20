# Data Sources
data "openstack_networking_network_v2" "fip_network" {
  name = var.floating_ip_pool
}

# Get current project/tenant information
data "openstack_identity_auth_scope_v3" "scope" {
  name = var.openstack_auth.tenant_name
}

