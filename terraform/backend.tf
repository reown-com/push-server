# Terraform Configuration
terraform {
  required_version = "~> 1.0"

  backend "remote" {
    hostname     = "app.terraform.io"
    organization = "wallet-connect"
    workspaces {
      prefix = "echo-server-"
    }
  }

  required_providers {
    assert = {
      source = "bwoznicki/assert"
    }
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.31"
    }
    grafana = {
      source = "grafana/grafana"
      # Bumped from ">= 2.1"/module "~> 2.0" (lock 2.19.0) for the AMG -> Grafana Cloud
      # dual-run: the aliased grafana.cloud + grafana_assume_role datasources need a modern
      # provider. Resource types used (data_source/dashboard) are stable 2.x->4.x.
      version = ">= 3.0, < 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "3.4.3"
    }
    github = {
      source  = "integrations/github"
      version = "5.7.0"
    }
  }
}
