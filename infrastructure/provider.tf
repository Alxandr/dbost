provider "aws" {
  region = var.region

  default_tags {
    tags = {
      Application = "dBost"
    }
  }
}

provider "spacelift" {}

provider "dnsimple" {}
