# (Slightly) better AWS sso login

Using AWS SSO with tools like terraform require you to go to the AWS SSO start url, click the account you want, click command line access, copy the text there and then save it to your AWS credentials file. 

This tool skips all that fuss, set up your AWS SSO like you normally would (`aws sso configure`). And then just run

```
ssologin -p <aws_profile> 
```

and it will save the credentials to your AWS credentials file directly.

## Installation
### Linux machines
Run the below command to download and run the installer: 

```shell
$ curl -LJ https://raw.githubusercontent.com/declan-fitzpatrick/thehamsterjam/master/install/linux_install.sh | bash
```