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
  version = "5.1.1"

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

###################################################################################
#
# DATABASE
#
###################################################################################

resource "random_password" "db_master_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

resource "aws_db_parameter_group" "dbost_db" {
  name   = "dbost"
  family = "postgres15"

  parameter {
    name  = "log_connections"
    value = "1"
  }
}

resource "aws_db_instance" "dbost_db" {
  allocated_storage      = 20
  db_name                = "dbost"
  engine                 = "postgres"
  engine_version         = "15.3"
  identifier             = "dbost"
  instance_class         = "db.t4g.micro"
  username               = "dbost_master"
  password               = random_password.db_master_password.result
  db_subnet_group_name   = module.vpc.database_subnet_group_name
  vpc_security_group_ids = [aws_security_group.dbost_db.id]
  parameter_group_name   = aws_db_parameter_group.dbost_db.name
  skip_final_snapshot    = true
  storage_encrypted      = true

  # TODO: remove
  publicly_accessible = true
}

resource "random_password" "db_user_app_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

resource "random_password" "db_user_migrator_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

resource "postgresql_role" "app" {
  name               = "dbost_app"
  login              = true
  password           = random_password.db_user_app_password.result
  encrypted_password = true

  depends_on = [
    module.vpc,
    aws_db_instance.dbost_db,
    aws_security_group.dbost_db,
    aws_vpc_security_group_ingress_rule.dbost_db_all_ingress,
  ]
}

resource "postgresql_role" "migrator" {
  name               = "dbost_migrator"
  login              = true
  password           = random_password.db_user_migrator_password.result
  encrypted_password = true

  depends_on = [
    module.vpc,
    aws_db_instance.dbost_db,
    aws_security_group.dbost_db,
    aws_vpc_security_group_ingress_rule.dbost_db_all_ingress,
  ]
}

# resource "postgresql_grant" "dbost_app" {
#   database    = "postgres"
#   role        = postgresql_role.app.name
#   schema      = "public"
#   object_type = "schema"
#   privileges  = ["USAGE"]
# }

# resource "postgresql_grant" "dbost_migrator" {
#   database    = "postgres"
#   role        = postgresql_role.migrator.name
#   schema      = "public"
#   object_type = "schema"
#   privileges  = ["USAGE", "CREATE"]
# }

resource "aws_secretsmanager_secret" "db_master_password" {
  name = "dbost_db_master_password"
}

resource "aws_secretsmanager_secret_version" "db_master_password" {
  secret_id     = aws_secretsmanager_secret.db_master_password.id
  secret_string = random_password.db_master_password.result
}
