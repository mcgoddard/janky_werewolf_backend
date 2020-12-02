variable "lambda_name" {
  description = "The name given to the lambda function"
  type        = string
}

variable "bin_dir" {
  description = "The directory containing binaries for deployment (lambda zips)"
  type        = string
}

variable "aws_account_id" {
  description = "The AWS account ID."
  type        = string
}

variable "aws_region" {
  description = "The AWS region to create things in."
  type        = string
}

variable "iam_role_arn" {
  description = "The ARN of the IAM role for the lambda to use"
  type        = string
}

variable "api_gateway_domain" {
  description = "The domain for API gateway that was manually created."
  type        = string
}

variable "api_gateway_stage" {
  description = "The stage for API gateway."
  type        = string
}

variable "environment" {
  description = "The environment to build the lambda for."
  type        = string
}

variable "table_name" {
  description = "The name of the dynamodb table to use in the lambda."
  type        = string
}
