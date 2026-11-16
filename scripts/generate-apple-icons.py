#!/usr/bin/env python3
"""Render the existing RouteLoom SVG mark into both Apple asset catalogs.

Source of truth: website/assets/mark.svg. This deliberately supports only the SVG
elements/path commands used by that mark and fails on unsupported geometry.
Rendering: SVG geometry -> ReportLab vector PDF -> PDFium -> Pillow PNG.
Reproducible environment: reportlab 4.4.9, pypdfium2 5.13.0, Pillow 13.0.0.
Run normally to regenerate, or with --check to verify dimensions, pixels and PNGs.
No Xcode project, Contents.json, signing configuration or icon design is changed.
"""
import argparse
from decimal import Decimal
import hashlib
import io
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

from PIL import Image, PngImagePlugin
import pypdfium2 as pdfium
from reportlab.lib.colors import HexColor
from reportlab.pdfgen.canvas import Canvas


ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "website/assets/mark.svg"
TOKEN = re.compile(r"[MmLlHhVvCcZz]|[+-]?(?:\d*\.\d+|\d+\.?)(?:[eE][+-]?\d+)?")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def color(value):
    require(re.fullmatch(r"#[0-9a-fA-F]{6}", value) is not None, "Only six-digit SVG colors are supported")
    return HexColor(value)


def draw_path(canvas, text):
    tokens = TOKEN.findall(text)
    require(not TOKEN.sub("", text).replace(",", "").strip(), "Unsupported SVG path syntax")
    path = canvas.beginPath()
    command = None
    index = 0
    x = y = start_x = start_y = 0.0
    count = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6}
    while index < len(tokens):
        if tokens[index].isalpha():
            command = tokens[index]
            index += 1
            if command.upper() == "Z":
                path.close()
                x, y = start_x, start_y
                command = None
                continue
        require(command is not None and command.upper() in count, "SVG path command is missing")
        arity = count[command.upper()]
        values = tokens[index:index + arity]
        require(len(values) == arity and not any(value.isalpha() for value in values), "Incomplete SVG path command")
        values = list(map(float, values))
        index += arity
        relative = command.islower()
        origin_x, origin_y = (x, y) if relative else (0.0, 0.0)
        if command.upper() in {"M", "L"}:
            x, y = origin_x + values[0], origin_y + values[1]
            if command.upper() == "M":
                path.moveTo(x, y)
                start_x, start_y = x, y
                command = "l" if relative else "L"
            else:
                path.lineTo(x, y)
        elif command.upper() == "H":
            x = origin_x + values[0]
            path.lineTo(x, y)
        elif command.upper() == "V":
            y = origin_y + values[0]
            path.lineTo(x, y)
        elif command.upper() == "C":
            path.curveTo(origin_x + values[0], origin_y + values[1],
                         origin_x + values[2], origin_y + values[3],
                         origin_x + values[4], origin_y + values[5])
            x, y = origin_x + values[4], origin_y + values[5]
    canvas.drawPath(path, stroke=1, fill=0)


