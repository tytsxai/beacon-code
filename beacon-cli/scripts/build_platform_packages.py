#!/usr/bin/env python3
"""Build npm tarballs for the platform-specific @tytsxai/beacon-code-* packages.

These packages are consumed via optionalDependencies by @tytsxai/beacon-code.
They contain only the native `code-<targetTriple>` binary under `bin/`.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
BEACON_CLI_ROOT = SCRIPT_DIR.parent
REPO_ROOT = BEACON_CLI_ROOT.parent


@dataclass(frozen=True)
class PlatformPackage:
    package_name: str
    target: str
    os: str
    cpu: str

    def binary_name(self) -> str:
        if "windows" in self.target:
            return f"code-{self.target}.exe"
        return f"code-{self.target}"


PLATFORM_PACKAGES: list[PlatformPackage] = [
    PlatformPackage(
        package_name="@tytsxai/beacon-code-darwin-arm64",
        target="aarch64-apple-darwin",
        os="darwin",
        cpu="arm64",
    ),
    PlatformPackage(
        package_name="@tytsxai/beacon-code-darwin-x64",
        target="x86_64-apple-darwin",
        os="darwin",
        cpu="x64",
    ),
    PlatformPackage(
        package_name="@tytsxai/beacon-code-linux-x64-musl",
        target="x86_64-unknown-linux-musl",
        os="linux",
        cpu="x64",
    ),
    PlatformPackage(
        package_name="@tytsxai/beacon-code-linux-arm64-musl",
        target="aarch64-unknown-linux-musl",
        os="linux",
        cpu="arm64",
    ),
    PlatformPackage(
        package_name="@tytsxai/beacon-code-win32-x64",
        target="x86_64-pc-windows-msvc",
        os="win32",
        cpu="x64",
    ),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--release-version",
        required=True,
        help="Version to write to package.json (e.g. 0.6.12).",
    )
    parser.add_argument(
        "--vendor-src",
        type=Path,
        required=True,
        help=(
            "Vendor directory containing built binaries in the layout produced by "
            "beacon-cli/scripts/install_native_deps.py (i.e. <vendor>/<target>/code/code)."
        ),
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Directory where npm tarballs should be written (default: dist/npm).",
    )
    parser.add_argument(
        "--keep-staging-dirs",
        action="store_true",
        help="Retain temporary staging directories instead of deleting them.",
    )
    return parser.parse_args()


def run(cmd: list[str], *, cwd: Path) -> None:
    print("+", " ".join(cmd))
    subprocess.run(cmd, cwd=cwd, check=True)


def load_base_metadata() -> dict:
    with open(BEACON_CLI_ROOT / "package.json", "r", encoding="utf-8") as fh:
        base = json.load(fh)
    return {
        "license": base.get("license", "Apache-2.0"),
        "repository": base.get("repository"),
        "homepage": base.get("homepage", "https://github.com/tytsxai/beacon-code"),
    }


def resolve_binary(vendor_src: Path, pkg: PlatformPackage) -> Path:
    candidate = vendor_src / pkg.target / "code" / "code"
    if "windows" in pkg.target:
        candidate = candidate.with_suffix(".exe")
    if not candidate.exists():
        raise FileNotFoundError(f"Missing binary for {pkg.target}: {candidate}")
    return candidate


def stage_one(
    pkg: PlatformPackage,
    version: str,
    vendor_src: Path,
    output_dir: Path,
    *,
    keep_staging_dirs: bool,
) -> Path:
    staging_dir = Path(tempfile.mkdtemp(prefix="beacon-platform-npm-"))
    try:
        (staging_dir / "bin").mkdir(parents=True, exist_ok=True)

        binary_src = resolve_binary(vendor_src, pkg)
        binary_name = pkg.binary_name()
        shutil.copy2(binary_src, staging_dir / "bin" / binary_name)

        meta = load_base_metadata()
        package_json = {
            "name": pkg.package_name,
            "version": version,
            "description": "Platform-specific native binary for Beacon Code.",
            "license": meta["license"],
            "repository": meta["repository"],
            "homepage": meta["homepage"],
            "os": [pkg.os],
            "cpu": [pkg.cpu],
            "files": ["bin/**"],
            "bin": {"code": f"bin/{binary_name}"},
        }

        with open(staging_dir / "package.json", "w", encoding="utf-8") as out:
            json.dump(package_json, out, indent=2)
            out.write("\n")

        license_src = REPO_ROOT / "LICENSE"
        if license_src.exists():
            shutil.copy2(license_src, staging_dir / "LICENSE")

        readme = staging_dir / "README.md"
        readme.write_text(
            f"# {pkg.package_name}\n\n"
            "This package contains the native `code` binary for a single platform.\n"
            "It is consumed as an optionalDependency of `@tytsxai/beacon-code`.\n",
            encoding="utf-8",
        )

        output_dir.mkdir(parents=True, exist_ok=True)
        run(["npm", "pack", "--silent"], cwd=staging_dir)
        tgz = next(staging_dir.glob("*.tgz"))
        dest = output_dir / tgz.name
        if dest.exists():
            dest.unlink()
        shutil.move(str(tgz), dest)
        return dest
    finally:
        if not keep_staging_dirs:
            shutil.rmtree(staging_dir, ignore_errors=True)


def main() -> int:
    args = parse_args()
    output_dir = args.output_dir or (REPO_ROOT / "dist" / "npm")
    vendor_src = args.vendor_src.resolve()

    outputs: list[Path] = []
    for pkg in PLATFORM_PACKAGES:
        out = stage_one(
            pkg,
            args.release_version,
            vendor_src,
            output_dir,
            keep_staging_dirs=args.keep_staging_dirs,
        )
        outputs.append(out)
        print(f"Staged {pkg.package_name} at {out}")

    print(f"Done. Wrote {len(outputs)} tarballs to {output_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

