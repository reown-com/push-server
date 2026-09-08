provider "aws" {
  region = var.region

  # Make it faster by skipping something
  skip_metadata_api_check     = true
  skip_region_validation      = true
  skip_credentials_validation = true
  skip_requesting_account_id  = true

  default_tags {
    tags = module.tags.tags
  }
}

provider "grafana" {
  url  = "https://${data.terraform_remote_state.monitoring.outputs.grafana_workspaces.central.grafana_endpoint}"
  auth = var.grafana_auth
}

# AMG -> Grafana Cloud migration dual-run (approach A): a SECOND grafana provider
# instance aliased "cloud", alongside the default AMG provider above. Same module, same
# TFC workspace, same state. AMG resources stay on the default provider; the Grafana
# Cloud twins take provider = grafana.cloud. Removed once AMG is torn down.
provider "grafana" {
  alias = "cloud"
  url   = var.grafana_cloud_url
  auth  = var.grafana_cloud_token
}

provider "random" {}

provider "github" {}
