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
# Swift Container for Longhorn Backups
###############################################################################

resource "openstack_objectstorage_container_v1" "longhorn_backup" {
  count = var.enable_longhorn && var.enable_longhorn_backup ? 1 : 0

  name          = "${local.resource_prefix}-longhorn-backup"
  force_destroy = false

  metadata = {
    managed_by = "terraform"
    cluster    = var.cluster_name
    purpose    = "longhorn-backup"
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

###############################################################################
# EC2 Credentials for Swift S3 API Access
###############################################################################

# Create EC2 credentials for Longhorn to access Swift via S3 API
resource "openstack_identity_ec2_credential_v3" "longhorn_s3" {
  count = var.enable_longhorn && var.enable_longhorn_backup ? 1 : 0

  project_id = data.openstack_identity_auth_scope_v3.scope.project_id
}

