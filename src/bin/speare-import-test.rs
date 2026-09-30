use std::{env, path::PathBuf, process::ExitCode};

use speare_legacy_account_import::{Acquirer, AcquisitionOptions, HttpLegacySource};

const DEFAULT_TOKEN_ENV: &str = "SPEARE_MIGRATION_BEARER_TOKEN";

#[derive(Debug, PartialEq, Eq)]
struct Args {
    base_url: String,
    endpoint_prefix: String,
    account_id: String,
    run_id: String,
    staging: PathBuf,
    page_size: usize,
    retries: usize,
    token_env: String,
}

fn usage() -> &'static str {
    "Usage: speare-import-test \
  --base-url URL \
  --account-id LOCAL_ACCOUNT_ID \
  --run-id LOCAL_RUN_ID \
  --staging FILE \
  [--endpoint-prefix /v1] \
  [--page-size 100] \
  [--retries 3] \
  [--token-env SPEARE_MIGRATION_BEARER_TOKEN]\n\n\
The bearer token is read from the environment variable selected by --token-env.\n\
The account and run IDs are local staging boundaries and are not sent to the exporter."
}

fn parse_args(values: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut values = values.into_iter();
    let mut base_url = None;
    let mut endpoint_prefix = "/v1".to_owned();
    let mut account_id = None;
    let mut run_id = None;
    let mut staging = None;
    let mut page_size = 100;
    let mut retries = 3;
    let mut token_env = DEFAULT_TOKEN_ENV.to_owned();

    while let Some(flag) = values.next() {
        if flag == "--help" || flag == "-h" {
            return Err(usage().into());
        }
        let value = values
            .next()
            .ok_or_else(|| format!("missing value for {flag}\n\n{}", usage()))?;
        match flag.as_str() {
            "--base-url" => base_url = Some(value),
            "--endpoint-prefix" => endpoint_prefix = value,
            "--account-id" => account_id = Some(value),
            "--run-id" => run_id = Some(value),
            "--staging" => staging = Some(PathBuf::from(value)),
            "--page-size" => {
                page_size = value
                    .parse()
                    .map_err(|_| "--page-size must be an integer".to_owned())?
            }
            "--retries" => {
                retries = value
                    .parse()
                    .map_err(|_| "--retries must be an integer".to_owned())?
            }
            "--token-env" => token_env = value,
            _ => return Err(format!("unknown option {flag}\n\n{}", usage())),
        }
    }

    Ok(Args {
        base_url: base_url.ok_or_else(|| format!("--base-url is required\n\n{}", usage()))?,
        endpoint_prefix,
        account_id: account_id.ok_or_else(|| format!("--account-id is required\n\n{}", usage()))?,
        run_id: run_id.ok_or_else(|| format!("--run-id is required\n\n{}", usage()))?,
        staging: staging.ok_or_else(|| format!("--staging is required\n\n{}", usage()))?,
        page_size,
        retries,
        token_env,
    })
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var(&args.token_env).map_err(|_| {
        format!(
            "authentication token environment variable {} is not set",
            args.token_env
        )
    })?;
    if token.is_empty() {
        return Err(format!(
            "authentication token environment variable {} is empty",
            args.token_env
        )
        .into());
    }

    let source =
        HttpLegacySource::with_endpoint_prefix(&args.base_url, &args.endpoint_prefix, token)?;
    let acquirer = Acquirer::with_options(
        source,
        AcquisitionOptions {
            page_size: args.page_size,
            transient_retries: args.retries,
        },
    )?;

    eprintln!(
        "Testing exporter at {}{}{}; staging to {}",
        args.base_url.trim_end_matches('/'),
        if args.endpoint_prefix.starts_with('/') {
            ""
        } else {
            "/"
        },
        args.endpoint_prefix,
        args.staging.display()
    );
    let progress = acquirer.acquire(&args.account_id, &args.run_id, &args.staging)?;
    for dataset in &progress.datasets {
        println!(
            "{:<32} {:>10}/{:<10} {}",
            dataset.name,
            dataset.acquired_rows,
            dataset.expected_rows,
            if dataset.complete {
                "complete"
            } else {
                "incomplete"
            }
        );
    }
    println!(
        "account download {} ({} datasets)",
        if progress.complete {
            "complete"
        } else {
            "incomplete"
        },
        progress.datasets.len()
    );
    Ok(())
}

fn main() -> ExitCode {
    let values = env::args().skip(1).collect::<Vec<_>>();
    if values
        .iter()
        .any(|value| value == "--help" || value == "-h")
    {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    }
    match parse_args(values).and_then(|args| run(args).map_err(|error| error.to_string())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_required_and_optional_arguments() {
        let args = parse_args(
            [
                "--base-url",
                "https://example.test",
                "--account-id",
                "account-1",
                "--run-id",
                "run-1",
                "--staging",
                "stage.sqlite",
                "--endpoint-prefix",
                "/api/migration/export/v1",
                "--page-size",
                "25",
                "--retries",
                "5",
                "--token-env",
                "TEST_TOKEN",
            ]
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(args.base_url, "https://example.test");
        assert_eq!(args.endpoint_prefix, "/api/migration/export/v1");
        assert_eq!(args.staging, PathBuf::from("stage.sqlite"));
        assert_eq!(args.page_size, 25);
        assert_eq!(args.retries, 5);
        assert_eq!(args.token_env, "TEST_TOKEN");
    }

    #[test]
    fn requires_explicit_local_boundaries() {
        let error =
            parse_args(["--base-url", "https://example.test"].map(str::to_owned)).unwrap_err();
        assert!(error.contains("--account-id is required"));
    }
}
