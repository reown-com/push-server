locals {
  # Turns the arn into the format expected by
  # the Grafana provider e.g.
  # net/prod-relay-load-balancer/e9a51c46020a0f85
  load_balancer = join("/", slice(split("/", var.load_balancer_arn), 1, 4))
}

module "monitoring-role" {
  source          = "app.terraform.io/wallet-connect/monitoring-role/aws"
  version         = "1.1.0"
  context         = module.this
  remote_role_arn = var.monitoring_role_arn
}

resource "grafana_data_source" "prometheus" {
  type = "prometheus"
  name = "${var.app_name}-amp"
  url  = "https://aps-workspaces.eu-central-1.amazonaws.com/workspaces/${var.prometheus_workspace_id}/"

  json_data_encoded = jsonencode({
    httpMethod         = "GET"
    manageAlerts       = false
    sigV4Auth          = true
    sigV4AuthType      = "ec2_iam_role"
    sigV4Region        = "eu-central-1"
    sigV4AssumeRoleArn = module.monitoring-role.iam_role_arn
  })
}

resource "grafana_data_source" "cloudwatch" {
  type = "cloudwatch"
  name = "${var.app_name}-cloudwatch"

  json_data_encoded = jsonencode({
    defaultRegion = "eu-central-1"
    assumeRoleArn = module.monitoring-role.iam_role_arn
  })
}

data "jsonnet_file" "dashboard" {
  source = "${path.module}/dashboard.jsonnet"

  ext_str = {
    dashboard_title = "Push Server - ${title(var.environment)}"
    dashboard_uid   = "push-${var.environment}"

    prometheus_uid = grafana_data_source.prometheus.uid
    cloudwatch_uid = grafana_data_source.cloudwatch.uid

    environment   = var.environment
    notifications = jsonencode(var.notification_channels)
  }
}

resource "grafana_dashboard" "push_server" {
  overwrite   = true
  message     = "Updated by Terraform"
  config_json = data.jsonnet_file.dashboard.rendered
}

