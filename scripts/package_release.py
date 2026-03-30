#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import shutil
import tarfile
import zipfile
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Package a QBNex release artifact with docs and checksums."
    )
    parser.add_argument("--binary", required=True, help="Path to the built qb binary")
    parser.add_argument("--version", required=True, help="Release version label")
    parser.add_argument("--target", required=True, help="Target/platform label")
    parser.add_argument("--output-dir", required=True, help="Directory to place packaged artifacts")
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_checksum(path: Path) -> Path:
    checksum_path = path.with_name(path.name + ".sha256")
    checksum_path.write_text(f"{sha256(path)}  {path.name}\n", encoding="utf-8")
    return checksum_path


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parent.parent
    binary_path = Path(args.binary).resolve()
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    bundle_name = f"qbnex-{args.version}-{args.target}"
    bundle_dir = output_dir / bundle_name
    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True)

    if not binary_path.exists():
        raise FileNotFoundError(f"release binary does not exist: {binary_path}")

    shutil.copy2(binary_path, bundle_dir / binary_path.name)
    for extra_name in ("README.md", "LICENSE", "CHANGELOG.md"):
        extra_path = repo_root / extra_name
        if extra_path.exists():
            shutil.copy2(extra_path, bundle_dir / extra_name)

    if binary_path.suffix.lower() == ".exe":
        archive_path = output_dir / f"{bundle_name}.zip"
        with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(bundle_dir.rglob("*")):
                if path.is_file():
                    archive.write(path, arcname=path.relative_to(output_dir))
    else:
        archive_path = output_dir / f"{bundle_name}.tar.gz"
        with tarfile.open(archive_path, "w:gz") as archive:
            archive.add(bundle_dir, arcname=bundle_dir.name)

    checksum_path = write_checksum(archive_path)
    print(archive_path)
    print(checksum_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
