##### ECS-Cluster #####
resource "aws_ecs_cluster" "cluster" {
  name = "dbost-cluster"
}

##### DB migrator task #####
resource "aws_ecs_task_definition" "dbost-db-migrator" {
  family                   = "dbost-db-migrator"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = 0.5
  memory                   = 1024
  container_definitions = jsonencode([{
    name   = "dbost-db-migrator"
    image  = "ghcr.io/alxandr/dbost/migrator:latest"
    cpu    = 1
    memory = 1024
    environment = [
      {
        name  = "DATABASE_URL"
        value = "postgres://${postgresql_role.migrator.name}:${postgresql_role.migrator.password}@${aws_db_instance.dbost_db.endpoint}/postgres"
      },
      {
        name  = "DATABASE_SCHEMA"
        value = "public"
      }
    ]
  }])
}
