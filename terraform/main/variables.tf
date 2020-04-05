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
