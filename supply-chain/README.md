# Supply Chain Audits

This directory contains [cargo-vet](https://mozilla.github.io/cargo-vet/) configuration for dependency supply chain review.

## How it works

Every dependency in `Cargo.lock` must be vetted — either audited by us, covered by a trusted organization's audit, or explicitly exempted.

## Trusted audit imports

We import audits from these organizations:

| Organization | Why we trust them |
|-------------|-------------------|
| **Bytecode Alliance** | Wasmtime/Cranelift maintainers; rigorous security review process |
| **Google** | Publishes audits for crates used in Android, Fuchsia, and other projects |
| **Mozilla** | Firefox/Servo; pioneered cargo-vet; audits core Rust ecosystem crates |
| **ISRG** | Internet Security Research Group (Let's Encrypt); audits TLS/crypto crates |
| **Zcash Foundation** | Cryptocurrency project with strong security audit requirements |
| **Embark Studios** | Game development studio; heavy Rust users with published audits |
| **ARIEL-OS** | Embedded systems project; audits low-level crates |

## Exemptions

Crates listed as exempted in `config.toml` were present when cargo-vet was initialized and have not yet been formally audited. The exemption list shrinks over time as:
- Trusted organizations audit new versions of existing crates
- We manually audit crates via `cargo vet certify`

## Day-to-day workflow

- **Adding a new dependency**: `cargo vet` will fail. Run `cargo vet certify <crate>` after reviewing, or `cargo vet add-exemption <crate>` to skip.
- **Updating a dependency**: `cargo vet diff <crate> <old> <new>` shows what changed. `cargo vet certify <crate>` to approve.
- **Reducing exemptions**: `cargo vet prune` removes exemptions that are now covered by imports.
