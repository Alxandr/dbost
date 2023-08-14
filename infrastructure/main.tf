resource "aws_kms_key" "dbost_db_master_key" {
  description = "dBost DB master key"
}

resource "aws_db_instance" "dbost_db" {
  allocated_storage             = 20
  db_name                       = "dbost"
  engine                        = "postgres"
  engine_version                = "15.3"
  identifier                    = "dbost"
  instance_class                = "db.t4g.micro"
  username                      = "dbost_master"
  manage_master_user_password   = true
  master_user_secret_kms_key_id = aws_kms_key.dbost_db_master_key.key_id
  skip_final_snapshot           = true
  storage_encrypted             = true
}

resource "random_password" "db_password" {
  length           = 32
  special          = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

resource "postgresql_role" "db_role" {
  name               = "dbost"
  login              = true
  password           = random_password.db_password.result
  encrypted_password = true
}

# provider "postgresql" {
#   scheme    = "awspostgres"
#   host      = "db.domain.name"
#   port      = "5432"
#   username  = "db_username"
#   password  = "db_password"
#   superuser = false
# }
