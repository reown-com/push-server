variable "region" {
  type    = string
  default = "eu-central-1"
}

variable "app_name" {
  type = string
}

variable "environment" {
  type = string
}

variable "prometheus_workspace_id" {
  type = string
}

# AMP workspace ARN, scoped as the Resource of the dedicated Cloud role's aps:Query* policy
# (main.tf). Passed from the root (aws_prometheus_workspace.prometheus.arn).
variable "prometheus_workspace_arn" {
  description = "ARN of the AMP workspace, for the Grafana Cloud role's aps:Query* policy."
  type        = string
}

# Grafana Cloud grafana_assume_role trust inputs (NOT secrets — the public trust contract).
# Defaults are the live-verified values for our stack, read from mx-prod-grafana-cloudwatch.
variable "grafana_cloud_principal_arn" {
  description = "Grafana Labs assume-role account principal that Grafana Cloud uses to assume the per-account datasource role."
  type        = string
  default     = "arn:aws:iam::008923505280:root"

  validation {
    condition     = can(regex("^arn:aws:iam::[0-9]{12}:(root|user/.+|role/.+)$", var.grafana_cloud_principal_arn))
    error_message = "Must be a valid IAM principal ARN."
  }
}

variable "grafana_cloud_external_id" {
  description = "sts:ExternalId scoping the assume to our Grafana Cloud stack (confused-deputy guard). REQUIRED."
  type        = string
  default     = "1464025"

  validation {
    condition     = length(trimspace(var.grafana_cloud_external_id)) > 0
    error_message = "grafana_cloud_external_id must be set (confused-deputy guard)."
  }
}

variable "load_balancer_arn" {
  type = string
}

variable "notification_channels" {
  description = "The notification channels to send alerts to"
  type        = list(any)
}

variable "monitoring_role_arn" {
  description = "The ARN of the monitoring role."
  type        = string
}
