provider "aws" {
  region = var.region

  default_tags {
    tags = {
      Application = "dBost"
    }
  }
}


provider "postgresql" {
  database        = "postgres"
  host            = aws_db_instance.dbost_db.address
  port            = aws_db_instance.dbost_db.port
  username        = aws_db_instance.dbost_db.username
  password        = random_password.db_master_password.result
  superuser       = false
  sslmode         = "require"
  connect_timeout = 60

  scheme = "awspostgres"
  # aws_rds_iam_auth = true
}

provider "spacelift" {}
