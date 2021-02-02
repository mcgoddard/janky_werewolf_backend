# JankyWerewolf backend code
This is the backend code for the JankyWerewolf service, the [frontend](https://www.jankywerewolf.co.uk/) is maintained by [Ripixel](https://github.com/ripixel/janky-werewolf-client).

The service is made up of a number of AWS lambda functions, written in Rust and backed by a dynamodb table. The API is built using API Gateway and is not currently defined in this codebase (as the Terraform AWS provider does not support Websockets on API Gateway). Hopefully this will change in the future.

## Installation
Ensure `make` is installed with `sudo apt install make`, then run `make install`.

This will run through each relevant directory in the project and call the `install` make command for each of those. 

Afterwards you will have Rust and it's toolchains installed on your system for the lambda functions. You will also have python3, pip, pipenv and the dependencies required for the tests. Terraform will also be installed for IaC.

## Building
To build the various lambdas that make up the service run `make build_all` from the project root. This can take some time on a fresh install. This runs the `make build` command in each lambda subdirectory.

If you only want to build/rebuild a single lambda simply navigate to that directory and use `make build` or use `t=x_lambda make build` from the project root.

## Deploying
Make sure your system is set up with a valid AWS SDK config defining the `jankywerewolf_admin` profile.

Run `make deploy` which will generate a terraform plan for you to inspect. If you are happy with the results of the plan simply type `yes` to have that plan actioned and the code deployed.

## Test (WIP)
Run `make test` from the project root to run the unit and integration tests.

Unit tests can be run for each lambda individually by running `make x_lambda test`.

Integration tests can be separately using `make integration_test`.

## Bootstrapping a new environment
If you are spinning up JankyWerewolf in a new AWS account there are a few small changes you will have to make to get it running.
1. Modify the root `Makefile` line that reads `deploy: export AWS_PROFILE = jankywerewolf_admin` to point to your own AWS profile.
2. Modify the AWS profile and bucket name inside `terraform/terraform-state/main.tf`. The bucket name must be modified as they are globally unique.
3. Run `terraform init` and `terraform apply` inside the `terraform/terraform-state` directory. This creates the state bucket and locking table for the main terraform of the service.
4. Follow the regular install, build, deploy, test sections above to get your own JankyWerewolf up and running! 
