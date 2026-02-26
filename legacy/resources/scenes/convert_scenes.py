#!/usr/bin/env python3
"""Convert legacy .sc scene files to the new JSON format."""

import json
import math
import os
import re


def fix_trailing_commas(text):
    """Remove trailing commas before } or ] in JSON-like text."""
    text = re.sub(r",\s*}", "}", text)
    text = re.sub(r",\s*\]", "]", text)
    return text


def vec3_sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def vec3_length(v):
    return math.sqrt(sum(x * x for x in v))


def vec3_normalize(v):
    l = vec3_length(v)
    if l < 1e-10:
        return [0.0, 1.0, 0.0]
    return [round(v[0] / l, 6), round(v[1] / l, 6), round(v[2] / l, 6)]


def r(val):
    """Round a float for clean JSON output."""
    if isinstance(val, list):
        return [r(x) for x in val]
    if isinstance(val, float):
        rounded = round(val, 4)
        if rounded == int(rounded):
            return int(rounded) * 1.0  # keep as float but clean
        return rounded
    return val


def convert_material(fig):
    """Convert legacy material string + fields to PBR Material dict."""
    mat_type = fig.get("material", "diffuse")
    color = fig.get("color", [0.8, 0.8, 0.8])
    emission_val = fig.get("emission", 0.0)
    specular = fig.get("specular", None)

    mat = {"base_color": color}

    if mat_type == "diffuse":
        mat["roughness"] = specular if specular is not None else 0.8
    elif mat_type == "emissive":
        mat["roughness"] = 0.9
        mat["emission"] = color[:]
        mat["emission_strength"] = emission_val if emission_val > 0 else 5.0
    elif mat_type == "reflect":
        mat["metallic"] = 1.0
        mat["roughness"] = specular if specular is not None else 0.05
    elif mat_type == "glass":
        mat["transmission"] = 1.0
        mat["ior"] = 1.5
        mat["roughness"] = 0.0
    elif mat_type == "transparent":
        mat["transmission"] = 1.0
        mat["ior"] = 1.0
        mat["roughness"] = 0.0
    elif mat_type == "negative":
        mat["base_color"] = [0.0, 0.0, 0.0]
        mat["roughness"] = 1.0

    return mat


