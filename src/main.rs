use clap::{App, Arg};
use ini::Ini;
use serde::Deserialize;
use std::process::Command;
use std::{fs, thread, time};
use ureq::Response;
use webbrowser;
extern crate dirs;
use std::time::{SystemTime, UNIX_EPOCH};

const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug)]
#[allow(non_snake_case)]
struct SsoProfile {
    sso_profile_name : String,
    sso_start_url: String,
    sso_region : String,
    sso_account_id : String,
    sso_role_name : String,
}

impl SsoProfile {
    fn new(
        sso_profile_name : String,
        sso_start_url: String,
        sso_region : String,
        sso_account_id : String,
        sso_role_name : String) -> SsoProfile {
            SsoProfile{
                sso_profile_name,
                sso_start_url,
                sso_region,
                sso_account_id,
                sso_role_name,
            }
        }
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct SsoCacheToken {
    startUrl: Option<String>,
    accessToken: Option<String>,
    expiresAt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RegisterClientResponse {
    clientId: String,
    clientSecret: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct StartDeviceAuthorizationResponse {
    deviceCode: String,
    verificationUriComplete: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct CreateTokenResponse {
    accessToken: String,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct RoleCreds {
    accessKeyId: String,
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
                        .required_unless("sso_session")
                        .help("AWS profile set up for SSO (old config format). With --sso-session, filters to a single profile under that session."))
                    .arg(Arg::with_name("sso_session")
                        .long("sso-session")
                        .takes_value(true)
                        .required_unless("profile")
                        .help("AWS SSO session name (new config format with [sso-session X]). Runs `aws sso login --sso-session X` and writes creds for every [profile *] under it."))
                    .arg(Arg::with_name("save_as_profile_name")
                        .short("s")
                        .long("save_as_profile_name")
                        .help("Save credentials under <profile>_ instead of <account_id>_<role_name>"))
                    .arg(Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("With --profile (old format): collect all profiles sharing the same sso_start_url. Ignored with --sso-session."))
                    .arg(Arg::with_name("verbose")
                        .short("v")
                        .long("verbose")
                        .help("Print verbose logging"))
                    .get_matches();

    let verbose = matches.is_present("verbose");
    let save_as_profile_name = matches.is_present("save_as_profile_name");

    let home = dirs::home_dir().unwrap().to_str().unwrap().to_owned();

    let (sso_profiles, access_token) = if let Some(session) = matches.value_of("sso_session") {
        let filter = matches.value_of("profile");
        let profiles = get_sso_profiles_new(session, &home, filter);
        run_aws_sso_login(session, verbose);
        let token = read_sso_cache_token(&profiles[0].sso_start_url, &home, verbose);
        (profiles, token)
    } else {
        let profile = matches.value_of("profile").unwrap().to_owned();
        let all = matches.is_present("all");
        let profiles = get_sso_profiles_old(profile, &home, all);
        let oidc_url = format!("https://oidc.{}.amazonaws.com", profiles[0].sso_region);
        let register_client_resp = register_client(&oidc_url, verbose);
        let device_auth_resp = device_auth(&oidc_url, &profiles[0].sso_start_url, &register_client_resp, verbose);
        let create_token_resp =
            create_token(&oidc_url, &register_client_resp, &device_auth_resp, verbose);
        (profiles, create_token_resp.accessToken)
    };

    let sso_url = format!("https://portal.sso.{}.amazonaws.com", sso_profiles[0].sso_region);

    for sso_profile in sso_profiles {

        let get_role_creds_resp = match get_role_credentials(
            &sso_url,
            &sso_profile.sso_account_id,
            &sso_profile.sso_role_name,
            &access_token,
            verbose,
        ) {
            Some(r) => r,
            None => continue,
        };

        save_sso(
            &sso_profile.sso_profile_name,
            &sso_profile.sso_account_id,
            &sso_profile.sso_role_name,
            &get_role_creds_resp,
            &home,
            save_as_profile_name,
            verbose,
        );
    }
}

fn get_sso_profiles_old(profile_name : String, home : &String, all : bool) -> Vec<SsoProfile> {

    let aws_conf = Ini::load_from_file(format!("{}{}", home, "/.aws/config")).unwrap();

    let sso_start_url = aws_conf
        .get_from(Some(profile_name.as_str()), "sso_start_url")
        .unwrap_or_else(|| panic!("Profile [{}] missing sso_start_url in ~/.aws/config (old format expected; new-format sessions need --sso-session)", profile_name))
        .to_owned();
    let sso_region = aws_conf
        .get_from(Some(profile_name.as_str()), "sso_region")
        .unwrap().to_owned();
    let sso_account_id = aws_conf
        .get_from(Some(profile_name.as_str()), "sso_account_id")
        .unwrap().to_owned();
    let sso_role_name = aws_conf
        .get_from(Some(profile_name.as_str()), "sso_role_name")
        .unwrap().to_owned();

    if !all {
        return vec![SsoProfile::new(profile_name, sso_start_url, sso_region, sso_account_id, sso_role_name)];
    }

    let mut profiles = Vec::new();
    for (section, properties) in aws_conf.iter() {
        let section_str = match section {
            Some(s) => s,
            None => continue,
        };
        // Skip new-format sections so a mixed config does not blow up here.
        if section_str.starts_with("sso-session ") || section_str.starts_with("profile ") {
            continue;
        }
        // Old-format profiles must have all four SSO keys inline.
        if !properties.contains_key("sso_start_url")
            || !properties.contains_key("sso_region")
            || !properties.contains_key("sso_account_id")
            || !properties.contains_key("sso_role_name")
        {
            continue;
        }
        if properties.get("sso_start_url").unwrap() != sso_start_url {
            continue;
        }
        profiles.push(SsoProfile::new(
            section_str.to_owned(),
            properties.get("sso_start_url").unwrap().to_owned(),
            properties.get("sso_region").unwrap().to_owned(),
            properties.get("sso_account_id").unwrap().to_owned(),
            properties.get("sso_role_name").unwrap().to_owned(),
        ));
    }
    return profiles;
}

fn get_sso_profiles_new(session: &str, home: &String, filter_profile: Option<&str>) -> Vec<SsoProfile> {

    let aws_conf = Ini::load_from_file(format!("{}{}", home, "/.aws/config")).unwrap();

    let session_section = format!("sso-session {}", session);
    let sso_start_url = aws_conf
        .get_from(Some(session_section.as_str()), "sso_start_url")
        .unwrap_or_else(|| panic!("[sso-session {}] not found or missing sso_start_url in ~/.aws/config", session))
        .to_owned();
    let sso_region = aws_conf
        .get_from(Some(session_section.as_str()), "sso_region")
        .unwrap().to_owned();

    let mut profiles = Vec::new();
    for (section, properties) in aws_conf.iter() {
        let section_str = match section {
            Some(s) => s,
            None => continue,
        };
        if !section_str.starts_with("profile ") {
            continue;
        }
        if properties.get("sso_session") != Some(session) {
            continue;
        }
        let bare_name = section_str.trim_start_matches("profile ").to_owned();
        if let Some(f) = filter_profile {
            if bare_name != f {
                continue;
            }
        }
        profiles.push(SsoProfile::new(
            bare_name,
            sso_start_url.clone(),
            sso_region.clone(),
            properties.get("sso_account_id").unwrap().to_owned(),
            properties.get("sso_role_name").unwrap().to_owned(),
        ));
    }

    if profiles.is_empty() {
        match filter_profile {
            Some(f) => panic!("No [profile {}] with sso_session = {} found in ~/.aws/config", f, session),
            None => panic!("No [profile *] with sso_session = {} found in ~/.aws/config", session),
        }
    }

    profiles
}

fn run_aws_sso_login(session: &str, verbose: bool) {
    if verbose {
        println!("Running: aws sso login --sso-session {}", session);
    }
    let status = Command::new("aws")
        .args(&["sso", "login", "--sso-session", session])
        .status()
        .unwrap();
    if !status.success() {
        panic!("aws sso login --sso-session {} failed (exit {:?})", session, status.code());
    }
}

fn read_sso_cache_token(start_url: &str, home: &str, verbose: bool) -> String {
    let cache_dir = format!("{}/.aws/sso/cache", home);
    let entries = fs::read_dir(&cache_dir)
        .unwrap_or_else(|e| panic!("Could not read SSO cache dir {}: {}", cache_dir, e));
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let token: SsoCacheToken = match serde_json::from_str(&contents) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if token.startUrl.as_deref() == Some(start_url) {
            if let Some(at) = token.accessToken {
                if verbose {
                    println!("Using cached token from {:?} (expires {:?})", path, token.expiresAt);
                }
                return at;
            }
        }
    }
    panic!("No SSO cache token found for startUrl {} in {}", start_url, cache_dir);
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
    start_url: &String,
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
    sso_account_id: &String,
    sso_role_name: &String,
    access_token: &str,
    verbose: bool,
) -> Option<GetRoleCredsResponse> {
    let resp = ureq::get(format!("{}{}", sso_url, "/federation/credentials").as_str())
        .query("account_id", sso_account_id)
        .query("role_name", sso_role_name)
        .set(
            "x-amz-sso_bearer_token",
            access_token,
        )
        .call();

    if !resp.ok() {
        let status = resp.status();
        let body = resp.into_string().unwrap_or_else(|_| "<unreadable body>".to_owned());
        eprintln!(
            "Skipping {}:{} — SSO portal returned HTTP {}: {}",
            sso_account_id, sso_role_name, status, body
        );
        return None;
    }

    let parsed = resp.into_json_deserialize::<GetRoleCredsResponse>();

    if verbose {
        println!("{:#?}", parsed);
    }

    match parsed {
        Ok(v) => {
            if verbose {
                println!("{:#?}", v);
            }
            Some(v)
        }
        Err(e) => {
            eprintln!(
                "Skipping {}:{} — could not parse credentials response: {}",
                sso_account_id, sso_role_name, e
            );
            None
        }
    }
}

fn save_sso(
    profile: &str,
    sso_account_id: &String,
    sso_role_name: &String,
    get_role_creds: &GetRoleCredsResponse,
    home_dir: &String,
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

fn _list_accounts(sso_url: String, access_token: String) -> Response {
    ureq::get(format!("{}{}", sso_url, "/assignment/accounts").as_str())
        .query("max_result", "100")
        .set(
            "x-amz-sso_bearer_token",
            format!("{}", access_token).as_str(),
        )
        .call()
}
