provider "aws" {
  region = "eu-north-1"

  default_tags {
    tags = {
      Application = "dBost"
    }
  }
}


provider "postgresql" {
  scheme           = "awspostgres"
  host             = aws_db_instance.dbost_db.address
  port             = aws_db_instance.dbost_db.port
  username         = aws_db_instance.dbost_db.username
  password         = random_password.db_master_password.result
  aws_rds_iam_auth = true
  superuser        = false
}

provider "spacelift" {}
