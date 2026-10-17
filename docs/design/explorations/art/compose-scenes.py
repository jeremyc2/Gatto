"""Assemble unchanged generated plates with one canonical picture prop.

Run with Python 3. No image processing or third-party dependencies are used.
"""

import base64
from pathlib import Path

ART = Path(__file__).resolve().parent
POSITIONS = {"bring": (610, 792), "upload": (660, 740), "copy": (540, 758)}


def image_data(filename):
    return "data:image/png;base64," + base64.b64encode(
        (ART / filename).read_bytes()
    ).decode()


for beat, (x, y) in POSITIONS.items():
    plate = (
        "story-upload-github-plate-v2.png"
        if beat == "upload"
        else f"story-{beat}-plate-v2.png"
    )
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" width="1536" height="1024" viewBox="0 0 1536 1024" role="img"><title>Gatto: {beat}, shared discovery redraw</title><desc>Three black cats with distinct builds and materials. The flower picture is the identical reusable asset in every scene.</desc><image width="1536" height="1024" href="{image_data(plate)}"/><g transform="matrix(.9 .08 -.48 .28 {x} {y})"><rect x="3" y="7" width="430" height="286.666667" fill="#59452d" opacity=".19"/><image width="430" height="286.666667" preserveAspectRatio="xMidYMid meet" href="{image_data('flower-picture-v2.png')}"/><rect width="430" height="286.666667" fill="none" stroke="#b59b70" stroke-width="2"/></g></svg>'''
    (ART / f"story-{beat}-v2.svg").write_text(svg)
