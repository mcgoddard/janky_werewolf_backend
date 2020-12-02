variable "aws_region" {
  description = "The AWS region to create things in."
  default     = "eu-west-2"
}

variable "aws_profile" {
  description = "The AWS profile to use with terraform."
  default     = "jankywerewolf_admin"
}

variable "aws_account_id" {
  description = "The AWS account ID."
  default     = "676626137701"
}

variable "bin_dir" {
  description = "The bin directory relative path."
  default     = "../.."
}

variable "api_gateway_domain" {
  description = "The domain for API gateway that was manually created."
  default     = "0a4nr0hbsk.execute-api.eu-west-2.amazonaws.com"
}

variable "api_gateway_stage" {
  description = "The stage for API gateway."
  default     = "dev"
}

variable "environment" {
  description = "The environment to build for."
  type        = string
}
