terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.12"
    }

    postgresql = {
      source  = "cyrilgdn/postgresql"
      version = "~> 1.20"
    }

    random = {
      source  = "hashicorp/random"
      version = "~> 3.5"
    }

    spacelift = {
      source  = "spacelift-io/spacelift"
      version = "~> 1.1"
    }
  }

  required_version = ">= 1.2.0"
}
