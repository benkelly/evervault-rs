mod client;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use client::EvervaultClient;
use types::RoundTrip;

#[derive(Parser, Debug)]
#[command(
    name = "evervault-rs",
    about = "Encrypt and decrypt a single field via the Evervault REST API",
    version
)]
struct Args {
    /// Field name (used as the JSON key in the request body)
    #[arg(long)]
    field: String,

    /// Plaintext value to encrypt
    #[arg(long)]
    value: String,

    /// Emit machine-readable JSON instead of a human-readable summary
    #[arg(long)]
    json: bool,
}

fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    let args = Args::parse();

    let app_id = require_env("EV_APP_ID")?;
    let api_key = require_env("EV_API_KEY")?;

    let client = EvervaultClient::new(&app_id, &api_key)?;

    let encrypted = client
        .encrypt(&args.field, &args.value)
        .context("encryption failed")?;
    let decrypted = client
        .decrypt(&args.field, &encrypted)
        .context("decryption failed")?;

    let result = RoundTrip {
        field: args.field,
        original: args.value,
        encrypted,
        decrypted,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Field:      {}", result.field);
        println!("Original:   {}", result.original);
        println!("Encrypted:  {}", result.encrypted);
        println!("Decrypted:  {}", result.decrypted);
    }

    Ok(())
}

fn require_env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| {
        anyhow::anyhow!(
            "missing required environment variable `{key}`. \
             Set it in your shell or add it to a .env file. \
             See .env.example for the expected format."
        )
    })
}
