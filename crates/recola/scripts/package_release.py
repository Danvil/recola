#!/usr/bin/env python3
import os
import shutil
import subprocess
from pathlib import Path

def main():
    repo_root = Path(".")

    tmp_dir = repo_root / "tmp" / "recola" / "release"
    tmp_dir.mkdir(parents=True, exist_ok=True)

    tmp_dir_assets = repo_root / "tmp" / "recola" / "release-assets"
    tmp_dir_assets.mkdir(parents=True, exist_ok=True)

    # 1. Build release
    subprocess.run([
        "cargo", "build", "--release", "-p", "recola",
    ], cwd=".", check=True)

    # 2. Copy binary
    bin_src = repo_root / "target" / "release" / ("recola.exe" if os.name == "nt" else "recola")
    bin_dst = tmp_dir / bin_src.name
    shutil.copy2(bin_src, bin_dst)

    # 3.a Copy recola assets
    assets_dir = repo_root / "assets" / "recola"
    tmp_asset_dir = tmp_dir_assets / "assets" / "recola"
    tmp_asset_dir.mkdir(parents=True, exist_ok=True)
    for ext in ("*.glb", "*.json"):
        for src in assets_dir.glob(ext):
            shutil.copy2(src, tmp_asset_dir / src.name)

    # 3.b Copy candy assets
    assets_dir = Path("I:/Ikabur/candy/crates/candy/shaders")
    tmp_asset_dir = tmp_dir_assets / "assets" / "shaders"
    tmp_asset_dir.mkdir(parents=True, exist_ok=True)
    for ext in ("*.wgsl",):
        for src in assets_dir.glob(ext):
            shutil.copy2(src, tmp_asset_dir / src.name)

    # Pack assets
    subprocess.run([
        "cargo", "run", "--manifest-path", "../candy/Cargo.toml",
        "--release", "--bin", "candy_asset_packer", "--",
        "--input-dir", tmp_dir_assets,
        "--exclude", "overgrowth",
        "--exclude", "*.blend",
        "--exclude", "*.blend1",
        "--db-file", tmp_dir / "recola.candy"
    ], cwd=".", check=True)

    # 4. Package zip
    zip_path = "../recola-release.7z"
    subprocess.run(
        ["C:/Program Files/7-Zip/7z.exe", "a", str(zip_path), "."],
        cwd=tmp_dir,
        check=True
    )

    print(f"Created package: {zip_path}")

if __name__ == "__main__":
    main()
