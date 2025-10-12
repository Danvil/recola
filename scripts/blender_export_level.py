import argparse
import bpy
import json
import mathutils
import os
import sys
from pathlib import Path

# ---------- helpers ----------

def world_xform(obj):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    eval_obj = obj.evaluated_get(depsgraph)
    loc, rot, scl = eval_obj.matrix_world.decompose()
    rot = rot.normalized()
    return (
        [float(loc.x), float(loc.y), float(loc.z)],
        [float(rot.x), float(rot.y), float(rot.z), float(rot.w)],  # quat
        [float(scl.x), float(scl.y), float(scl.z)],
    )

def clean_name(name: str) -> str:
    if name.endswith(tuple(f".{i:03d}" for i in range(1, 1000))):
        return name.rsplit(".", 1)[0]
    return name

def asset_id_for_object(obj) -> str:
    if "asset_id" in obj:
        return str(obj["asset_id"])
    if obj.instance_type == 'COLLECTION' and obj.instance_collection:
        col = obj.instance_collection
        return str(col.get("asset_id", clean_name(col.name)))
    if obj.library or (obj.data and obj.data.library):
        return clean_name(obj.name)
    return None

def is_exportable_instance(obj) -> bool:
    if obj.instance_type == 'COLLECTION' and obj.instance_collection:
        return True
    if obj.type == 'MESH':
        return True
    return False

def is_visible_in_view_layer(obj, view_layer) -> bool:
    if not obj.visible_get(view_layer=view_layer):
        return False
    if getattr(obj, "hide_viewport", False):
        return False
    if getattr(obj, "hide_render", False):
        return False
    if obj.hide_get():
        return False
    return True

# --- custom properties serialization ---

def _json_sanitize(v):
    # Convert Blender/IDProperty types into plain JSON types.
    if isinstance(v, (str, int, bool, float)) or v is None:
        return v
    if isinstance(v, (list, tuple)):
        return [_json_sanitize(x) for x in v]
    if isinstance(v, (mathutils.Vector, mathutils.Euler)):
        return [float(x) for x in v]
    if isinstance(v, mathutils.Quaternion):
        q = v.normalized()
        return [float(q.x), float(q.y), float(q.z), float(q.w)]
    if isinstance(v, mathutils.Color):
        return [float(v.r), float(v.g), float(v.b)]
    # IDPropertyArray and similar sequence-like values
    try:
        return [_json_sanitize(x) for x in v]
    except Exception:
        return str(v)

def collect_custom_props(id_block) -> dict:
    # id_block.items() yields only user-defined properties; filter out UI meta.
    return {k: _json_sanitize(v) for k, v in id_block.items() if k != "_RNA_UI"}

def blend_name() -> str:
    filepath = bpy.context.blend_data.filepath
    filename = bpy.path.basename(filepath)
    filestem = os.path.splitext(filename)[0]
    return filestem

# ---------- collect instances ----------

def collect_all_instances():
    instances = []
    vl = bpy.context.view_layer

    for obj in vl.objects:
    #    if not is_exportable_instance(obj):
    #        continue
        if obj.hide_get():   # skip if invisible via eye icon
            continue

        location, rotation, scale = world_xform(obj)

        entry = {
            "name": obj.name,
            #"kind": ("collection" if obj.instance_type == 'COLLECTION' else "object"),
            "location": location,
            "rotation": rotation,   # quaternion [x, y, z, w]
            "scale": scale,
            "custom": collect_custom_props(obj),  # custom props on the instance object
        }

        asset_id = asset_id_for_object(obj)
        if asset_id != None:
            entry["asset_id"] = asset_id


    #    # If this is a collection instance, also export properties on the source collection.
    #    if obj.instance_type == 'COLLECTION' and obj.instance_collection:
    #        entry["collection_custom"] = collect_custom_props(obj.instance_collection)

        instances.append(entry)

    return instances


# ---------- write json ----------

def export(out_dir):
    output_path = out_dir / (blend_name() + ".json")
    print(f"Exporting to {output_path}")

    instances = collect_all_instances()

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump({"instances": instances}, f, indent=2)

    print(f"  Exported {len(instances)} instances to {output_path}")


def main(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)
    export(out_dir)


if __name__ == "__main__":
    # Blender adds its own args before and after '--'
    argv = sys.argv[(sys.argv.index("--") + 1):] if "--" in sys.argv else []
    main(argv)
