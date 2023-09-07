data "aws_availability_zones" "available" {
  # Only Availability Zones (no Local Zones):
  filter {
    name   = "opt-in-status"
    values = ["opt-in-not-required"]
  }
}

locals {
  azs      = slice(data.aws_availability_zones.available.names, 0, 3)
  vpc_cidr = "10.0.0.0/16"
}

###################################################################################
#
# NETWORKING
#
###################################################################################

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.1.2"

  name = "dBost"
  cidr = local.vpc_cidr
  azs  = local.azs

  private_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k)]
  public_subnets   = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 4)]
  database_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 8)]
  intra_subnets    = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 20)]
  # elasticache_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 12)]
  # redshift_subnets    = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 16)]
  # public_subnets       = ["10.0.4.0/24", "10.0.5.0/24", "10.0.6.0/24"]
  # enable_dns_hostnames = true
  # enable_dns_support   = true

  # TODO: Figure this shit out
  create_database_subnet_route_table     = true
  create_database_internet_gateway_route = true

  create_database_subnet_group  = true
  manage_default_network_acl    = false
  manage_default_route_table    = false
  manage_default_security_group = false

  enable_dns_hostnames = true
  enable_dns_support   = true

  enable_nat_gateway = true
  single_nat_gateway = true
}

#################
# DB networking
#################

resource "aws_security_group" "dbost_db" {
  name_prefix = "dbost-rds"
  description = "Allow PostgreSQL inbound traffic"
  vpc_id      = module.vpc.vpc_id
}

resource "aws_vpc_security_group_ingress_rule" "dbost_db_vpc_ingress" {
  description       = "TLS from VPC"
  from_port         = 5432
  to_port           = 5432
  ip_protocol       = "tcp"
  cidr_ipv4         = module.vpc.vpc_cidr_block
  security_group_id = aws_security_group.dbost_db.id
}

# figure out how to limit this to spacelift
resource "aws_vpc_security_group_ingress_rule" "dbost_db_all_ingress" {
  description       = "TLS from anywhere"
  from_port         = 5432
  to_port           = 5432
  ip_protocol       = "tcp"
  cidr_ipv4         = "0.0.0.0/0"
  security_group_id = aws_security_group.dbost_db.id
}

#################
# public networking
#################

resource "aws_security_group" "public" {
  name        = "Allow public HTTP/HTTPS ALB"
  description = "Public internet access"
  vpc_id      = module.vpc.vpc_id
}

resource "aws_security_group_rule" "public_out" {
  type        = "egress"
  from_port   = 0
  to_port     = 0
  protocol    = "-1"
  cidr_blocks = ["0.0.0.0/0"]

  security_group_id = aws_security_group.public.id
}

resource "aws_security_group_rule" "public_in_http" {
  type              = "ingress"
  from_port         = 80
  to_port           = 80
  protocol          = "tcp"
  cidr_blocks       = ["0.0.0.0/0"]
  security_group_id = aws_security_group.public.id
}

resource "aws_security_group_rule" "public_in_https" {
  type              = "ingress"
  from_port         = 443
  to_port           = 443
  protocol          = "tcp"
  cidr_blocks       = ["0.0.0.0/0"]
  security_group_id = aws_security_group.public.id
}

#################
# ECS networking
#################

resource "aws_security_group" "ec2_ecs_instance" {
  name        = "Allow internal VPC traffic"
  description = "Allow internal VPC traffic"
  vpc_id      = module.vpc.vpc_id

}

resource "aws_security_group_rule" "allow_internal_VPC_traffic" {
  type              = "ingress"
  from_port         = 0
  to_port           = 0
  protocol          = "-1"
  cidr_blocks       = [local.vpc_cidr]
  security_group_id = aws_security_group.ec2_ecs_instance.id
}

resource "aws_security_group_rule" "public_out_ec2" {
  type        = "egress"
  from_port   = 0
  to_port     = 0
  protocol    = "-1"
  cidr_blocks = ["0.0.0.0/0"]

  security_group_id = aws_security_group.ec2_ecs_instance.id
}
