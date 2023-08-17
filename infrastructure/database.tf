###################################################################################
#
# DATABASE
#
###################################################################################

resource "random_password" "db_master_password" {
  length           = 32
  special          = true
  override_special = local.url_safe_password_specials
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

resource "postgresql_extension" "pg_trgm" {
  name = "pg_trgm"
}

resource "random_password" "db_user_app_password" {
  length           = 32
  special          = true
  override_special = local.url_safe_password_specials
}

resource "random_password" "db_user_migrator_password" {
  length           = 32
  special          = true
  override_special = local.url_safe_password_specials
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
    random_password.db_master_password,
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
    random_password.db_master_password,
  ]
}

resource "postgresql_default_privileges" "dbost_app" {
  role     = postgresql_role.app.name
  database = "postgres"
  schema   = "public"

  owner       = postgresql_role.migrator.name
  object_type = "table"
  privileges  = ["SELECT", "INSERT", "UPDATE", "DELETE"]
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

###################################################################################
#
# DATABASE SECRETS
#
###################################################################################

resource "aws_secretsmanager_secret" "db_master" {
  name = "dbost_db_master"
}

resource "aws_secretsmanager_secret_version" "db_master" {
  secret_id = aws_secretsmanager_secret.db_master.id
  secret_string = jsonencode({
    username          = aws_db_instance.dbost_db.username
    password          = random_password.db_master_password.result
    address           = aws_db_instance.dbost_db.address
    endpoint          = aws_db_instance.dbost_db.endpoint
    database          = "postgres"
    connection_string = "postgres://${aws_db_instance.dbost_db.username}:${random_password.db_master_password.result}@${aws_db_instance.dbost_db.endpoint}/postgres"
  })
}

resource "aws_secretsmanager_secret" "db_app" {
  name = "dbost_db_app"
}

resource "aws_secretsmanager_secret_version" "db_app" {
  secret_id = aws_secretsmanager_secret.db_app.id
  secret_string = jsonencode({
    username          = postgresql_role.app.name
    password          = postgresql_role.app.password
    address           = aws_db_instance.dbost_db.address
    endpoint          = aws_db_instance.dbost_db.endpoint
    database          = "postgres"
    connection_string = "postgres://${postgresql_role.app.name}:${postgresql_role.app.password}@${aws_db_instance.dbost_db.endpoint}/postgres"
  })
}

resource "aws_secretsmanager_secret" "db_migrator" {
  name = "dbost_db_migrator"
}

resource "aws_secretsmanager_secret_version" "db_migrator" {
  secret_id = aws_secretsmanager_secret.db_migrator.id
  secret_string = jsonencode({
    username          = postgresql_role.migrator.name
    password          = postgresql_role.migrator.password
    address           = aws_db_instance.dbost_db.address
    endpoint          = aws_db_instance.dbost_db.endpoint
    database          = "postgres"
    connection_string = "postgres://${postgresql_role.migrator.name}:${postgresql_role.migrator.password}@${aws_db_instance.dbost_db.endpoint}/postgres"
  })
}
