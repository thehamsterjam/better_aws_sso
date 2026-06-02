# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Rust CLI (`ssologin`) that performs AWS SSO login and writes the resulting temporary credentials directly into `~/.aws/credentials`, bypassing the manual copy-paste flow from the AWS SSO portal. Supports both legacy (`[profile]` with inline `sso_start_url`) and new (`[profile]` + `[sso-session X]`) AWS config formats via two explicit CLI routes.

## Commands

- Build: `cargo build` (debug), `cargo build --release` (used in CI).
- Static Linux build (matches release artifact): `cargo build --release --target=x86_64-unknown-linux-musl` inside `stevenleadbeater/rust-musl-builder` container.
- Run (old format): `cargo run -- -p <aws_profile> [-a] [-s] [-v]`.
- Run (new format): `cargo run -- --sso-session <session> [-p <profile>] [-s] [-v]`.
- No test suite; `cargo test` is a no-op.
- Lint/format: standard `cargo fmt`, `cargo clippy` (not enforced in CI).

## CLI routes

`--profile` and `--sso-session` are mutually requiring (one of the two must be set) — clap enforces with `required_unless`.

| Invocation | Format | Behavior |
|---|---|---|
| `-p X` | old | reads `[X]` with inline `sso_start_url`/`sso_region`. Runs internal OIDC device-grant flow. |
| `-p X -a` | old | also picks up every other `[Y]` whose `sso_start_url` matches X's. |
| `--sso-session Y` | new | reads `[sso-session Y]` for start URL/region. Shells `aws sso login --sso-session Y`. Writes creds for every `[profile *]` whose `sso_session = Y`. |
| `--sso-session Y -p X` | new | as above but filters to single profile `[profile X]`. |

`-a` is ignored in the `--sso-session` route (multi-profile is the default there).

## Build script side-effect

`build.rs` runs every build and performs a network call to `https://api.github.com/repos/thehamsterjam/better_aws_sso/releases/latest`, then writes the tag name to `src/version`. `src/main.rs` consumes it via `include_str!("version")` to set the clap `--version` string. Consequences:
- Offline builds fail unless `src/version` already exists.
- `src/version` is generated; do not hand-edit. (Not in `.gitignore` — check before committing.)
- Forks pointing at a different repo must update the URL in `build.rs`.

## Runtime architecture (`src/main.rs`, single file)

`main()` branches on `--sso-session` vs `--profile`:

**Old-format flow** (`get_sso_profiles_old` + internal OIDC):
1. `get_sso_profiles_old` parses `~/.aws/config` via `rust-ini`. With `-a`, iterates every section but skips ones starting with `sso-session ` or `profile ` (would otherwise blow up on a mixed-format config) and requires all four `sso_*` keys inline.
2. `register_client` → POST `https://oidc.<region>.amazonaws.com/client/register` to get an OIDC client id/secret.
3. `device_auth` → POST `/device_authorization` to get `verificationUriComplete` + `deviceCode`.
4. `create_token` opens the verification URL in the user's browser (`webbrowser` crate), then polls `/token` once per second until the user completes auth (loop has no timeout — exits only on success).

**New-format flow** (`get_sso_profiles_new` + `aws` shell-out):
1. `get_sso_profiles_new` reads `[sso-session <name>]` for start URL/region and collects every `[profile *]` whose `sso_session` key equals `<name>`. Optional single-profile filter via `-p`.
2. `run_aws_sso_login` shells out: `aws sso login --sso-session <name>`. AWS CLI handles the browser flow and writes a token to `~/.aws/sso/cache/<sha1>.json`. Panics if `aws` isn't on PATH or exits non-zero.
3. `read_sso_cache_token` scans `~/.aws/sso/cache/*.json`, returns the `accessToken` from the file whose `startUrl` matches. Match is exact-string (no normalization of trailing slashes).

**Shared tail** — for each profile: `get_role_credentials` → GET `https://portal.sso.<region>.amazonaws.com/federation/credentials` with `x-amz-sso_bearer_token`, then `save_sso` writes `aws_access_key_id` / `aws_secret_access_key` / `aws_session_token` into `~/.aws/credentials`.

Per-profile failures (non-2xx from the portal, e.g. role not assigned to that account) are logged and skipped — the loop continues. Other failures still panic via `.unwrap()`.

Credential section name in `~/.aws/credentials`:
- Default: `<sso_account_id>_<sso_role_name>`.
- With `-s` / `--save_as_profile_name`: `<profile>_` (trailing underscore intentional, matches existing behavior).

HTTP is `ureq` 1.5 (blocking, sync). JSON via `serde` derive on the response structs at the top of `main.rs`; field names are camelCase to match the AWS API (hence the `#[allow(non_snake_case)]`). Response structs hold only fields actually read by the code — serde drops unknown JSON keys by default.

## Editing notes

- All logic lives in `src/main.rs`. No module split.
- Errors mostly use `.unwrap()` — failures panic. The one intentional exception is `get_role_credentials`, which returns `Option<GetRoleCredsResponse>` so a single bad profile (e.g. role not assigned) doesn't abort the whole `-a` / `--sso-session` run. Preserve the panic style elsewhere unless a change requires otherwise.
- The token-polling loop in `create_token` (old-format flow only) does not honor `expiresIn` or `interval` from the device-auth response; it hardcodes 1s. Mention this if touching that function.
- `_list_accounts` is dead code retained for reference (leading underscore suppresses warning).
- `old_format.config` and `new_format.config` at the repo root are test fixtures the user swaps into `~/.aws/config` to exercise each route — not loaded automatically.

## Release / install

- `.github/workflows/release.yml` cuts releases; `rust.yml` builds Linux (musl static), Windows, and macOS artifacts on push/PR to `master`.
- `install/linux_install.sh` downloads the latest release artifact to `/usr/local/bin` by default (`-p <path>` overrides). The installer is what the README's `curl | bash` invocation runs.
