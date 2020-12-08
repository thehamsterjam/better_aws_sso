use clap::{App, Arg};
use ini::Ini;
use serde::Deserialize;
use std::{thread, time};
use ureq::Response;
use webbrowser;
extern crate dirs;
use std::time::{SystemTime, UNIX_EPOCH};

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RegisterClientResponse {
    clientId: String,
    clientIdIssuedAt: i32,
    clientSecret: String,
    clientSecretExpiresAt: i32,
    authorizationEndpoint: Option<String>,
    tokenEndpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct StartDeviceAuthorizationResponse {
    deviceCode: String,
    expiresIn: i32,
    interval: i32,
    userCode: String,
    verificationUri: Option<String>,
    verificationUriComplete: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct CreateTokenResponse {
    accessToken: String,
    expiresIn: i32,
    idToken: Option<String>,
    refreshToken: Option<String>,
    tokenType: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RoleCreds {
    accessKeyId: String,
    expiration: i64,
    secretAccessKey: String,
    sessionToken: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct GetRoleCredsResponse {
    roleCredentials: RoleCreds,
}

fn main() {
    static VERSION: &'static str = include_str!(concat!("", "version"));
    let matches = App::new("AWS SSO, but better")
                    .version(VERSION)
                    .author("Damien Maier")
                    .author("Saves your SSO login credentials into the credentials file, so it can be used with things like terraform")
                    .arg(Arg::with_name("profile")
                        .short("p")
                        .long("profile")
                        .takes_value(true)
                        .required(true)
                        .help("AWS profile set up for SSO"))
                    .arg(Arg::with_name("save_as_profile_name")
                        .short("s")
                        .long("save_as_profile_name")
                        .help("Whether to save the credentials under the profile name with an _ at the end or under <account_id>_<role_name>"))
                    .arg(Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .help("Print verbose logging"))
                    .get_matches();

    let profile = matches.value_of("profile").unwrap();
    let verbose = matches.is_present("verbose");
    let save_as_profile_name = matches.is_present("save_as_profile_name");

    let home = dirs::home_dir().unwrap().to_str().unwrap().to_owned();

    let aws_conf = Ini::load_from_file(format!("{}{}", home, "/.aws/config")).unwrap();

    let sso_start_url = aws_conf
        .get_from(Some(format!("profile {}", profile)), "sso_start_url")
        .unwrap();
    let sso_region = aws_conf
        .get_from(Some(format!("profile {}", profile)), "sso_region")
        .unwrap();
    let sso_account_id = aws_conf
        .get_from(Some(format!("profile {}", profile)), "sso_account_id")
        .unwrap();
    let sso_role_name = aws_conf
        .get_from(Some(format!("profile {}", profile)), "sso_role_name")
        .unwrap();

    let oidc_url = format!("https://oidc.{}.amazonaws.com", sso_region);
    let sso_url = format!("https://portal.sso.{}.amazonaws.com", sso_region);

    let register_client_resp = register_client(&oidc_url, verbose);

    let device_auth_resp = device_auth(&oidc_url, sso_start_url, &register_client_resp, verbose);

    let create_token_resp =
        create_token(&oidc_url, &register_client_resp, &device_auth_resp, verbose);

    let get_role_creds_resp = get_role_credentials(
        &sso_url,
        sso_account_id,
        sso_role_name,
        &create_token_resp,
        verbose,
    );

    save_sso(
        profile,
        sso_account_id,
        sso_role_name,
        &get_role_creds_resp,
        home,
        save_as_profile_name,
        verbose,
    );
}

fn register_client(oidc_url: &String, verbose: bool) -> RegisterClientResponse {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let register_resp = ureq::post(format!("{}{}", oidc_url, "/client/register").as_str())
        .set("Content-type", "application/json")
        .set("Action", "RegisterClient")
        .set("Version", "2019-06-10")
        .send_json(serde_json::json!({
            "clientName" : format!("rustSSO-{}", since_the_epoch).as_str(),
            "clientType" : "public"
        }))
        .into_json_deserialize::<RegisterClientResponse>();

    if verbose {
        println!("{:#?}", register_resp);
    }

    let register_resp_un = register_resp.unwrap();

    if verbose {
        println!("{:#?}", register_resp_un);
    }

    register_resp_un
}

fn device_auth(
    oidc_url: &String,
    start_url: &str,
    register_resp: &RegisterClientResponse,
    verbose: bool,
) -> StartDeviceAuthorizationResponse {
    let device_auth_resp = ureq::post(format!("{}{}", oidc_url, "/device_authorization").as_str())
        .set("Content-type", "application/json")
        .set("Action", "StartDeviceAuthorization")
        .set("Version", "2019-06-10")
        .send_json(serde_json::json!( {
            "clientId" : register_resp.clientId,
            "clientSecret" : register_resp.clientSecret,
            "startUrl" : start_url
        }))
        .into_json_deserialize::<StartDeviceAuthorizationResponse>();

    if verbose {
        println!("{:#?}", device_auth_resp);
    }
    let device_auth_resp_un = device_auth_resp.unwrap();
    if verbose {
        println!("{:#?}", device_auth_resp_un);
    }

    device_auth_resp_un
}

fn create_token(
    oidc_url: &String,
    register_resp: &RegisterClientResponse,
    device_auth_resp: &StartDeviceAuthorizationResponse,
    verbose: bool,
) -> CreateTokenResponse {
    if webbrowser::open(device_auth_resp.verificationUriComplete.as_str()).is_err() {
        println!(
            "Go to {}",
            device_auth_resp.verificationUriComplete.as_str()
        );
    }
    let sec = time::Duration::from_secs(1);
    let create_tok_resp_un = loop {
        thread::sleep(sec);
        let create_tok_resp = ureq::post(format!("{}{}", oidc_url, "/token").as_str())
            .set("Content-type", "application/json")
            .set("Action", "CreateToken")
            .set("Version", "2019-06-10")
            .send_json(serde_json::json!({
                "clientId": register_resp.clientId,
                "clientSecret": register_resp.clientSecret,
                "deviceCode": device_auth_resp.deviceCode,
                "grantType": GRANT_TYPE,
            }));

        if verbose {
            println!("{:#?}", create_tok_resp);
        }

        if create_tok_resp.ok() {
            break create_tok_resp
                .into_json_deserialize::<CreateTokenResponse>()
                .unwrap();
        } else {
            if verbose {
                println!("{:#?}", create_tok_resp.into_json());
            }
        }
    };
    if verbose {
        println!("{:#?}", create_tok_resp_un);
    }

    create_tok_resp_un
}

fn get_role_credentials(
    sso_url: &String,
    sso_account_id: &str,
    sso_role_name: &str,
    create_token_resp: &CreateTokenResponse,
    verbose: bool,
) -> GetRoleCredsResponse {
    let get_role_creds = ureq::get(format!("{}{}", sso_url, "/federation/credentials").as_str())
        .query("account_id", sso_account_id)
        .query("role_name", sso_role_name)
        .set(
            "x-amz-sso_bearer_token",
            format!("{}", create_token_resp.accessToken).as_str(),
        )
        .call()
        .into_json_deserialize::<GetRoleCredsResponse>();

    if verbose {
        println!("{:#?}", get_role_creds);
    }

    let get_role_creds_un = get_role_creds.unwrap();

    if verbose {
        println!("{:#?}", get_role_creds_un);
    }

    get_role_creds_un
}

fn save_sso(
    profile: &str,
    sso_account_id: &str,
    sso_role_name: &str,
    get_role_creds: &GetRoleCredsResponse,
    home_dir: String,
    save_as_profile_name: bool,
    verbose: bool,
) {
    let section_name = if save_as_profile_name {
        format!("{}_", profile)
    } else {
        format!("{}_{}", sso_account_id, sso_role_name)
    };

    let mut aws_creds =
        Ini::load_from_file(format!("{}{}", home_dir, "/.aws/credentials")).unwrap();

    if verbose {
        println!("Section name : {}", section_name);
    }

    aws_creds
        .with_section(Some(section_name))
        .set(
            "aws_access_key_id",
            get_role_creds.roleCredentials.accessKeyId.to_owned(),
        )
        .set(
            "aws_secret_access_key",
            get_role_creds.roleCredentials.secretAccessKey.to_owned(),
        )
        .set(
            "aws_session_token",
            get_role_creds.roleCredentials.sessionToken.to_owned(),
        );

    aws_creds
        .write_to_file(format!("{}{}", home_dir, "/.aws/credentials"))
        .unwrap();
}

fn list_accounts(sso_url: String, accessToken: String) -> Response {
    ureq::get(format!("{}{}", sso_url, "/assignment/accounts").as_str())
        .query("max_result", "100")
        .set(
            "x-amz-sso_bearer_token",
            format!("{}", accessToken).as_str(),
        )
        .call()
}
