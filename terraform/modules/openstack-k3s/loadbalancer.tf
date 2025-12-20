# Load Balancer for HA K3s Control Plane
resource "openstack_lb_loadbalancer_v2" "k3s_lb" {
  count         = var.enable_load_balancer ? 1 : 0
  name          = "${local.resource_prefix}-lb"
  vip_subnet_id = openstack_networking_subnet_v2.subnet.id
  depends_on    = [openstack_networking_subnet_v2.subnet]
}
resource "openstack_lb_listener_v2" "k3s_listener" {
  count           = var.enable_load_balancer ? 1 : 0
  name            = "${local.resource_prefix}-api-listener"
  protocol        = "TCP"
  protocol_port   = 6443
  loadbalancer_id = openstack_lb_loadbalancer_v2.k3s_lb[0].id
}
resource "openstack_lb_pool_v2" "k3s_pool" {
  count       = var.enable_load_balancer ? 1 : 0
  name        = "${local.resource_prefix}-server-pool"
  protocol    = "TCP"
  lb_method   = "ROUND_ROBIN"
  listener_id = openstack_lb_listener_v2.k3s_listener[0].id
}
resource "openstack_lb_members_v2" "k3s_members" {
  count   = var.enable_load_balancer ? 1 : 0
  pool_id = openstack_lb_pool_v2.k3s_pool[0].id
  dynamic "member" {
    for_each = openstack_compute_instance_v2.k3s_server[*].access_ip_v4
    content {
      address       = member.value
      protocol_port = 6443
    }
  }
}
resource "openstack_lb_monitor_v2" "k3s_monitor" {
  count       = var.enable_load_balancer ? 1 : 0
  name        = "${local.resource_prefix}-api-monitor"
  pool_id     = openstack_lb_pool_v2.k3s_pool[0].id
  type        = "TCP"
  delay       = 10
  timeout     = 10
  max_retries = 5
  depends_on = [
    openstack_lb_loadbalancer_v2.k3s_lb,
    openstack_lb_listener_v2.k3s_listener,
    openstack_lb_pool_v2.k3s_pool,
    openstack_lb_members_v2.k3s_members
  ]
}
###############################################################################
# Floating IPs
###############################################################################
resource "openstack_networking_floatingip_v2" "fip_lb" {
  count      = var.enable_load_balancer ? 1 : 0
  pool       = var.floating_ip_pool
  port_id    = openstack_lb_loadbalancer_v2.k3s_lb[0].vip_port_id
  depends_on = [openstack_networking_router_interface_v2.router_interface]
}
resource "openstack_networking_floatingip_v2" "fip_bastion" {
  count      = var.enable_bastion ? 1 : 0
  pool       = var.floating_ip_pool
  port_id    = openstack_networking_port_v2.bastion_port[0].id
  depends_on = [openstack_networking_router_interface_v2.router_interface]
}
