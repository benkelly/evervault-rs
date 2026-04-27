# evervault-rs

A small Rust CLI that encrypts and decrypts a single sensitive field via the
[Evervault](https://evervault.com) REST API. It's a learning project — Evervault
doesn't ship an official Rust SDK, so this demo talks to the REST endpoints
directly to show how their encryption model works end-to-end from a
language they don't natively support.

## Why this exists

I wanted to see what it takes to integrate with Evervault from Rust without
an SDK doing the heavy lifting. The result is a tiny CLI that:

1. Takes a field name and plaintext value as arguments.
2. Posts the value to `/encrypt` and prints the resulting `ev:...` token —
   this is what you'd persist in your database.
3. Posts the token to `/decrypt` to round-trip it back to plaintext.

The point isn't a production-grade integration; it's a clear, readable
illustration of the request/response shape and the three states a sensitive
value moves through.

## How Evervault encryption works (very short version)

Evervault holds the key material. Your application gets back an opaque token
in the form `ev:version:datatype:keyiv:pubkey:ciphertext:$`:

- `ev` — fixed scheme prefix.
- `version` — token version. `debug` means the value was encrypted with a
  development key and is not safe for production data.
- `datatype` — encoded hint about the original type (string, number, etc.).
- `keyiv` — the IV used for this specific encryption.
- `pubkey` — the public key reference used.
- `ciphertext` — the encrypted payload itself.
- `$` — terminator.

Because the private key never leaves Evervault, the only way to recover the
plaintext is to send the token back to their API with credentials that have
the `decrypt` permission. Tokens are safe to store in your own database;
plaintext is only reconstructed inside the API call.

## Setup

1. **Create an Evervault account** at <https://evervault.com> and create
   an App.
2. **Grab your App ID** (`app_...`) and **create an API key** (`ev:key:...`).
3. **Enable the `decrypt` permission** on that API key in the Evervault
   dashboard — without it, `/decrypt` will return 403.
4. Copy `.env.example` to `.env` and fill in the values:

   ```bash
   cp .env.example .env
   ```

   ```env
   EV_APP_ID=app_xxxxxxxxxxxxxxxx
   EV_API_KEY=ev:key:xxxxxxxxxxxxxxxx
   ```

## Running it

You need a recent stable Rust toolchain (`rustup` will sort you out).
Or, skip the local install entirely and open the repo in
[GitHub Codespaces](https://github.com/features/codespaces) — the bundled
`.devcontainer` config provisions Rust, `rust-analyzer`, and pre-fetches
dependencies on first launch.

```bash
# Round-trip a card number
cargo run -- --field card_number --value 4242424242424242

# Round-trip an email
cargo run -- --field email --value ben@example.com

# Same thing, but emit JSON instead of the human-readable summary
cargo run -- --field ssn --value 123-45-6789 --json
```

### Default output

```
Field:      card_number
Original:   4242424242424242
Encrypted:  ev:debug:Tk9D:GWgxSXezEFNw10b/:A6JZWe...:$
Decrypted:  4242424242424242
```

### `--json` output

```json
{
  "field": "card_number",
  "original": "4242424242424242",
  "encrypted": "ev:debug:Tk9D:GWgxSXezEFNw10b/:A6JZWe...:$",
  "decrypted": "4242424242424242"
}
```

## Project layout

```
src/
├── main.rs     # CLI parsing, env loading, output
├── client.rs   # Thin wrapper around the Evervault REST API
└── types.rs    # Output struct shared between text and JSON output
```

## Errors you might hit

- **`missing required environment variable EV_APP_ID`** — copy `.env.example`
  to `.env` and fill it in, or export the variable in your shell.
- **HTTP 401** — `EV_APP_ID` and `EV_API_KEY` don't match. Double-check both.
- **HTTP 403 from `/decrypt`** — the API key doesn't have the `decrypt`
  permission. Toggle it on for that key in the Evervault dashboard.
- **JSON parse failure** — the CLI prints the raw response body so you can
  see what came back.

The CLI uses `anyhow::Result` throughout and shouldn't panic in normal
operation; failures bubble up with context attached.

## A note on compliance

Decrypting sensitive data inside your own backend (which is what this demo
does) puts that backend in scope for whatever compliance regime applies
to the data — PCI DSS for card numbers, for example. Evervault's
[Relay](https://docs.evervault.com/products/relay) product exists precisely
to keep plaintext out of your infrastructure: tokens are decrypted at the
edge so your servers only ever see ciphertext.

This project is for learning, not for handling real cardholder data.
