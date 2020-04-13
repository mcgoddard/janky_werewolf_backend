module "connect_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "connect_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}

module "start_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "start_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}

module "sleep_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "sleep_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}

module "lynch_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "lynch_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}

module "seer_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "seer_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}

module "werewolf_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "werewolf_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_write.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}

module "broadcast_lambda" {
  source             = "../modules/lambda"

  lambda_name        = "broadcast_lambda"
  iam_role_arn       = aws_iam_role.iam_for_lambda_read_only.arn
  bin_dir            = var.bin_dir
  aws_region         = var.aws_region
  aws_account_id     = var.aws_account_id
  api_gateway_domain = var.api_gateway_domain
  api_gateway_stage  = var.api_gateway_stage
}
