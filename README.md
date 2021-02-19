[![Create Release](https://github.com/thehamsterjam/better_aws_sso/workflows/Create%20Release/badge.svg)](https://github.com/thehamsterjam/better_aws_sso/actions?query=workflow%3A%22Create+Release%22) 
[![Rust](https://github.com/thehamsterjam/better_aws_sso/workflows/Rust/badge.svg)](https://github.com/thehamsterjam/better_aws_sso/actions?query=workflow%3ARust)
# (Slightly) better AWS sso login

Using AWS SSO with tools like terraform require you to go to the AWS SSO start url, click the account you want, click command line access, copy the text there and then save it to your AWS credentials file. 

This tool skips all that fuss, set up your AWS SSO like you normally would (`aws sso configure`) ([more info](#Configuring-AWS-SSO-for-AWS-CLI)). And then just run

```shell
$ ssologin -p <aws_profile> 
```

and it will save the credentials to your AWS credentials file directly.

There is an extended mode, which will collect all your credentials from a single start URL (meaning that you run `ssologin` once, and authenticate once in your browser). Use the `-a` flag. 

```shell
$ ssologin -p <aws_profile> -a
```

## Installation

### Linux machines

#### Automatic Installation and Updates
Run the below command to download and run the installer: 

```shell
$ curl -LJ https://raw.githubusercontent.com/thehamsterjam/better_aws_sso/master/install/linux_install.sh | bash
```

#### Manual Installation
The installer installs to a default location `/usr/local/bin`. To change this, instead download the installer, and pass the desired path in. This path is preserved with all updates. 

```shell
$ wget https://raw.githubusercontent.com/thehamsterjam/better_aws_sso/master/install/linux_install.sh
```

```shell
$ chmod +x ./linux_install.sh
$ ./linux_install.sh -p <desired_path>
```

## Configuring AWS SSO for AWS CLI

Configure your [AWS CLI config file](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html), which is usually located at `~/.aws/config`, with the below snippet, filling in the fields with your specific information: 

```
[my-dev-profile]
sso_start_url  = https://my-sso-portal.awsapps.com/start
sso_region     = us-east-1
sso_account_id = 123456789011
sso_role_name  = readOnly
region         = us-west-2
output         = json
```

Then run `ssologin -p my-dev-profile`. This will add credentials to your `~/.aws/credentials` file in the following form:

```
[123456789011_readOnly]
aws_access_key_id=ASIAXYZ0123456789ABC
aws_secret_access_key=xyzABC123456789defGHIjklMN/xyzABC1234567
aws_session_token=XYZ
```