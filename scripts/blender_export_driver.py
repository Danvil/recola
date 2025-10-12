# blender_export_driver.py
import os, json, hashlib, subprocess
from pathlib import Path

BLENDER = "C:/Program Files/Blender Foundation/Blender 4.5/blender.exe"

ROOT = Path(".").resolve()
GLB_EXPORTER   = ROOT / "scripts" / "blender_export_glb.py"
LEVEL_EXPORTER = ROOT / "scripts" / "blender_export_level.py"
LEVELS_DIR     = ROOT / "assets" / "recola" / "levels"
PROPS_DIR      = ROOT / "assets" / "recola" / "props"
OUT            = ROOT / "tmp" / "export" / "recola"
CACHE_FILE     = OUT / ".export_cache.json"

OUT.mkdir(parents=True, exist_ok=True)

def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def combined_hash(blend: Path, exporter: Path) -> str:
    # Hash the blend bytes + exporter script bytes. If either changes, we rebuild.
    h = hashlib.sha256()
    for p in (blend, exporter):
        h.update(p.name.encode("utf-8"))
        with p.open("rb") as f:
            for chunk in iter(lambda: f.read(1024 * 1024), b""):
                h.update(chunk)
    return h.hexdigest()

def load_cache() -> dict:
    if CACHE_FILE.is_file():
        try:
            return json.loads(CACHE_FILE.read_text(encoding="utf-8"))
        except Exception:
            return {}
    return {}

def save_cache(cache: dict) -> None:
    CACHE_FILE.write_text(json.dumps(cache, indent=2, sort_keys=True), encoding="utf-8")

def should_export(blend: Path, kind: str, cache: dict) -> tuple[bool, str, Path]:
    stem = blend.stem
    if kind == "asset":
        exporter = GLB_EXPORTER
        out_file = OUT / "props" / f"{stem}.glb"
    else:
        exporter = LEVEL_EXPORTER
        out_file = OUT / "levels" / f"{stem}.json"

    new_hash = combined_hash(blend, exporter)
    key = str(blend.resolve())

    # Export if hash changed or output is missing.
    prev = cache.get(key)
    if prev != new_hash or not out_file.is_file():
        return True, new_hash, out_file
    return False, new_hash, out_file

def run_asset(blend: Path, out: Path):
    subprocess.run(
        [
            BLENDER, "--background", "--factory-startup",
            str(blend),
            "--python", str(GLB_EXPORTER),
            "--", "--out", str(out),
        ],
        check=True,
    )

def run_level(blend: Path, out: Path):
    subprocess.run(
        [
            BLENDER, "--background", "--factory-startup",
            str(blend),
            "--python", str(LEVEL_EXPORTER),
            "--", "--out", str(out),
        ],
        check=True,
    )

def process_dir(dir_path: Path, kind: str, cache: dict) -> None:
    if not dir_path.is_dir():
        return
    for b in sorted(dir_path.rglob("*.blend")):
        do_export, h, out_file = should_export(b, kind, cache)
        header = f"===== {kind.upper()}: {b}"
        if do_export:
            print(header + "  -> exporting")
            if kind == "asset":
                run_asset(b, OUT / "props")
            else:
                run_level(b, OUT / "levels")
            if out_file.is_file():
                cache[str(b.resolve())] = h
            else:
                print(f"WARNING: expected output not found: {out_file}")
        else:
            print(header + "  -> up-to-date")

def main():
    cache = load_cache()
    process_dir(PROPS_DIR,  "asset", cache)
    process_dir(LEVELS_DIR, "level", cache)
    save_cache(cache)

if __name__ == "__main__":
    main()
