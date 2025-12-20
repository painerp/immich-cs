# Network Resources
resource "openstack_networking_network_v2" "network" {
  name           = "${local.resource_prefix}-network"
  admin_state_up = true
}
resource "openstack_networking_subnet_v2" "subnet" {
  name            = "${local.resource_prefix}-subnet"
  network_id      = openstack_networking_network_v2.network.id
  cidr            = local.subnet_cidr
  ip_version      = 4
  dns_nameservers = var.dns_servers
}
resource "openstack_networking_router_v2" "router" {
  name                = "${local.resource_prefix}-router"
  admin_state_up      = true
  external_network_id = data.openstack_networking_network_v2.fip_network.id
}
resource "openstack_networking_router_interface_v2" "router_interface" {
  router_id = openstack_networking_router_v2.router.id
  subnet_id = openstack_networking_subnet_v2.subnet.id
}
###############################################################################
# Ports for K3s Servers
###############################################################################
resource "openstack_networking_port_v2" "server_port" {
  count              = var.server_count
  name               = "${local.resource_prefix}-server-${count.index}-port"
  network_id         = openstack_networking_network_v2.network.id
  admin_state_up     = true
  security_group_ids = [openstack_networking_secgroup_v2.server.id]
  fixed_ip {
    subnet_id = openstack_networking_subnet_v2.subnet.id
  }
}
###############################################################################
# Port for Bastion Host
###############################################################################
resource "openstack_networking_port_v2" "bastion_port" {
  count              = var.enable_bastion ? 1 : 0
  name               = "${local.resource_prefix}-bastion-port"
  network_id         = openstack_networking_network_v2.network.id
  admin_state_up     = true
  security_group_ids = [openstack_networking_secgroup_v2.bastion[0].id]
  fixed_ip {
    subnet_id = openstack_networking_subnet_v2.subnet.id
  }
}
