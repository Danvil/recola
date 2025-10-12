# export_blend.py (runs inside Blender)
import bpy, sys, argparse
from pathlib import Path
import importlib.util

def export_asset_glb(output_path: Path):
    print(f"Exporting to {output_path}")

    # Example deterministic-ish settings; adjust to your pipeline.
    bpy.ops.object.select_all(action='DESELECT')
    # Optionally select only exportable collections/objects here.

    bpy.ops.export_scene.gltf(
        filepath=str(output_path),
        export_format='GLB',
        use_visible=False,
        use_renderable=False,
        export_yup=False,
        export_apply=True,   # apply modifiers
        export_texcoords=True,
        export_normals=True,
        export_tangents=False,
        export_materials='EXPORT',
        export_vertex_color='NONE',
        export_cameras=False,
        export_lights=False,
        export_extras=False,  # reduce metadata noise
        export_skins=True,
        export_animations=False,   # assets usually static; change if needed
        export_optimize_animation_size=True,
        export_morph=True,
        export_image_format='AUTO',
        export_hierarchy_full_collections=True,
        will_save_settings=False,
    )
    print(f"   Exported to {output_path}")

def parse_args(argv):
    p = argparse.ArgumentParser()
    p.add_argument("--out", required=True)
    return p.parse_args(argv)

def main(argv):
    # Blender passes its own args; split after '--'
    args = parse_args(argv)
    out_dir = Path(args.out)

    # Ensure the .blend file is actually loaded (it is when invoked as `blender file.blend --python ...`)
    if not bpy.data.filepath:
        raise RuntimeError("No .blend loaded; call Blender with a file path before --python.")

    blend = Path(bpy.data.filepath)
    out_dir.mkdir(parents=True, exist_ok=True)
    output_path = out_dir / blend.with_suffix(".glb").name

    export_asset_glb(output_path)

if __name__ == "__main__":
    argv = sys.argv[(sys.argv.index("--") + 1):] if "--" in sys.argv else []
    main(argv)
