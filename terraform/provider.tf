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

# AMG workspace provider. All AMG resources have been removed; this block is kept only so
# the AMG resources deleted in this change can be destroyed on apply. Remove it (and the
# `grafana = grafana` pass in main.tf) in a follow-up once those destroys have applied.
provider "grafana" {
  url  = "https://g-aa89c04cfd.grafana-workspace.eu-central-1.amazonaws.com"
  auth = var.grafana_auth
}

# Grafana Cloud provider (aliased).
provider "grafana" {
  alias = "cloud"
  url   = var.grafana_cloud_url
  auth  = var.grafana_cloud_token
}

provider "random" {}

provider "github" {}
