terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 4.16"
    }
  }

  required_version = ">= 1.2.0"
}

resource "aws_kms_key" "dbost_db_master_key" {
  description = "dBost DB master key"
}

resource "aws_db_instance" "dbost_db" {
  allocated_storage             = 20
  db_name                       = "dbost"
  engine                        = "postgres"
  engine_version                = "15.4"
  identifier                    = "dbost"
  instance_class                = "db.t4g.micro"
  username                      = "dbost-master"
  manage_master_user_password   = true
  master_user_secret_kms_key_id = aws_kms_key.dbost_db_master_key.key_id
  skip_final_snapshot           = true
}
