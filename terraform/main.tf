locals {
  app_name    = "push"
  environment = terraform.workspace

  fqdn        = local.environment == "prod" ? var.public_url : "${local.environment}.${var.public_url}"
  backup_fqdn = replace(local.fqdn, ".com", ".org")

  latest_release_name = data.github_release.latest_release.name
  version             = coalesce(var.image_version, substr(local.latest_release_name, 1, length(local.latest_release_name)))

  database_url        = "postgres://${module.database_cluster.cluster_master_username}:${module.database_cluster.cluster_master_password}@${module.database_cluster.cluster_endpoint}:${module.database_cluster.cluster_port}/postgres"
  tenant_database_url = "postgres://${module.tenant_database_cluster.cluster_master_username}:${module.tenant_database_cluster.cluster_master_password}@${module.tenant_database_cluster.cluster_endpoint}:${module.tenant_database_cluster.cluster_port}/postgres"

  geoip_db_bucket_name            = "${local.environment}.relay.geo.ip.database.private.${local.environment}.walletconnect"
  analytics_data_lake_bucket_name = "walletconnect.data-lake.${local.environment}"
}

#tflint-ignore: terraform_required_providers,terraform_unused_declarations
data "assert_test" "workspace" {
  test  = terraform.workspace != "default"
  throw = "default workspace is not valid in this project"
}

data "github_release" "latest_release" {
  repository  = "echo-server"
  owner       = "walletconnect"
  retrieve_by = "latest"
}

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "3.19.0"

  name = "${local.environment}-${local.app_name}"

  cidr = "10.0.0.0/16"

  azs             = var.azs
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.4.0/24", "10.0.5.0/24", "10.0.6.0/24"]

  private_subnet_tags = {
    Visibility = "private"
  }
  public_subnet_tags = {
    Visibility = "public"
  }

  enable_dns_support     = true
  enable_dns_hostnames   = true
  enable_nat_gateway     = true
  single_nat_gateway     = true
  one_nat_gateway_per_az = false
}

module "tags" {
  source = "git::https://github.com/WalletConnect/terraform-modules.git//modules/tags?ref=52a74ee5bcaf5cacb5664c6f88d9dbce28500581"

  application = local.app_name
  env         = local.environment
}

module "dns" {
  source = "git::https://github.com/WalletConnect/terraform-modules.git//modules/dns?ref=52a74ee5bcaf5cacb5664c6f88d9dbce28500581"

  hosted_zone_name = var.public_url
  fqdn             = local.fqdn
}

module "backup_dns" {
  source = "git::https://github.com/WalletConnect/terraform-modules.git//modules/dns?ref=52a74ee5bcaf5cacb5664c6f88d9dbce28500581"

  hosted_zone_name = replace(var.public_url, ".com", ".org")
  fqdn             = local.backup_fqdn
}

module "database_cluster" {
  source  = "terraform-aws-modules/rds-aurora/aws"
  version = "7.7.0"

  name           = "${local.environment}-${local.app_name}-database"
  engine         = "aurora-postgresql"
  engine_version = "13.12"
  engine_mode    = "provisioned"
  instance_class = "db.serverless"
  instances = {
    1 = {}
  }

  database_name = "postgres"

  vpc_id  = module.vpc.vpc_id
  subnets = module.vpc.private_subnets

  allowed_cidr_blocks = [module.vpc.vpc_cidr_block]

  storage_encrypted = true
  apply_immediately = true

  allow_major_version_upgrade = true

  monitoring_interval             = 30
  enabled_cloudwatch_logs_exports = ["postgresql"]

  serverlessv2_scaling_configuration = {
    min_capacity = local.environment == "prod" ? 1 : 0.5
    max_capacity = 20
  }
}

module "tenant_database_cluster" {
  source  = "terraform-aws-modules/rds-aurora/aws"
  version = "7.7.0"

  name           = "${local.environment}-${local.app_name}-tenant-database"
  engine         = "aurora-postgresql"
  engine_version = "13.12"
  engine_mode    = "provisioned"
  instance_class = "db.serverless"
  instances = {
    1 = {}
  }

  database_name = "postgres"

  vpc_id  = module.vpc.vpc_id
  subnets = module.vpc.private_subnets

  allowed_cidr_blocks = [module.vpc.vpc_cidr_block]

  storage_encrypted = true
  apply_immediately = true

  allow_major_version_upgrade = true

  monitoring_interval             = 30
  enabled_cloudwatch_logs_exports = ["postgresql"]

  serverlessv2_scaling_configuration = {
    min_capacity = local.environment == "prod" ? 1 : 0.5
    max_capacity = 20
  }
}

module "ecs" {
  source = "./ecs"

  app_name                   = "${local.environment}-${local.app_name}"
  environment                = local.environment
  prometheus_endpoint        = aws_prometheus_workspace.prometheus.prometheus_endpoint
  database_url               = local.database_url
  tenant_database_url        = local.tenant_database_url
  image                      = "${data.aws_ecr_repository.repository.repository_url}:${local.version}"
  image_version              = local.version
  acm_certificate_arn        = module.dns.certificate_arn
  cpu                        = local.environment == "prod" ? 512 : 256
  fqdn                       = local.fqdn
  memory                     = local.environment == "prod" ? 1024 : 512
  private_subnets            = module.vpc.private_subnets
  public_subnets             = module.vpc.public_subnets
  region                     = var.region
  route53_zone_id            = module.dns.zone_id
  backup_acm_certificate_arn = module.backup_dns.certificate_arn
  backup_fqdn                = local.backup_fqdn
  backup_route53_zone_id     = module.backup_dns.zone_id
  vpc_cidr                   = module.vpc.vpc_cidr_block
  vpc_id                     = module.vpc.vpc_id
  telemetry_sample_ratio     = local.environment == "prod" ? 0.25 : 1.0
  allowed_origins            = local.environment == "prod" ? "https://cloud.walletconnect.com" : "*"

  aws_otel_collector_ecr_repository_url = data.aws_ecr_repository.aws_otel_collector.repository_url

  analytics_datalake_bucket_name = local.analytics_data_lake_bucket_name
  analytics_key_arn              = local.environment == "prod" ? "arn:aws:kms:eu-central-1:898587786287:key/06e7c9fd-943d-47bf-bcf4-781b44411ba4" : "arn:aws:kms:eu-central-1:898587786287:key/d1d2f047-b2a3-4f4a-8786-7c87ee83c954"
  analytics_geoip_db_bucket_name = local.geoip_db_bucket_name
  analytics_geoip_db_key         = var.geoip_db_key

  jwt_secret       = var.jwt_secret
  relay_public_key = var.relay_public_key

  autoscaling_max_capacity = local.environment == "prod" ? 10 : 1
  autoscaling_min_capacity = local.environment == "prod" ? 2 : 1
  desired_count            = local.environment == "prod" ? 2 : 1
}


module "monitoring" {
  source = "./monitoring"

  app_name                = "${local.environment}-${local.app_name}"
  prometheus_workspace_id = aws_prometheus_workspace.prometheus.id
  load_balancer_arn       = module.ecs.load_balancer_arn
  environment             = local.environment
  notification_channels   = var.notification_channels
}

data "aws_ecr_repository" "repository" {
  name = "echo-server"
}

data "aws_ecr_repository" "aws_otel_collector" {
  name = "aws-otel-collector"
}

resource "aws_prometheus_workspace" "prometheus" {
  alias = "prometheus-${local.environment}-${local.app_name}"
}
