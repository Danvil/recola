#!/usr/bin/env python3
import os
import shutil
import subprocess
from pathlib import Path
import argparse


def package_assets():
    repo_root = Path(".")

    tmp_dir = repo_root / "tmp" / "recola" / "release"
    tmp_dir.mkdir(parents=True, exist_ok=True)

    tmp_dir_assets = repo_root / "tmp" / "recola" / "release-assets"
    tmp_dir_assets.mkdir(parents=True, exist_ok=True)

    # 3.a Copy recola assets
    for sub in ["props", "levels"]:
        assets_dir = repo_root / "tmp" / "export" / "recola" / sub
        tmp_asset_dir = tmp_dir_assets / "assets" / "recola" / sub
        tmp_asset_dir.mkdir(parents=True, exist_ok=True)
        for ext in ("*.glb", "*.json"):
            for src in assets_dir.glob(ext):
                shutil.copy2(src, tmp_asset_dir / src.name)

    # 3.a Copy props.json
    shutil.copy2(
        repo_root / "assets" / "recola" / "props.json",
        tmp_dir_assets / "assets" / "recola" / "props.json",
    )

    # 3. audio
    for sub in ["effects", "music"]:
        assets_dir = repo_root / "assets" / "recola" / "audio" / sub
        tmp_asset_dir = tmp_dir_assets / "assets" / "recola" / "audio" / sub
        tmp_asset_dir.mkdir(parents=True, exist_ok=True)
        for ext in ("*.wav",):
            for src in assets_dir.glob(ext):
                shutil.copy2(src, tmp_asset_dir / src.name)

    # 3.b Copy candy assets
    assets_dir = Path("I:/Ikabur/atuin/crates/candy/candy_glassworks/shaders")
    tmp_asset_dir = tmp_dir_assets / "assets" / "shaders"
    tmp_asset_dir.mkdir(parents=True, exist_ok=True)
    for ext in ("*.wgsl",):
        for src in assets_dir.glob(ext):
            shutil.copy2(src, tmp_asset_dir / src.name)

    for folder in ["bloom", "fxaa", "screen_space_quad", "sky", "tonemap"]:
        assets_dir = (
            Path("I:/Ikabur/atuin/crates/candy/candy_render_nodes/src") / folder
        )
        tmp_asset_dir = tmp_dir_assets / "assets" / "candy" / folder
        tmp_asset_dir.mkdir(parents=True, exist_ok=True)
        for ext in ("*.wgsl",):
            for src in assets_dir.glob(ext):
                shutil.copy2(src, tmp_asset_dir / src.name)

    # Pack assets
    subprocess.run(
        [
            "cargo",
            "run",
            "--manifest-path",
            "../atuin/Cargo.toml",
            "--release",
            "--bin",
            "candy_asset_packer",
            "--",
            "--input-dir",
            tmp_dir_assets,
            "--exclude",
            "cable/",
            "--exclude",
            "overgrowth/",
            "--exclude",
            "coverart/",
            "--exclude",
            "*.blend",
            "--exclude",
            "*.blend1",
            "--db-file",
            tmp_dir / "recola.candy",
        ],
        cwd=".",
        check=True,
    )


def create_release():
    repo_root = Path(".")

    tmp_dir = repo_root / "tmp" / "recola" / "release"
    tmp_dir.mkdir(parents=True, exist_ok=True)

    # build binary
    subprocess.run(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            "recola",
        ],
        cwd=".",
        check=True,
    )

    # Copy binary
    bin_src = (
        repo_root
        / "target"
        / "release"
        / ("recola.exe" if os.name == "nt" else "recola")
    )
    bin_dst = tmp_dir / bin_src.name
    shutil.copy2(bin_src, bin_dst)

    zip_path = "../recola-release.7z"
    subprocess.run(
        ["C:/Program Files/7-Zip/7z.exe", "a", str(zip_path), "."],
        cwd=tmp_dir,
        check=True,
    )
    print(f"Created package: {zip_path}")


def main():
    parser = argparse.ArgumentParser(description="Release packaging tool.")
    parser.add_argument(
        "--package-assets", action="store_true", help="Run the asset packaging step."
    )
    parser.add_argument(
        "--create-release", action="store_true", help="Run the release creation step."
    )

    args = parser.parse_args()

    if not args.package_assets and not args.create_release:
        parser.error(
            "No action specified. Use --package-assets, --create-release, or both."
        )

    if args.package_assets:
        package_assets()
    if args.create_release:
        create_release()


if __name__ == "__main__":
    main()
