# Storage Resources for Longhorn
###############################################################################
# Cinder Volumes for Agent Longhorn Storage
###############################################################################

resource "openstack_blockstorage_volume_v3" "agent_longhorn_storage" {
  count = var.enable_longhorn ? var.agent_count : 0

  name        = "${local.resource_prefix}-agent-${count.index}-longhorn"
  description = "Longhorn distributed storage volume for agent ${count.index}"
  size        = var.longhorn_storage_size

  metadata = merge(local.common_tags, {
    purpose           = "longhorn-storage"
    agent             = "${local.resource_prefix}-agent-${count.index}"
    terraform_managed = "true"
  })

  lifecycle {
    prevent_destroy = false
  }
}

###############################################################################
# Attach Volumes to Agent Instances
###############################################################################

resource "openstack_compute_volume_attach_v2" "agent_longhorn_attach" {
  count       = var.enable_longhorn ? var.agent_count : 0
  instance_id = openstack_compute_instance_v2.k3s_agent[count.index].id
  volume_id   = openstack_blockstorage_volume_v3.agent_longhorn_storage[count.index].id

  device = "/dev/vdb"
}
