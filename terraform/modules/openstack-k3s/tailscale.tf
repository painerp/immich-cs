# Tailscale Ephemeral Auth Key Generation
# Auto-generates ephemeral keys per node for automatic cleanup when VMs are destroyed

###############################################################################
# Server Node Auth Keys
###############################################################################

resource "tailscale_tailnet_key" "server" {
  count = var.enable_tailscale ? var.server_count : 0

  reusable      = false # Single-use key
  ephemeral     = true  # Node auto-removed on shutdown
  preauthorized = true  # Auto-approve new nodes
  expiry        = var.tailscale_key_expiry
  description   = "k3s-${var.cluster_name}-server-${count.index}"

  tags = [
    "tag:k3s",
    "tag:openstack",
    "tag:server",
    "tag:${var.cluster_name}",
  ]
}

###############################################################################
# Agent Node Auth Keys
###############################################################################

resource "tailscale_tailnet_key" "agent" {
  count = var.enable_tailscale ? var.agent_count : 0

  reusable      = false
  ephemeral     = true
  preauthorized = true
  expiry        = var.tailscale_key_expiry
  description   = "k3s-${var.cluster_name}-agent-${count.index}"

  tags = [
    "tag:k3s",
    "tag:openstack",
    "tag:agent",
    "tag:${var.cluster_name}",
  ]
}

