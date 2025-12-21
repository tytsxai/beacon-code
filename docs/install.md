## Install & build

### System requirements

| Requirement                 | Details                                                         |
| --------------------------- | --------------------------------------------------------------- |
| Operating systems           | macOS 12+, Ubuntu 20.04+/Debian 10+, or Windows 11 **via WSL2** |
| Git (optional, recommended) | 2.23+ for built-in PR helpers                                   |
| RAM                         | 4-GB minimum (8-GB recommended)                                 |

### DotSlash

The GitHub Release also contains a [DotSlash](https://dotslash-cli.com/) file for the Beacon CLI named `code`. Using a DotSlash file makes it possible to make a lightweight commit to source control to ensure all contributors use the same version of an executable, regardless of what platform they use for development.

### Build from source

```bash
# Clone the repository and navigate to the root of the Cargo workspace.
git clone https://github.com/tytsxai/beacon-code.git
cd beacon-code

# Bootstrap local prerequisites (Rust components + common Cargo tools).
./scripts/bootstrap.sh

# Build Beacon (Rust workspace).
make build-dev

# Launch the TUI with a sample prompt.
just tui -- "explain this codebase to me"

# After making changes, use the workspace justfile helpers:
just fmt
just fix -p <crate-you-touched>

# Run the relevant tests (project-specific is fastest), for example:
cargo test -p code-tui
# If you have cargo-nextest installed, `just test` runs the full suite:
just test
# Otherwise, fall back to:
cargo test --all-features
```
