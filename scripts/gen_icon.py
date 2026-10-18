#!/usr/bin/env python3
"""Generate app icon (ICO) for edge-agent Windows installer."""
from PIL import Image, ImageDraw
import os
import subprocess
import tempfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
OUTPUT_PATH = os.path.join(PROJECT_ROOT, "assets", "app-icon.ico")


def create_icon(size):
    """Create a single icon image at given size."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Colors
    primary = (0x7C, 0xB3, 0x42, 0xFF)      # #7CB342 green
    dark = (0x5A, 0x8E, 0x2E, 0xFF)          # darker green border
    white = (0xFF, 0xFF, 0xFF, 0xFF)
    light = (0xA8, 0xD4, 0x70, 0xFF)         # lighter green highlight

    margin = max(1, size // 16)
    radius = size // 4

    # Draw rounded rectangle background
    draw.rounded_rectangle(
        [(margin, margin), (size - margin, size - margin)],
        radius=radius,
        fill=primary,
        outline=dark,
        width=max(1, size // 32)
    )

    # Draw a simple "eye" symbol in the center (visual inspection theme)
    cx, cy = size // 2, size // 2
    eye_w = size * 6 // 16   # eye width
    eye_h = size * 4 // 16   # eye height

    # Eye outline (white ellipse)
    draw.ellipse(
        [(cx - eye_w, cy - eye_h), (cx + eye_w, cy + eye_h)],
        outline=white,
        width=max(1, size // 24)
    )

    # Pupil (white filled circle)
    pupil_r = max(1, size // 16)
    draw.ellipse(
        [(cx - pupil_r, cy - pupil_r), (cx + pupil_r, cy + pupil_r)],
        fill=white
    )

    # Highlight dot on pupil (lighter green)
    hl_r = max(1, pupil_r // 3)
    hl_offset = pupil_r // 2
    draw.ellipse(
        [(cx - hl_r - hl_offset, cy - hl_r - hl_offset),
         (cx + hl_r - hl_offset, cy + hl_r - hl_offset)],
        fill=light
    )

    return img


def main():
    sizes = [16, 32, 48, 64, 128, 256]

    os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)

    with tempfile.TemporaryDirectory() as tmpdir:
        png_paths = []
        for s in sizes:
            img = create_icon(s)
            path = os.path.join(tmpdir, f"icon_{s}.png")
            img.save(path, "PNG")
            png_paths.append(path)

        # Use ImageMagick to merge PNGs into multi-size ICO
        cmd = ["magick", "convert"] + png_paths + [OUTPUT_PATH]
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            # Fallback to old 'convert' command
            cmd = ["convert"] + png_paths + [OUTPUT_PATH]
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                print("Error creating ICO:", result.stderr)
                return

    # Verify
    file_size = os.path.getsize(OUTPUT_PATH)
    print(f"Generated {OUTPUT_PATH}")
    print(f"  File size: {file_size} bytes")
    print(f"  Sizes included: {sizes}")


if __name__ == "__main__":
    main()
