module "connect_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "connect_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
}

module "broadcast_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "broadcast_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_only.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
}
