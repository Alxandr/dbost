data "aws_availability_zones" "available" {
  # Only Availability Zones (no Local Zones):
  filter {
    name   = "opt-in-status"
    values = ["opt-in-not-required"]
  }
}

locals {
  azs = slice(data.aws_availability_zones.available.names, 0, 3)
}

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.1.1"

  name             = "dBost"
  cidr             = "10.0.0.0/16"
  azs              = local.azs
  private_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k)]
  public_subnets   = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 4)]
  database_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 8)]
  intra_subnets    = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 20)]
  # elasticache_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 12)]
  # redshift_subnets    = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 16)]
  # public_subnets       = ["10.0.4.0/24", "10.0.5.0/24", "10.0.6.0/24"]
  # enable_dns_hostnames = true
  # enable_dns_support   = true

  create_database_subnet_group  = false
  manage_default_network_acl    = false
  manage_default_route_table    = false
  manage_default_security_group = false

  enable_dns_hostnames = true
  enable_dns_support   = true

  enable_nat_gateway = true
  single_nat_gateway = true
}

resource "random_password" "db_master_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

resource "aws_db_instance" "dbost_db" {
  allocated_storage   = 20
  db_name             = "dbost"
  engine              = "postgres"
  engine_version      = "15.3"
  identifier          = "dbost"
  instance_class      = "db.t4g.micro"
  username            = "dbost_master"
  password            = random_password.db_master_password.result
  skip_final_snapshot = true
  storage_encrypted   = true
}

resource "random_password" "db_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

# resource "postgresql_role" "db_role" {
#   name               = "dbost"
#   login              = true
#   password           = random_password.db_password.result
#   encrypted_password = true
# }

# provider "postgresql" {
#   scheme    = "awspostgres"
#   host      = "db.domain.name"
#   port      = "5432"
#   username  = "db_username"
#   password  = "db_password"
#   superuser = false
# }
