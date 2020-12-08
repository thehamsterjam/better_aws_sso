// CARGO BUILD SCRIPT

use std::{
    error::Error, fs::File, io::{BufWriter, Write}, path::Path, io::Read
};
use ureq;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = "./src";
    let dest_path = Path::new(&out_dir).join("version");
    let mut f = BufWriter::new(File::create(&dest_path)?);
    let version = get_latest_version().replace("\"", "");
    write!(f, "{}", version)?;
    Ok(())
}

fn get_latest_version() -> String {
    let api_url = "https://api.github.com/repos/thehamsterjam/better_aws_sso/releases/latest";
    let resp = ureq::get(api_url)
        .call()
        .into_json()
        .unwrap();
    resp["tag_name"].to_string()
}