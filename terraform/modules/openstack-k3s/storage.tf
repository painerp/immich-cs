# Storage Resources for Longhorn
###############################################################################
# Data Source: Check for Existing Longhorn Volumes
###############################################################################

data "openstack_blockstorage_volume_v3" "existing_agent_longhorn" {
  count = var.enable_longhorn && !var.force_recreate_volumes ? var.agent_count : 0
  name  = "${local.resource_prefix}-agent-${count.index}-longhorn"

  # If volume doesn't exist, this will fail - that's expected and handled
  lifecycle {
    postcondition {
      condition     = self.id != ""
      error_message = "Volume ${local.resource_prefix}-agent-${count.index}-longhorn not found. Set force_recreate_volumes=true to create new volumes."
    }
  }
}

###############################################################################
# Cinder Volumes for Agent Longhorn Storage (Create New)
###############################################################################

resource "openstack_blockstorage_volume_v3" "agent_longhorn_storage" {
  count = var.enable_longhorn && var.force_recreate_volumes ? var.agent_count : 0

  name        = "${local.resource_prefix}-agent-${count.index}-longhorn"
  description = "Longhorn distributed storage volume for agent ${count.index}"
  size        = var.longhorn_storage_size

  metadata = merge(local.common_tags, {
    purpose           = "longhorn-storage"
    agent             = "${local.resource_prefix}-agent-${count.index}"
    terraform_managed = "true"
    created_at        = timestamp()
  })

  # Prevent accidental deletion when not forcing recreation
  lifecycle {
    prevent_destroy = false
    ignore_changes = var.force_recreate_volumes ? [] : [
      metadata["created_at"]
    ]
  }
}

###############################################################################
# Local to Determine Which Volume IDs to Use
###############################################################################

locals {
  # Use existing volumes if available, otherwise use newly created ones
  agent_longhorn_volume_ids = var.enable_longhorn ? (
    var.force_recreate_volumes ?
      openstack_blockstorage_volume_v3.agent_longhorn_storage[*].id :
      data.openstack_blockstorage_volume_v3.existing_agent_longhorn[*].id
  ) : []
}

###############################################################################
# Attach Volumes to Agent Instances
###############################################################################

resource "openstack_compute_volume_attach_v2" "agent_longhorn_attach" {
  count       = var.enable_longhorn ? var.agent_count : 0
  instance_id = openstack_compute_instance_v2.k3s_agent[count.index].id
  volume_id   = local.agent_longhorn_volume_ids[count.index]

  device = "/dev/vdb"

  depends_on = [
    openstack_blockstorage_volume_v3.agent_longhorn_storage,
    data.openstack_blockstorage_volume_v3.existing_agent_longhorn
  ]
}
