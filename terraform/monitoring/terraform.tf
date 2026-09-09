terraform {
  required_version = ">= 1.0"

  required_providers {
    # Used by module.monitoring-role (AMG) and by the dedicated grafana-cloud role below.
    # Match the root pin (~> 4.31) — the IAM resources used are stable across 4.x.
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.31"
    }
    grafana = {
      source  = "grafana/grafana"
      version = ">= 3.0, < 5.0" # was "~> 2.0" (lock 2.19.0); bumped for the Cloud dual-run (see root backend.tf)
      # Receives the aliased grafana.cloud instance from the root (approach A). Without
      # this, provider = grafana.cloud on a resource in this module is an error.
      configuration_aliases = [grafana.cloud]
    }
    jsonnet = {
      source  = "alxrem/jsonnet"
      version = "~> 2.3.0"
    }
  }
}
