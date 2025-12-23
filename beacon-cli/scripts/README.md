# Beacon Code npm releases

Use the helpers in this directory to generate npm tarballs for a Beacon Code CLI
release. For example, invoke `build_npm_package.py` after
`beacon-cli/scripts/install_native_deps.py` has hydrated `vendor/` for the desired
packages; point `--vendor-src` at the populated `vendor/` tree. The build script
also writes `checksums.json` from the vendor binaries so npm installs can verify
downloaded artifacts.
