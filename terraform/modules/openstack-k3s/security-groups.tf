# Security Groups and Firewall Rules
###############################################################################
# Security Groups
###############################################################################
# Security group for K3s server nodes (control plane)
resource "openstack_networking_secgroup_v2" "server" {
  name                 = "${local.resource_prefix}-server"
  description          = "Security group for K3s server nodes"
  delete_default_rules = true
}
# Security group for K3s agent nodes (workers)
resource "openstack_networking_secgroup_v2" "agent" {
  name                 = "${local.resource_prefix}-agent"
  description          = "Security group for K3s agent nodes"
  delete_default_rules = true
}
# Security group for bastion host
resource "openstack_networking_secgroup_v2" "bastion" {
  count                = var.enable_bastion ? 1 : 0
  name                 = "${local.resource_prefix}-bastion"
  description          = "Security group for bastion/jump host"
  delete_default_rules = true
}
###############################################################################
# Egress Rules - Allow all outbound traffic
###############################################################################
resource "openstack_networking_secgroup_rule_v2" "server_egress" {
  direction         = "egress"
  ethertype         = "IPv4"
  security_group_id = openstack_networking_secgroup_v2.server.id
}
resource "openstack_networking_secgroup_rule_v2" "agent_egress" {
  direction         = "egress"
  ethertype         = "IPv4"
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
resource "openstack_networking_secgroup_rule_v2" "bastion_egress" {
  count             = var.enable_bastion ? 1 : 0
  direction         = "egress"
  ethertype         = "IPv4"
  security_group_id = openstack_networking_secgroup_v2.bastion[0].id
}
###############################################################################
# External Access Rules for Server Nodes
# TODO: Restrict SSH and K8s API to specific CIDR ranges for production use
###############################################################################
# SSH access from external CIDRs
resource "openstack_networking_secgroup_rule_v2" "server_ssh_external" {
  for_each          = toset(var.external_ssh_cidrs)
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 22
  port_range_max    = 22
  remote_ip_prefix  = each.value
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# K8s API access from external CIDRs
resource "openstack_networking_secgroup_rule_v2" "server_k8s_api_external" {
  for_each          = toset(var.external_api_cidrs)
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 6443
  port_range_max    = 6443
  remote_ip_prefix  = each.value
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# K8s API access from load balancer subnet
resource "openstack_networking_secgroup_rule_v2" "server_k8s_api_lb" {
  count             = var.enable_load_balancer ? 1 : 0
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 6443
  port_range_max    = 6443
  remote_ip_prefix  = local.subnet_cidr
  security_group_id = openstack_networking_secgroup_v2.server.id
}
###############################################################################
# Bastion Host Rules
###############################################################################
# SSH access to bastion from external CIDRs
resource "openstack_networking_secgroup_rule_v2" "bastion_ssh_external" {
  for_each          = var.enable_bastion ? toset(var.external_ssh_cidrs) : []
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 22
  port_range_max    = 22
  remote_ip_prefix  = each.value
  security_group_id = openstack_networking_secgroup_v2.bastion[0].id
}
# Bastion can SSH to servers
resource "openstack_networking_secgroup_rule_v2" "server_ssh_from_bastion" {
  count             = var.enable_bastion ? 1 : 0
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 22
  port_range_max    = 22
  remote_group_id   = openstack_networking_secgroup_v2.bastion[0].id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Bastion can SSH to agents
resource "openstack_networking_secgroup_rule_v2" "agent_ssh_from_bastion" {
  count             = var.enable_bastion ? 1 : 0
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 22
  port_range_max    = 22
  remote_group_id   = openstack_networking_secgroup_v2.bastion[0].id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
###############################################################################
# K3s etcd Rules - Server to Server Only (HA Embedded etcd)
###############################################################################
resource "openstack_networking_secgroup_rule_v2" "server_etcd" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 2379
  port_range_max    = 2380
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
###############################################################################
# K3s API Server Rules (port 6443)
###############################################################################
# Server to server K8s API
resource "openstack_networking_secgroup_rule_v2" "server_k8s_api_server_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 6443
  port_range_max    = 6443
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Agent to server K8s API
resource "openstack_networking_secgroup_rule_v2" "server_k8s_api_agent_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 6443
  port_range_max    = 6443
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
###############################################################################
# Kubelet Metrics (port 10250) - All Nodes to All Nodes
###############################################################################
# Server to server
resource "openstack_networking_secgroup_rule_v2" "server_kubelet_server_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 10250
  port_range_max    = 10250
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Server to agent
resource "openstack_networking_secgroup_rule_v2" "agent_kubelet_server_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 10250
  port_range_max    = 10250
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
# Agent to server
resource "openstack_networking_secgroup_rule_v2" "server_kubelet_agent_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 10250
  port_range_max    = 10250
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Agent to agent
resource "openstack_networking_secgroup_rule_v2" "agent_kubelet_agent_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 10250
  port_range_max    = 10250
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
###############################################################################
# Flannel VXLAN (UDP 8472) - All Nodes to All Nodes
###############################################################################
# Server to server
resource "openstack_networking_secgroup_rule_v2" "server_vxlan_server_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 8472
  port_range_max    = 8472
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Server to agent
resource "openstack_networking_secgroup_rule_v2" "agent_vxlan_server_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 8472
  port_range_max    = 8472
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
# Agent to server
resource "openstack_networking_secgroup_rule_v2" "server_vxlan_agent_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 8472
  port_range_max    = 8472
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Agent to agent
resource "openstack_networking_secgroup_rule_v2" "agent_vxlan_agent_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 8472
  port_range_max    = 8472
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
###############################################################################
# Flannel Wireguard (UDP 51820) - All Nodes to All Nodes
# For future use if switching from VXLAN to Wireguard
###############################################################################
# Server to server
resource "openstack_networking_secgroup_rule_v2" "server_wireguard_server_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 51820
  port_range_max    = 51820
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Server to agent
resource "openstack_networking_secgroup_rule_v2" "agent_wireguard_server_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 51820
  port_range_max    = 51820
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
# Agent to server
resource "openstack_networking_secgroup_rule_v2" "server_wireguard_agent_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 51820
  port_range_max    = 51820
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Agent to agent
resource "openstack_networking_secgroup_rule_v2" "agent_wireguard_agent_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 51820
  port_range_max    = 51820
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
###############################################################################
# Spegel Distributed Registry (TCP 5001) - All Nodes to All Nodes
###############################################################################
# Server to server
resource "openstack_networking_secgroup_rule_v2" "server_spegel_server_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 5001
  port_range_max    = 5001
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Server to agent
resource "openstack_networking_secgroup_rule_v2" "agent_spegel_server_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 5001
  port_range_max    = 5001
  remote_group_id   = openstack_networking_secgroup_v2.server.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}
# Agent to server
resource "openstack_networking_secgroup_rule_v2" "server_spegel_agent_to_server" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 5001
  port_range_max    = 5001
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.server.id
}
# Agent to agent
resource "openstack_networking_secgroup_rule_v2" "agent_spegel_agent_to_agent" {
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "tcp"
  port_range_min    = 5001
  port_range_max    = 5001
  remote_group_id   = openstack_networking_secgroup_v2.agent.id
  security_group_id = openstack_networking_secgroup_v2.agent.id
}

###############################################################################
# Tailscale WireGuard (UDP 41641) - External Access
# Required for Tailscale mesh VPN connectivity
###############################################################################

# Server nodes Tailscale access
resource "openstack_networking_secgroup_rule_v2" "server_tailscale_external" {
  count             = var.enable_tailscale ? 1 : 0
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 41641
  port_range_max    = 41641
  remote_ip_prefix  = "0.0.0.0/0"
  security_group_id = openstack_networking_secgroup_v2.server.id
}

# Agent nodes Tailscale access
resource "openstack_networking_secgroup_rule_v2" "agent_tailscale_external" {
  count             = var.enable_tailscale ? 1 : 0
  direction         = "ingress"
  ethertype         = "IPv4"
  protocol          = "udp"
  port_range_min    = 41641
  port_range_max    = 41641
  remote_ip_prefix  = "0.0.0.0/0"
  security_group_id = openstack_networking_secgroup_v2.agent.id
}

