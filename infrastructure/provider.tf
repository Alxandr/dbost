terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.16"
    }

    postgresql = {
      source  = "cyrilgdn/postgresql"
      version = "1.20.0"
    }

    random = {
      source  = "hashicorp/random"
      version = "3.5.1"
    }
  }

  required_version = ">= 1.2.0"
}

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
  password         = aws_db_instance.dbost_db.password
  aws_rds_iam_auth = true
  superuser        = false
}
