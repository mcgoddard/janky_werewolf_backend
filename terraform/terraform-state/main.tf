provider "aws" {
  region  = "eu-west-2"
  profile = "jankywerewolf_admin"
}

resource "aws_s3_bucket" "terraform_state" {
  bucket = "janky-werewolf-backend-terraform-state"
  versioning {
    enabled = true
  }
  lifecycle {
    prevent_destroy = true
  }
}

resource "aws_dynamodb_table" "janky-werewolf-terraform-state-lock" {
  name = "terraform-state-lock-dynamo"
  hash_key = "LockID"
  billing_mode = "PAY_PER_REQUEST"

  attribute {
    name = "LockID"
    type = "S"
  }

  tags = {
    Name = "DynamoDB Terraform State Lock Table"
  }
}
