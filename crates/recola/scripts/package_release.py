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
    crate_dir = repo_root / "crates" / "recola"
    assets_dir = repo_root / "assets" / "recola"
    tmp_dir = repo_root / "tmp" / "recola" / "release"
    tmp_dir.mkdir(parents=True, exist_ok=True)

    # 1. Build release
    subprocess.run(["cargo", "build", "--release", "-p", "recola"], cwd=crate_dir, check=True)

    # 2. Copy binary
    bin_src = repo_root / "target" / "release" / ("recola.exe" if os.name == "nt" else "recola")
    bin_dst = tmp_dir / bin_src.name
    shutil.copy2(bin_src, bin_dst)

    # 3. Copy assets
    tmp_asset_dir = tmp_dir / "assets" / "recola"
    tmp_asset_dir.mkdir(parents=True, exist_ok=True)
    for ext in ("*.glb", "*.json"):
        for src in assets_dir.glob(ext):
            shutil.copy2(src, tmp_asset_dir / src.name)

    # 4. Package zip
    zip_path = repo_root / "tmp" / "recola" / "recola-release.zip"
    zipfolder(zip_path, tmp_dir)

    print(f"Created package: {zip_path}")

if __name__ == "__main__":
    main()
