#!/usr/bin/env python3
"""Install Beacon Code native binaries (Rust CLI)."""

import argparse
import os
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Iterable, Sequence

SCRIPT_DIR = Path(__file__).resolve().parent
BEACON_CLI_ROOT = SCRIPT_DIR.parent
DEFAULT_WORKFLOW_URL = ""
VENDOR_DIR_NAME = "vendor"
BINARY_TARGETS = (
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
)


@dataclass(frozen=True)
class BinaryComponent:
    artifact_prefix: str  # matches the artifact filename prefix (e.g. code-<target>.zst)
    dest_dir: str  # directory under vendor/<target>/ where the binary is installed
    binary_basename: str  # executable name inside dest_dir (before optional .exe)
    targets: tuple[str, ...] | None = None  # limit installation to specific targets


WINDOWS_TARGETS = tuple(target for target in BINARY_TARGETS if "windows" in target)

BINARY_COMPONENTS = {
    "code": BinaryComponent(
        artifact_prefix="code",
        dest_dir="code",
        binary_basename="code",
    ),
    "code-responses-api-proxy": BinaryComponent(
        artifact_prefix="code-responses-api-proxy",
        dest_dir="code-responses-api-proxy",
        binary_basename="code-responses-api-proxy",
    ),
}

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Install native Beacon Code binaries."
    )
    parser.add_argument(
        "--workflow-url",
        help=(
            "GitHub Actions workflow URL that produced the artifacts."
        ),
    )
    parser.add_argument(
        "--component",
        dest="components",
        action="append",
        choices=tuple(BINARY_COMPONENTS),
        help=(
            "Limit installation to the specified components."
            " May be repeated. Defaults to code (Beacon Code CLI)."
        ),
    )
    parser.add_argument(
        "root",
        nargs="?",
        type=Path,
        help=(
            "Directory containing package.json for the staged package. If omitted, the "
            "repository checkout is used."
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    beacon_cli_root = (args.root or BEACON_CLI_ROOT).resolve()
    vendor_dir = beacon_cli_root / VENDOR_DIR_NAME
    vendor_dir.mkdir(parents=True, exist_ok=True)

    components = args.components or [
        "code",
    ]

    workflow_url = (args.workflow_url or DEFAULT_WORKFLOW_URL).strip()
    if not workflow_url:
        raise RuntimeError(
            "Missing --workflow-url (expected a GitHub Actions run URL that contains the build artifacts)."
        )

    workflow_id = workflow_url.rstrip("/").split("/")[-1]
    print(f"Downloading native artifacts from workflow {workflow_id}...")

    with tempfile.TemporaryDirectory(prefix="beacon-native-artifacts-") as artifacts_dir_str:
        artifacts_dir = Path(artifacts_dir_str)
        _download_artifacts(workflow_id, artifacts_dir)
        install_binary_components(
            artifacts_dir,
            vendor_dir,
            [BINARY_COMPONENTS[name] for name in components if name in BINARY_COMPONENTS],
        )

    print(f"Installed native dependencies into {vendor_dir}")
    return 0


def _download_artifacts(workflow_id: str, dest_dir: Path) -> None:
    cmd = [
        "gh",
        "run",
        "download",
        "--dir",
        str(dest_dir),
        "--repo",
        "tytsxai/beacon-code",
        workflow_id,
    ]
    subprocess.check_call(cmd)


def install_binary_components(
    artifacts_dir: Path,
    vendor_dir: Path,
    selected_components: Sequence[BinaryComponent],
) -> None:
    if not selected_components:
        return

    for component in selected_components:
        component_targets = list(component.targets or BINARY_TARGETS)

        print(
            f"Installing {component.binary_basename} binaries for targets: "
            + ", ".join(component_targets)
        )
        max_workers = min(len(component_targets), max(1, (os.cpu_count() or 1)))
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = {
                executor.submit(
                    _install_single_binary,
                    artifacts_dir,
                    vendor_dir,
                    target,
                    component,
                ): target
                for target in component_targets
            }
            for future in as_completed(futures):
                installed_path = future.result()
                print(f"  installed {installed_path}")


def _install_single_binary(
    artifacts_dir: Path,
    vendor_dir: Path,
    target: str,
    component: BinaryComponent,
) -> Path:
    artifact_subdir = artifacts_dir / target
    archive_name = _archive_name_for_target(component.artifact_prefix, target)
    archive_path = artifact_subdir / archive_name
    if not archive_path.exists():
        raise FileNotFoundError(f"Expected artifact not found: {archive_path}")

    dest_dir = vendor_dir / target / component.dest_dir
    dest_dir.mkdir(parents=True, exist_ok=True)

    binary_name = (
        f"{component.binary_basename}.exe" if "windows" in target else component.binary_basename
    )
    dest = dest_dir / binary_name
    dest.unlink(missing_ok=True)
    extract_archive(archive_path, "zst", None, dest)
    if "windows" not in target:
        dest.chmod(0o755)
    return dest


def _archive_name_for_target(artifact_prefix: str, target: str) -> str:
    if "windows" in target:
        return f"{artifact_prefix}-{target}.exe.zst"
    return f"{artifact_prefix}-{target}.zst"


def extract_archive(
    archive_path: Path,
    archive_format: str,
    archive_member: str | None,
    dest: Path,
) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)

    if archive_format == "zst":
        output_path = archive_path.parent / dest.name
        subprocess.check_call(
            ["zstd", "-f", "-d", str(archive_path), "-o", str(output_path)]
        )
        shutil.move(str(output_path), dest)
        return

    if archive_format == "tar.gz":
        if not archive_member:
            raise RuntimeError("Missing 'path' for tar.gz archive in DotSlash manifest.")
        with tarfile.open(archive_path, "r:gz") as tar:
            try:
                member = tar.getmember(archive_member)
            except KeyError as exc:
                raise RuntimeError(
                    f"Entry '{archive_member}' not found in archive {archive_path}."
                ) from exc
            tar.extract(member, path=archive_path.parent, filter="data")
        extracted = archive_path.parent / archive_member
        shutil.move(str(extracted), dest)
        return

    if archive_format == "zip":
        if not archive_member:
            raise RuntimeError("Missing 'path' for zip archive in DotSlash manifest.")
        with zipfile.ZipFile(archive_path) as archive:
            try:
                with archive.open(archive_member) as src, open(dest, "wb") as out:
                    shutil.copyfileobj(src, out)
            except KeyError as exc:
                raise RuntimeError(
                    f"Entry '{archive_member}' not found in archive {archive_path}."
                ) from exc
        return

    raise RuntimeError(f"Unsupported archive format '{archive_format}'.")


if __name__ == "__main__":
    import sys

    sys.exit(main())
