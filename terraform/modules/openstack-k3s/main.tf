# OpenStack K3s Cluster Module - Main Configuration

###############################################################################
# Configuration Checks
###############################################################################

# Require Tailscale provider configuration when Tailscale is enabled
check "tailscale_config_required" {
  assert {
    condition     = !var.enable_tailscale || (var.tailscale_api_key != "" && var.tailscale_tailnet != "")
    error_message = <<-EOT
      ERROR: Tailscale is enabled but provider configuration is missing.

      When enable_tailscale=true, you must provide:
      - tailscale_api_key: OAuth client secret from Tailscale admin console
      - tailscale_tailnet: Your tailnet identifier (e.g., user@example.com or example.com)

      To create an OAuth client:
      1. Visit: https://login.tailscale.com/admin/settings/oauth
      2. Click "Generate OAuth Client"
      3. Select scopes: devices:write, auth_keys:write
      4. Copy the client secret (tskey-api-xxxxx-yyyyyyyy)

      Add to terraform.tfvars:
        enable_tailscale   = true
        tailscale_api_key  = "tskey-api-xxxxx-yyyyyyyy"
        tailscale_tailnet  = "your-tailnet"
    EOT
  }
}

# Require Tailscale when ArgoCD is enabled
check "argocd_tailscale_dependency" {
  assert {
    condition     = !var.enable_argocd || var.enable_tailscale
    error_message = <<-EOT
      ERROR: ArgoCD GitOps is enabled but Tailscale is not enabled.
        When enable_argocd=true, Tailscale must also be enabled (enable_tailscale=true)
        to allow secure access to the ArgoCD server via Tailscale Serve.
        To fix, either:
        1. Set enable_tailscale=true
        OR
        2. Set enable_argocd=false
    EOT
  }
}

# Soft warning when both bastion and Tailscale are enabled
# Bastion becomes redundant when Tailscale SSH is available
check "bastion_tailscale_redundancy" {
  assert {
    condition     = !(var.enable_bastion && var.enable_tailscale)
    error_message = <<-EOT
      WARNING: Both bastion host and Tailscale are enabled.

      When Tailscale is enabled, the bastion host becomes redundant as you can SSH
      directly to nodes via Tailscale hostnames (e.g., ssh ubuntu@${local.tailscale_prefix}-server-0).

      Consider setting enable_bastion=false to save costs, or keep it enabled during
      migration from bastion to Tailscale SSH.

      This is a warning only - your deployment will proceed.
    EOT
  }
}

###############################################################################
# Cloud Controller Manifests
###############################################################################

locals {
  # Calculate total nodes for GPU Operator script
  total_nodes                    = var.server_count + var.agent_count
  enable_argocd_with_tailscale   = var.enable_argocd && var.enable_tailscale
  enable_longhorn_with_tailscale = var.enable_longhorn && var.enable_tailscale

  # Generate GPU Operator installation script
  gpu_operator_install_script = var.enable_nvidia_gpu_operator ? templatefile("${path.root}/templates/gpu-operator-install.tpl", {
    total_nodes = local.total_nodes
  }) : ""

  # Generate ArgoCD installation script
  argocd_install_script = local.enable_argocd_with_tailscale ? templatefile("${path.root}/templates/argocd-install.tpl", {
    total_nodes    = local.total_nodes
    repo_url       = var.argocd_repo_url
    repo_branch    = var.argocd_repo_branch
    admin_password = var.argocd_admin_password
  }) : ""

  # Generate cloud controller manifest for K3s
  manifests_raw = merge(
    {
      "cloud-controller-openstack.yaml" = templatefile("${path.root}/manifests/cloud-controller-openstack.yaml.tpl", {
        auth_url            = local.auth_url
        region              = local.region
        project_id          = data.openstack_identity_auth_scope_v3.scope.project_id
        app_id              = openstack_identity_application_credential_v3.k3s.id
        app_secret          = openstack_identity_application_credential_v3.k3s.secret
        app_name            = openstack_identity_application_credential_v3.k3s.name
        network_id          = openstack_networking_network_v2.network.id
        subnet_id           = openstack_networking_subnet_v2.subnet.id
        floating_network_id = data.openstack_networking_network_v2.fip_network.id
        lb_provider         = var.lb_provider
        cluster_name        = var.cluster_name
        insecure            = var.insecure
      })
    },
    var.enable_tailscale && var.tailscale_oauth_client_id != "" && var.tailscale_oauth_client_secret != "" ? {
      "tailscale-oauth-secret.yaml" = templatefile("${path.root}/templates/tailscale-oauth-secret.tpl", {
        client_id_b64     = local.tailscale_oauth_client_id_b64
        client_secret_b64 = local.tailscale_oauth_client_secret_b64
      })
    } : {},
    var.enable_csi_cinder ? {
      "csi-cinder.yaml" = templatefile("${path.root}/manifests/csi-cinder.yaml.tpl", {
        operator_replica = local.operator_replica
        auth_url         = local.auth_url
        region           = local.region
        project_id       = data.openstack_identity_auth_scope_v3.scope.project_id
        app_id           = openstack_identity_application_credential_v3.k3s.id
        app_secret       = openstack_identity_application_credential_v3.k3s.secret
        app_name         = openstack_identity_application_credential_v3.k3s.name
        insecure         = var.insecure
      })
      "csi-cinder-retain.yaml" = templatefile("${path.root}/manifests/csi-cinder-sc.yaml.tpl", {
        name          = "csi-cinder-retain"
        reclaimPolicy = "Retain"
        is_default    = var.cinder_default_reclaim_policy == "Retain"
        parameters    = {}
      })
      "csi-cinder-delete.yaml" = templatefile("${path.root}/manifests/csi-cinder-sc.yaml.tpl", {
        name          = "csi-cinder-delete"
        reclaimPolicy = "Delete"
        is_default    = var.cinder_default_reclaim_policy == "Delete"
        parameters    = {}
      })
      "csi-cinder-snapclass.yaml" = file("${path.root}/manifests/csi-cinder-snapclass.yaml")
    } : {},
    var.enable_longhorn ? {
      "longhorn.yaml" = templatefile("${path.root}/manifests/longhorn.yaml.tpl", {
        replica_count  = var.longhorn_replica_count
        backup_enabled = local.longhorn_backup_enabled
        backup_target  = local.longhorn_backup_target
      })
    } : {},
    local.longhorn_backup_enabled ? {
      "longhorn-backup-secret.yaml" = templatefile("${path.root}/manifests/longhorn-backup-secret.yaml.tpl", {
        access_key  = openstack_identity_ec2_credential_v3.longhorn_s3[0].access
        secret_key  = openstack_identity_ec2_credential_v3.longhorn_s3[0].secret
        s3_endpoint = local.longhorn_s3_endpoint
        ca_cert     = fileexists(local.cacert_file) ? base64encode(trimspace(file(local.cacert_file))) : ""
      })
      "longhorn-recurring-backup.yaml" = templatefile("${path.root}/manifests/longhorn-recurring-backup.yaml.tpl", {
        backup_cron        = var.longhorn_backup_schedule
        backup_retain      = var.longhorn_backup_retention
        backup_concurrency = var.longhorn_backup_concurrency
      })
    } : {}
  )

  # Encode manifests for write_files with gz+b64 encoding
  manifests_encoded = {
    for k, v in local.manifests_raw : k => base64gzip(v)
  }
}

