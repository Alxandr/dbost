resource "aws_secretsmanager_secret" "web" {
  name = "dbost_web"
}

data "aws_secretsmanager_secret_version" "web" {
  secret_id = aws_secretsmanager_secret.web.id
}

resource "aws_secretsmanager_secret" "tvdb" {
  name = "dbost_tvdb"
}

data "aws_secretsmanager_secret_version" "tvdb" {
  secret_id = aws_secretsmanager_secret.tvdb.id
}
