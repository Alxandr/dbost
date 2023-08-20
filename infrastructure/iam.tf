
data "aws_iam_policy_document" "ecs_agent_trust" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ec2.amazonaws.com"]
    }
  }

  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }

  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["scheduler.amazonaws.com"]
    }
  }
}

data "aws_iam_policy_document" "ecs_agent_read_secrets" {
  statement {
    actions = [
      "secretsmanager:GetResourcePolicy",
      "secretsmanager:GetSecretValue",
      "secretsmanager:DescribeSecret",
      "secretsmanager:ListSecretVersionIds",
    ]
    resources = [
      aws_secretsmanager_secret.web.arn,
      aws_secretsmanager_secret.tvdb.arn,
      aws_secretsmanager_secret.db_app.arn,
      aws_secretsmanager_secret.db_migrator.arn,
    ]
    effect = "Allow"
  }

  statement {
    actions = [
      "secretsmanager:ListSecrets",
    ]
    resources = ["*"]
    effect    = "Allow"
  }
}

data "aws_iam_policy_document" "esc_agent_write_dlq_message" {
  statement {
    actions = [
      "sqs:SendMessage",
    ]
    resources = [
      aws_sqs_queue.dbost_db_schedule_dlq.arn,
    ]
    effect = "Allow"
  }
}

data "aws_iam_policy_document" "esc_agent_start_task" {
  statement {
    actions = [
      "ecs:RunTask",
    ]
    resources = [
      aws_ecs_cluster.cluster.arn,
      data.aws_ecs_task_definition.dbost_db_cleaner.arn_without_revision,
    ]
    effect = "Allow"
  }

  statement {
    actions = [
      "iam:PassRole"
    ]
    resources = [
      aws_iam_role.ecs_agent.arn
    ]
    effect = "Allow"
  }
}

resource "aws_iam_role" "ecs_agent" {
  name               = "ecs-agent"
  assume_role_policy = data.aws_iam_policy_document.ecs_agent_trust.json
}


resource "aws_iam_role_policy_attachment" "ecs_agent" {
  role       = aws_iam_role.ecs_agent.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEC2ContainerServiceforEC2Role"
}

resource "aws_iam_role_policy_attachment" "Cloudwatch_FullAccess" {
  role       = aws_iam_role.ecs_agent.name
  policy_arn = "arn:aws:iam::aws:policy/CloudWatchLogsFullAccess"
}

resource "aws_iam_instance_profile" "ecs_agent" {
  name = "ecs-agent"
  role = aws_iam_role.ecs_agent.name
}

resource "aws_iam_policy" "ecs_agent_read_secrets" {
  name        = "read-dbost-secrets"
  description = "Read dBost secrets"
  policy      = data.aws_iam_policy_document.ecs_agent_read_secrets.json
}

resource "aws_iam_policy" "esc_agent_write_dlq_message" {
  name        = "write-dbost-dlq-messages"
  description = "Write dBost DLQ messages"
  policy      = data.aws_iam_policy_document.esc_agent_write_dlq_message.json
}

resource "aws_iam_policy" "esc_agent_start_task" {
  name        = "start-dbost-tasks"
  description = "Start dBost tasks"
  policy      = data.aws_iam_policy_document.esc_agent_start_task.json
}

resource "aws_iam_role_policy_attachment" "ecs_agent_read_secrets" {
  role       = aws_iam_role.ecs_agent.name
  policy_arn = aws_iam_policy.ecs_agent_read_secrets.arn
}

resource "aws_iam_role_policy_attachment" "esc_agent_write_dlq_message" {
  role       = aws_iam_role.ecs_agent.name
  policy_arn = aws_iam_policy.esc_agent_write_dlq_message.arn
}

resource "aws_iam_role_policy_attachment" "esc_agent_start_task" {
  role       = aws_iam_role.ecs_agent.name
  policy_arn = aws_iam_policy.esc_agent_start_task.arn
}
