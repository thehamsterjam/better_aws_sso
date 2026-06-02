[![Create Release](https://github.com/thehamsterjam/better_aws_sso/workflows/Create%20Release/badge.svg)](https://github.com/thehamsterjam/better_aws_sso/actions?query=workflow%3A%22Create+Release%22) 
[![Rust](https://github.com/thehamsterjam/better_aws_sso/workflows/Rust/badge.svg)](https://github.com/thehamsterjam/better_aws_sso/actions?query=workflow%3ARust)
# (Slightly) better AWS sso login

Using AWS SSO with tools like terraform require you to go to the AWS SSO start url, click the account you want, click command line access, copy the text there and then save it to your AWS credentials file.

This tool skips all that fuss, set up your AWS SSO like you normally would (`aws sso configure`) ([more info](#Configuring-AWS-SSO-for-AWS-CLI)).

### Old config format

If your `~/.aws/config` uses the original inline format (`[my-dev-profile]` with `sso_start_url = ...` directly inside it), run:

```shell
$ ssologin -p <aws_profile>
```

Extended mode authenticates once and collects creds for every profile sharing the same `sso_start_url`:

```shell
$ ssologin -p <aws_profile> -a
```

### New config format (sso-session)

If your `~/.aws/config` uses the newer format introduced by `aws configure sso` — `[profile X]` sections referencing a shared `[sso-session Y]` — pass the session name:

```shell
$ ssologin --sso-session <session_name>
```

This shells out to `aws sso login --sso-session <session_name>` for the browser auth, then writes credentials for **every** `[profile *]` whose `sso_session = <session_name>`. Filter to a single profile with `-p`:

```shell
$ ssologin --sso-session <session_name> -p <aws_profile>
```

Requires the AWS CLI v2 on `PATH`.

### Other flags

- `-s` / `--save_as_profile_name` — save credentials under `[<profile>_]` instead of the default `[<account_id>_<role_name>]`.
- `-v` / `--verbose` — print HTTP responses and cache lookups.

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

### Windows and Mac Users

Please download the [latest release](https://github.com/thehamsterjam/better_aws_sso/releases/latest) for your OS.

* Help wanted creating install scripts for Windows/Mac users

## Configuring AWS SSO for AWS CLI

Configure your [AWS CLI config file](https://docs.aws.amazon.com/cli/latest/userguide/cli-configure-sso.html), which is usually located at `~/.aws/config`. Either format is supported.

**Old (inline) format** — use `ssologin -p my-dev-profile`:

```
[my-dev-profile]
sso_start_url  = https://my-sso-portal.awsapps.com/start
sso_region     = us-east-1
sso_account_id = 123456789011
sso_role_name  = readOnly
region         = us-west-2
output         = json
```

**New (sso-session) format** — use `ssologin --sso-session my-org`:

```
[sso-session my-org]
sso_region              = us-east-1
sso_start_url           = https://my-sso-portal.awsapps.com/start
sso_registration_scopes = sso:account:access

[profile my-dev-profile]
sso_session    = my-org
sso_account_id = 123456789011
sso_role_name  = readOnly
region         = us-west-2
output         = json
```

Either way, credentials end up in `~/.aws/credentials` under a section named `<account_id>_<role_name>`:

```
[123456789011_readOnly]
aws_access_key_id=ASIAXYZ0123456789ABC
aws_secret_access_key=xyzABC123456789defGHIjklMN/xyzABC1234567
aws_session_token=XYZ
```

## Development

To build and test locally, clone the repo and run `cargo build` and `cargo run -- -p <aws_profile>`. 

## Contributing

Contributions are very welcome! Please open an issue or a PR if you have any suggestions or improvements.