def convert_shape(fig):
    """Convert a legacy figure dict to a new Shape dict."""
    fig_type = fig.get("type", "sphere")
    center = fig.get("center", [0.0, 0.0, 0.0])
    center2 = fig.get("center2", None)
    center3 = fig.get("center3", None)
    radius = fig.get("radius", None)
    radius2 = fig.get("radius2", None)
    angle = fig.get("angle", None)
    normal = fig.get("normal", None)

    # Type renaming
    type_map = {
        "julia_fract": "julia",
        "parabolid": "paraboloid",
    }
    shape_type = type_map.get(fig_type, fig_type)

    shape = {"type": shape_type}

    if shape_type == "sphere":
        shape["position"] = center
        if radius is not None:
            shape["radius"] = radius

    elif shape_type == "plane":
        shape["position"] = center
        if normal:
            shape["normal"] = normal
        elif center2:
            shape["normal"] = center2

    elif shape_type == "disc":
        shape["position"] = center
        if center2:
            shape["normal"] = center2
        if radius is not None:
            shape["radius"] = radius

    elif shape_type in ("cylinder", "cone", "paraboloid", "hyperboloid"):
        shape["position"] = center
        h = 1.0
        if center2:
            axis = vec3_sub(center2, center)
            h = vec3_length(axis)
            shape["normal"] = vec3_normalize(axis)
            if h > 0:
                shape["height"] = round(h, 4)
        if shape_type == "cone" and angle is not None:
            shape["radius"] = round(h * math.tan(math.radians(angle)), 4)
        elif radius is not None:
            shape["radius"] = radius
        if radius2 is not None and radius2 != 0:
            shape["radius2"] = radius2

    elif shape_type == "torus":
        shape["position"] = center
        if center2:
            shape["normal"] = center2
        if radius is not None:
            shape["radius"] = radius
        if radius2 is not None:
            shape["radius2"] = radius2

    elif shape_type in ("cube", "pyramid"):
        shape["position"] = center
        if center2:
            axis = vec3_sub(center2, center)
            if vec3_length(axis) > 1e-10:
                shape["normal"] = vec3_normalize(axis)
        if radius is not None:
            shape["radius"] = radius
        if radius2 is not None:
            shape["radius2"] = radius2

    elif shape_type == "triangle":
        shape["position"] = [0.0, 0.0, 0.0]
        shape["v0"] = center
        shape["v1"] = center2 if center2 else [0.0, 0.0, 0.0]
        shape["v2"] = center3 if center3 else [0.0, 0.0, 0.0]

    elif shape_type == "tetrahedron":
        shape["position"] = center
        if center2:
            axis = vec3_sub(center2, center)
            if vec3_length(axis) > 1e-10:
                shape["normal"] = vec3_normalize(axis)
        if radius is not None:
            shape["radius"] = radius

    elif shape_type == "ellipsoid":
        shape["position"] = center
        if center2 and vec3_length(center2) > 1e-10:
            shape["rotation"] = center2
        if radius is not None:
            shape["radius"] = radius

    elif shape_type == "mebius":
        if center != [0, 0, 0] and center != [0.0, 0.0, 0.0]:
            shape["position"] = center
        if center2:
            shape["normal"] = center2
        if radius is not None:
            shape["radius"] = radius
        if radius2 is not None:
            shape["radius2"] = radius2

    elif shape_type == "mandelbulb":
        shape["position"] = center
        if radius is not None and radius > 0:
            shape["radius"] = radius
        else:
            shape["radius"] = 1.5
        shape["power"] = 8
        shape["max_iterations"] = 12

    elif shape_type == "julia":
        # In legacy, center encoded the Julia C constant, not position
        shape["position"] = [0.0, 0.0, 0.0]
        shape["rotation"] = center
        shape["radius2"] = 0.0
        shape["radius"] = 1.5
        shape["max_iterations"] = 14

    elif shape_type == "skybox":
        shape["position"] = [0.0, 0.0, 0.0]

    # Material
    shape["material"] = convert_material(fig)

    return shape


def convert_scene(sc_data):
    """Convert a legacy scene dict to the new format."""
    scene = {}

    # Camera
    if "camera" in sc_data:
        cam = sc_data["camera"]
        scene["camera"] = {
            "position": cam.get("position", [0.0, 0.0, 0.0]),
            "rotation": cam.get("angles", [0.0, 0.0, 0.0]),
            "fov": cam.get("fov", 60.0),
            "exposure": cam.get("exposure", 1.0),
        }

    # Shapes
    scene["shapes"] = [convert_shape(fig) for fig in sc_data.get("figures", [])]

    # Models (external_object)
    ext_obj = sc_data.get("external_object")
    if ext_obj:
        scene["models"] = [
            {"path": ext_obj, "position": [0.0, 0.0, 0.0], "scale": 1.0}
        ]

    return scene


def main():
    scenes_dir = "/home/phrytsenko/Projects/PathTracer/resources/scenes"

    for filename in sorted(os.listdir(scenes_dir)):
        if not filename.endswith(".sc"):
            continue

        sc_path = os.path.join(scenes_dir, filename)
        json_name = filename.replace(".sc", ".json")
        json_path = os.path.join(scenes_dir, json_name)

        # Skip if JSON already exists
        if os.path.exists(json_path):
            print(f"SKIP {filename} (JSON already exists)")
            continue

        with open(sc_path, "r") as f:
            raw = f.read()

        fixed = fix_trailing_commas(raw)

        try:
            sc_data = json.loads(fixed)
        except json.JSONDecodeError as e:
            print(f"ERROR {filename}: {e}")
            continue

        scene = convert_scene(sc_data)

        with open(json_path, "w") as f:
            json.dump(scene, f, indent=2)
            f.write("\n")

        print(f"OK    {filename} -> {json_name}")


if __name__ == "__main__":
    main()
