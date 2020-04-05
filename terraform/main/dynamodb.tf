

resource "aws_dynamodb_table" "janky-werewolf-table" {
  name           = "janky-werewolf-table"
  billing_mode   = "PAY_PER_REQUEST"
  hash_key       = "pk"

  attribute {
    name = "pk"
    type = "S"
  }
}
