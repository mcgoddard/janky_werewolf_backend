# Specify the provider and access details
provider "aws" {
  region  = var.aws_region
  profile = var.aws_profile
}

provider "archive" {
}

terraform {
  backend "s3" {
    bucket         = "janky-werewolf-backend-terraform-state"
    key            = "janky-werewolf/terraform/key"
    region         = "eu-west-2"
    encrypt        = true
    dynamodb_table = "terraform-state-lock-dynamo"
  }
}
