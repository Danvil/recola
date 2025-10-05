#!/usr/bin/env python3
import os
import shutil
import subprocess
import zipfile
from pathlib import Path

def zipfolder(zip_path, root):
    zipobj = zipfile.ZipFile(zip_path, 'w', zipfile.ZIP_DEFLATED)
    rootlen = len(str(root)) + 1
    for base, dirs, files in os.walk(root):
        for file in files:
            fn = os.path.join(base, file)
            zipobj.write(fn, fn[rootlen:])


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
        "--db-file", tmp_dir / "recola.candy"
    ], cwd=".", check=True)

    # 4. Package zip
    zip_path = repo_root / "tmp" / "recola" / "recola-release.zip"
    zipfolder(zip_path, tmp_dir)

    print(f"Created package: {zip_path}")

if __name__ == "__main__":
    main()
