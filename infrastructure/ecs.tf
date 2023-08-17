##### ECS-Cluster #####
resource "aws_ecs_cluster" "cluster" {
  name = "dbost-cluster"
}

##### dBost task (service) definition #####
resource "aws_ecs_task_definition" "dbost" {
  family                   = "dbost"
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  cpu                      = 512
  memory                   = 1024
  execution_role_arn       = aws_iam_role.ecs_agent.arn
  depends_on               = []
  container_definitions = jsonencode([
    {
      name                   = "dbost-db-migrator"
      image                  = "ghcr.io/alxandr/dbost/migrator:latest"
      essential              = false
      readonlyRootFilesystem = true
      memory                 = 1024

      environment = [
        {
          name  = "DATABASE_SCHEMA"
          value = "public"
        },
        {
          name  = "RUST_LOG"
          value = "INFO"
        },
      ]
      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.db_migrator.arn}:connection_string::${aws_secretsmanager_secret_version.db_migrator.version_id}"
        },
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          awslogs-create-group  = "true"
          awslogs-group         = "dbost"
          awslogs-region        = var.region
          awslogs-stream-prefix = "migrator"
        }
      }
    },
    {
      name                   = "dbost"
      image                  = "ghcr.io/alxandr/dbost:latest"
      readonlyRootFilesystem = true
      memory                 = 1024

      portMappings = [
        {
          appProtocol   = "http2"
          containerPort = 8000
          name          = "www"
        }
      ]

      healthCheck = {
        command     = ["CMD-SHELL", "curl -f http://localhost:8000/healthz || exit 1"]
        startPeriod = 2
      }

      dependsOn = [
        {
          containerName = "dbost-db-migrator"
          condition     = "SUCCESS"
        }
      ]

      environment = [
        {
          name  = "DATABASE_SCHEMA"
          value = "public"
        },
        {
          name  = "RUST_LOG"
          value = "INFO"
        },
        {
          name  = "SECURE_COOKIES"
          value = "true"
        },
        {
          name  = "SELF_URL"
          value = "https://dbost.tv/"
        },
      ]

      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = "${aws_secretsmanager_secret.db_app.arn}:connection_string::${aws_secretsmanager_secret_version.db_app.version_id}"
        },
        {
          name      = "SESSION_KEY"
          valueFrom = "${aws_secretsmanager_secret.web.arn}:session_key::${data.aws_secretsmanager_secret_version.web.version_id}"
        },
        {
          name      = "API_KEY"
          valueFrom = "${aws_secretsmanager_secret.web.arn}:api_key::${data.aws_secretsmanager_secret_version.web.version_id}"
        },
        {
          name      = "GITHUB_CLIENT_ID"
          valueFrom = "${aws_secretsmanager_secret.web.arn}:github_client_id::${data.aws_secretsmanager_secret_version.web.version_id}"
        },
        {
          name      = "GITHUB_CLIENT_SECRET"
          valueFrom = "${aws_secretsmanager_secret.web.arn}:github_client_secret::${data.aws_secretsmanager_secret_version.web.version_id}"
        },
        {
          name      = "TVDB_API_KEY"
          valueFrom = "${aws_secretsmanager_secret.tvdb.arn}:api_key::${data.aws_secretsmanager_secret_version.tvdb.version_id}"
        },
        {
          name      = "TVDB_USER_PIN"
          valueFrom = "${aws_secretsmanager_secret.tvdb.arn}:user_pin::${data.aws_secretsmanager_secret_version.tvdb.version_id}"
        },
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          awslogs-create-group  = "true"
          awslogs-group         = "dbost"
          awslogs-region        = var.region
          awslogs-stream-prefix = "web"
        }
      }
    }
  ])
}

##### AWS ECS-SERVICE #####
resource "aws_ecs_service" "dbost" {
  cluster         = aws_ecs_cluster.cluster.id        # ECS Cluster ID
  desired_count   = 1                                 # Number of tasks running
  launch_type     = "FARGATE"                         # Cluster type [ECS OR FARGATE]
  name            = "dbost"                           # Name of service
  task_definition = aws_ecs_task_definition.dbost.arn # Attach the task to service

  network_configuration {
    subnets          = module.vpc.public_subnets
    security_groups  = [aws_security_group.public.id]
    assign_public_ip = true
  }

  # load_balancer {
  #   container_name   = "folderit-webservice"
  #   container_port   = "80"
  #   target_group_arn = aws_alb_target_group.alb_public_webservice_target_group.arn
  # }
}