def vector_pdf(source):
    svg = ET.fromstring(source)
    require(svg.tag.rsplit("}", 1)[-1] == "svg", "Source is not SVG")
    require(set(svg.attrib) == {"viewBox"}, "Unsupported SVG root attributes")
    viewbox = list(map(float, svg.attrib["viewBox"].split()))
    require(len(viewbox) == 4 and viewbox[:2] == [0, 0] and viewbox[2] == viewbox[3], "Expected square origin-zero mark")
    side = viewbox[2]
    output = io.BytesIO()
    canvas = Canvas(output, pagesize=(side, side), invariant=1, pageCompression=1)
    canvas.translate(0, side)
    canvas.scale(1, -1)
    background = None
    for element in svg:
        tag = element.tag.rsplit("}", 1)[-1]
        attrs = element.attrib
        if tag == "rect":
            require(set(attrs) <= {"x", "y", "width", "height", "rx", "ry", "fill"}, "Unsupported SVG rectangle attributes")
            x, y = float(attrs.get("x", "0")), float(attrs.get("y", "0"))
            width, height = float(attrs["width"]), float(attrs["height"])
            radius = float(attrs.get("rx", "0"))
            require(float(attrs.get("ry", str(radius))) == radius, "Elliptical SVG corner radii are unsupported")
            canvas.setFillColor(color(attrs["fill"]))
            canvas.roundRect(x, y, width, height, radius, stroke=0, fill=1)
            if x == 0 and y == 0 and width == side and height == side:
                background = tuple(int(attrs["fill"][position:position + 2], 16) for position in (1, 3, 5))
        elif tag == "path":
            require(set(attrs) <= {"d", "fill", "stroke", "stroke-width", "stroke-linecap", "stroke-linejoin"},
                    "Unsupported SVG path attributes")
            require(attrs.get("fill") == "none", "Only unfilled mark strokes are supported")
            require(attrs.get("stroke-linecap") == "round" and attrs.get("stroke-linejoin") == "round",
                    "Expected existing round mark caps and joins")
            canvas.setStrokeColor(color(attrs["stroke"]))
            canvas.setLineWidth(float(attrs["stroke-width"]))
            canvas.setLineCap(1)
            canvas.setLineJoin(1)
            draw_path(canvas, attrs["d"])
        else:
            raise ValueError("Unsupported SVG element: " + tag)
    require(background is not None, "The mark's background color is missing")
    canvas.showPage()
    canvas.save()
    return output.getvalue(), side, background


def catalog_sizes(platform):
    catalog = ROOT / "apps/client" / platform / "Runner/Assets.xcassets/AppIcon.appiconset"
    images = json.loads((catalog / "Contents.json").read_text(encoding="utf-8"))["images"]
    sizes = {}
    for item in images:
        filename = item["filename"]
        require(Path(filename).name == filename and filename.endswith(".png"), "Invalid catalog filename")
        width, height = map(Decimal, item["size"].split("x"))
        scale = Decimal(item["scale"].removesuffix("x"))
        pixels = width * scale
        require(width == height and pixels == int(pixels), "Invalid square icon size")
        require(filename not in sizes or sizes[filename] == int(pixels), "Conflicting icon catalog sizes")
        sizes[filename] = int(pixels)
    return catalog, sizes


def render_png(page, side, background, size, platform, source_hash):
    # Keep the existing SVG's transparent rounded boundary on macOS. iOS applies
    # its own icon mask, so composite the same mark over its orange background.
    fill = background + (255,) if platform == "ios" else (0, 0, 0, 0)
    bitmap = page.render(scale=size * 4 / side, fill_color=fill, rev_byteorder=True)
    image = bitmap.to_pil().copy()
    bitmap.close()
    image = image.convert("RGB" if platform == "ios" else "RGBA")
    image = image.resize((size, size), Image.Resampling.LANCZOS)
    metadata = PngImagePlugin.PngInfo()
    metadata.add_text("Source", "website/assets/mark.svg")
    metadata.add_text("Source-SHA256", source_hash)
    output = io.BytesIO()
    image.save(output, format="PNG", pnginfo=metadata, compress_level=9)
    return output.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Verify existing files without writing")
    args = parser.parse_args()
    source = SOURCE.read_bytes()
    source_hash = hashlib.sha256(source).hexdigest()
    pdf, side, background = vector_pdf(source)
    count = 0
    with pdfium.PdfDocument(pdf) as document:
        page = document[0]
        for platform in ("ios", "macos"):
            catalog, sizes = catalog_sizes(platform)
            for filename, size in sorted(sizes.items()):
                png = render_png(page, side, background, size, platform, source_hash)
                destination = catalog / filename
                if args.check:
                    require(destination.read_bytes() == png, "Icon needs regeneration: " + str(destination.relative_to(ROOT)))
                else:
                    destination.write_bytes(png)
                with Image.open(destination) as image:
                    require(image.size == (size, size), "Icon dimensions are wrong")
                    require(platform != "ios" or image.mode == "RGB", "iOS icon must be RGB without an alpha channel")
                count += 1
        page.close()
    print(("Verified" if args.check else "Generated") + " " + str(count) + " Apple icons from website/assets/mark.svg")
    print("Source SHA-256: " + source_hash)


if __name__ == "__main__":
    main()
