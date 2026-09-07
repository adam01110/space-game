#!/usr/bin/env python3
"""Generate an ODT directly from the live ExcaliDash drawing.

The Drawing Agent API is documented by ExcaliDash v0.6 and its MCP adapter:
https://github.com/ZimengXiong/ExcaliDash/releases/tag/v0.6.0-dev.a6969c9
https://github.com/chlee1001/excalidash-mcp

The API key is read at runtime and is never written to the document or logs.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from html import escape
from pathlib import Path
from typing import Any
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile

DEFAULT_URL = "https://excalidash.zezura.xyz"
DEFAULT_DRAWING_ID = "b43ce1a4-e0ab-4f62-a43e-4f57cf0017c2"
DEFAULT_KEY_FILE = Path.home() / ".config/sops-nix/secrets/ai/excalidash_key"
DEFAULT_OUTPUT = Path(__file__).resolve().parent / "project-1-analysis.odt"

# These named drawing frames are the document source, in output order.
DOCUMENT_FRAME_NAMES = (
    "Analysis and advice",
    "MoSCoW",
    "Technology study",
)
DOCUMENT_TITLE_PREFIX = "Document title:"

NS = {
    "office": "urn:oasis:names:tc:opendocument:xmlns:office:1.0",
    "style": "urn:oasis:names:tc:opendocument:xmlns:style:1.0",
    "text": "urn:oasis:names:tc:opendocument:xmlns:text:1.0",
    "fo": "urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0",
    "xlink": "http://www.w3.org/1999/xlink",
    "dc": "http://purl.org/dc/elements/1.1/",
    "meta": "urn:oasis:names:tc:opendocument:xmlns:meta:1.0",
    "svg": "urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0",
    "number": "urn:oasis:names:tc:opendocument:xmlns:datastyle:1.0",
    "manifest": "urn:oasis:names:tc:opendocument:xmlns:manifest:1.0",
}
NS_DECL = " ".join(
    f'xmlns:{prefix}="{uri}"' for prefix, uri in NS.items() if prefix != "manifest"
)

BULLET_RE = re.compile(r"^\s*(?:[-*•])\s+(.*)$")
URL_RE = re.compile(r"https?://\S+")
WEEK_RE = re.compile(r"Week\s+\d+", re.IGNORECASE)


def xml_text(value: str) -> str:
    return escape(value, quote=False)


def paragraph(value: str = "", style: str = "Body") -> str:
    return f'<text:p text:style-name="{style}">{xml_text(value)}</text:p>'


def heading(level: int, value: str) -> str:
    return (
        f'<text:h text:style-name="Heading{level}" text:outline-level="{level}">'
        f"{xml_text(value)}</text:h>"
    )


def bullet_list(items: list[str]) -> str:
    if not items:
        return ""
    entries = "".join(
        f"<text:list-item>{paragraph(item)}</text:list-item>" for item in items
    )
    return f'<text:list text:style-name="BulletList">{entries}</text:list>'


def source_line(label: str, value: str) -> str:
    return (
        '<text:p text:style-name="SourceMeta">'
        f'<text:span text:style-name="Strong">{xml_text(label)}: </text:span>'
        f"{xml_text(value)}</text:p>"
    )


@dataclass(frozen=True)
class FrameContent:
    frame: dict[str, Any]
    text_elements: list[dict[str, Any]]


@dataclass(frozen=True)
class CleanedText:
    element: dict[str, Any]
    lines: list[str]
    removed_lines: int


def api_base(url: str) -> str:
    base = url.strip().rstrip("/")
    if not base:
        raise ValueError("ExcaliDash URL cannot be empty")
    return base if base.endswith("/api") else f"{base}/api"


def fetch_drawing(url: str, drawing_id: str, key_file: Path) -> dict[str, Any]:
    try:
        api_key = key_file.read_text(encoding="utf-8").strip()
    except OSError as error:
        raise RuntimeError(
            f"Cannot read ExcaliDash API key file: {key_file}"
        ) from error
    if not api_key:
        raise RuntimeError(f"ExcaliDash API key file is empty: {key_file}")

    endpoint = f"{api_base(url)}/drawings/{urllib.parse.quote(drawing_id, safe='')}"
    request = urllib.request.Request(
        endpoint,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Accept": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            drawing = json.load(response)
    except urllib.error.HTTPError as error:
        detail = error.read(1000).decode("utf-8", errors="replace")
        raise RuntimeError(
            f"ExcaliDash returned HTTP {error.code} for drawing {drawing_id}: {detail}"
        ) from error
    except urllib.error.URLError as error:
        raise RuntimeError(
            f"Cannot reach ExcaliDash at {url}: {error.reason}"
        ) from error

    if not isinstance(drawing, dict) or not isinstance(drawing.get("elements"), list):
        raise TypeError("ExcaliDash returned a drawing without an elements array")
    return drawing


def active_elements(drawing: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        element
        for element in drawing["elements"]
        if isinstance(element, dict) and not element.get("isDeleted", False)
    ]


def element_bounds(element: dict[str, Any]) -> tuple[float, float, float, float]:
    x = float(element.get("x", 0))
    y = float(element.get("y", 0))
    width = abs(float(element.get("width", 0)))
    height = abs(float(element.get("height", 0)))
    return x, y, x + width, y + height


def contains(frame: dict[str, Any], element: dict[str, Any]) -> bool:
    fx1, fy1, fx2, fy2 = element_bounds(frame)
    ex1, ey1, ex2, ey2 = element_bounds(element)
    center_x = (ex1 + ex2) / 2
    center_y = (ey1 + ey2) / 2
    return fx1 <= center_x <= fx2 and fy1 <= center_y <= fy2


def group_by_frames(
    elements: list[dict[str, Any]],
) -> tuple[list[FrameContent], list[dict[str, Any]]]:
    frames = [element for element in elements if element.get("type") == "frame"]
    text_elements = [element for element in elements if element.get("type") == "text"]
    frame_by_id = {frame["id"]: frame for frame in frames}
    grouped: dict[str, list[dict[str, Any]]] = {frame["id"]: [] for frame in frames}
    unframed: list[dict[str, Any]] = []

    for element in text_elements:
        selected = frame_by_id.get(element.get("frameId"))
        if selected is None:
            candidates = [frame for frame in frames if contains(frame, element)]
            if candidates:
                selected = min(
                    candidates, key=lambda frame: frame["width"] * frame["height"]
                )
        if selected is None:
            unframed.append(element)
        else:
            grouped[selected["id"]].append(element)

    preferred_position = {
        name.casefold(): position for position, name in enumerate(DOCUMENT_FRAME_NAMES)
    }
    ordered_frames = sorted(
        frames,
        key=lambda frame: (
            preferred_position.get(
                str(frame.get("name") or "").casefold(), len(preferred_position)
            ),
            frame.get("y", 0),
            frame.get("x", 0),
        ),
    )
    frame_contents = [
        FrameContent(
            frame=frame,
            text_elements=sorted(
                grouped[frame["id"]], key=lambda element: (element["y"], element["x"])
            ),
        )
        for frame in ordered_frames
        if grouped[frame["id"]]
    ]
    return frame_contents, sorted(
        unframed, key=lambda element: (element["y"], element["x"])
    )


def horizontal_line_segment(
    element: dict[str, Any],
) -> tuple[float, float, float] | None:
    if element.get("type") != "line" or abs(float(element.get("angle", 0))) > 0.05:
        return None
    points = element.get("points")
    if not isinstance(points, list) or len(points) < 2:
        return None
    try:
        first_x, first_y = points[0]
        last_x, last_y = points[-1]
        x0 = float(element.get("x", 0))
        y0 = float(element.get("y", 0))
        x1, y1 = x0 + float(first_x), y0 + float(first_y)
        x2, y2 = x0 + float(last_x), y0 + float(last_y)
    except (TypeError, ValueError):
        return None
    if abs(y2 - y1) > max(14.0, abs(x2 - x1) * 0.08):
        return None
    return min(x1, x2), max(x1, x2), (y1 + y2) / 2


def struck_line_indexes(
    text_element: dict[str, Any],
    lines: list[str],
    line_segments: list[tuple[float, float, float]],
) -> set[int]:
    if not lines:
        return set()

    x1, y1, x2, _ = element_bounds(text_element)
    font_size = float(text_element.get("fontSize", 20))
    line_height = font_size * float(text_element.get("lineHeight", 1.25))
    tolerance = max(5.0, line_height * 0.28)
    struck: set[int] = set()

    for line_x1, line_x2, line_y in line_segments:
        overlap = max(0.0, min(x2, line_x2) - max(x1, line_x1))
        segment_width = max(1.0, line_x2 - line_x1)
        if overlap < min(20.0, segment_width * 0.6):
            continue
        index = round((line_y - y1 - line_height / 2) / line_height)
        if 0 <= index < len(lines):
            center_y = y1 + index * line_height + line_height / 2
            if abs(line_y - center_y) <= tolerance:
                struck.add(index)

    # A crossed subsection heading removes its following bullet block as well.
    for index in sorted(struck):
        if lines[index].strip() and BULLET_RE.match(lines[index]) is None:
            cursor = index + 1
            while cursor < len(lines) and lines[cursor].strip():
                if BULLET_RE.match(lines[cursor]) is None:
                    break
                struck.add(cursor)
                cursor += 1
    return struck


def clean_text_elements(
    elements: list[dict[str, Any]],
) -> dict[str, CleanedText]:
    line_segments = [
        segment
        for element in elements
        if (segment := horizontal_line_segment(element)) is not None
    ]
    cleaned: dict[str, CleanedText] = {}
    for element in elements:
        if element.get("type") != "text":
            continue
        raw_lines = str(element.get("text", "")).splitlines()
        removed = struck_line_indexes(element, raw_lines, line_segments)
        lines = [line for index, line in enumerate(raw_lines) if index not in removed]
        cleaned[element["id"]] = CleanedText(element, lines, len(removed))
    return cleaned


def join_wrapped(lines: list[str]) -> str:
    return " ".join(line.strip() for line in lines if line.strip()).strip()


def render_structured_lines(lines: list[str], base_heading_level: int = 3) -> str:
    output: list[str] = []
    paragraph_lines: list[str] = []
    bullets: list[str] = []

    def flush_paragraph() -> None:
        if paragraph_lines:
            output.append(paragraph(join_wrapped(paragraph_lines)))
            paragraph_lines.clear()

    def flush_bullets() -> None:
        if bullets:
            output.append(bullet_list(bullets.copy()))
            bullets.clear()

    index = 0
    while index < len(lines):
        raw = lines[index]
        stripped = raw.strip()
        if not stripped:
            flush_paragraph()
            flush_bullets()
            index += 1
            continue

        bullet = BULLET_RE.match(raw)
        if bullet:
            flush_paragraph()
            item_parts = [bullet.group(1).strip()]
            index += 1
            while index < len(lines):
                continuation = lines[index]
                if not continuation.strip() or BULLET_RE.match(continuation):
                    break
                if continuation.startswith((" ", "\t")):
                    item_parts.append(continuation.strip())
                    index += 1
                    continue
                break
            bullets.append(join_wrapped(item_parts))
            continue

        next_is_bullet = index + 1 < len(lines) and BULLET_RE.match(lines[index + 1])
        previous_blank = index == 0 or not lines[index - 1].strip()
        heading_candidate = (
            len(stripped) <= 80
            and not URL_RE.fullmatch(stripped)
            and (next_is_bullet or previous_blank and stripped.endswith((":", "?")))
        )
        if heading_candidate:
            flush_paragraph()
            flush_bullets()
            output.append(heading(base_heading_level, stripped.rstrip(":")))
        else:
            flush_bullets()
            paragraph_lines.append(stripped)
        index += 1

    flush_paragraph()
    flush_bullets()
    return "".join(output)


def render_note(cleaned: CleanedText, heading_level: int = 3) -> str:
    lines = cleaned.lines.copy()
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    if not lines:
        return ""

    first = lines[0].strip()
    has_more_content = any(line.strip() for line in lines[1:])
    title_like = (
        has_more_content
        and len(first) <= 80
        and not first.startswith(("-", "*", "•"))
        and not URL_RE.fullmatch(first)
    )
    output: list[str] = []
    if title_like:
        output.append(heading(heading_level, first.rstrip(":")))
        lines = lines[1:]
    output.append(render_structured_lines(lines, min(heading_level + 1, 4)))
    return "".join(output)


def render_moscow(notes: list[CleanedText]) -> str:
    priority_order = {"MUST": 0, "SHOULD": 1, "COULD": 2, "WON’T": 3, "WON'T": 3}

    def priority(cleaned: CleanedText) -> tuple[int, float, float]:
        first = next(
            (line.strip().upper() for line in cleaned.lines if line.strip()), ""
        )
        return (
            priority_order.get(first, len(priority_order)),
            cleaned.element.get("y", 0),
            cleaned.element.get("x", 0),
        )

    display_labels = {
        "MUST": "Must",
        "SHOULD": "Should",
        "COULD": "Could",
        "WON’T": "Won’t",
        "WON'T": "Won't",
    }
    output: list[str] = []
    for cleaned in sorted(notes, key=priority):
        lines = cleaned.lines.copy()
        while lines and not lines[0].strip():
            lines.pop(0)
        if not lines:
            continue
        label = lines.pop(0).strip()
        output.append(heading(2, display_labels.get(label.upper(), label)))
        output.append(render_structured_lines(lines, 3))
    return "".join(output)


def render_planning(notes: list[CleanedText]) -> str:
    week_header = next(
        (
            cleaned
            for cleaned in notes
            if len(WEEK_RE.findall(" ".join(cleaned.lines))) >= 4
        ),
        None,
    )
    week_labels = WEEK_RE.findall(" ".join(week_header.lines)) if week_header else []
    candidates = [
        cleaned
        for cleaned in notes
        if cleaned is not week_header and cleaned.element.get("y", 0) < 2500
    ]
    candidates.sort(key=lambda cleaned: cleaned.element.get("x", 0))
    output: list[str] = []
    for index, cleaned in enumerate(candidates):
        label = (
            week_labels[index]
            if index < len(week_labels)
            else f"Planning block {index + 1}"
        )
        output.append(heading(2, label))
        output.append(render_structured_lines(cleaned.lines, 3))

    additional = [
        cleaned
        for cleaned in notes
        if cleaned is not week_header and cleaned not in candidates
    ]
    if additional:
        output.append(heading(2, "Additional planning notes"))
        for cleaned in sorted(additional, key=lambda note: note.element.get("x", 0)):
            output.append(render_note(cleaned, 3))
    return "".join(output)


def is_document_metadata(cleaned: CleanedText) -> bool:
    return any(
        line.strip().casefold().startswith(DOCUMENT_TITLE_PREFIX.casefold())
        for line in cleaned.lines
    )


def frame_body(frame_content: FrameContent, cleaned: dict[str, CleanedText]) -> str:
    notes = [
        cleaned[element["id"]]
        for element in frame_content.text_elements
        if not is_document_metadata(cleaned[element["id"]])
    ]
    frame_name = str(frame_content.frame.get("name") or "Untitled frame")
    if frame_name.casefold() == "moscow":
        return render_moscow(notes)
    if frame_name.casefold() == "project planning":
        return render_planning(notes)
    return "".join(render_note(note, 2) for note in notes)


def document_body(drawing: dict[str, Any]) -> tuple[str, int]:
    elements = active_elements(drawing)
    all_frames, _ = group_by_frames(elements)
    cleaned = clean_text_elements(elements)
    selected_names = {name.casefold() for name in DOCUMENT_FRAME_NAMES}
    frames = [
        content
        for content in all_frames
        if str(content.frame.get("name") or "").casefold() in selected_names
    ]
    frame_names = [
        str(content.frame.get("name") or "Untitled frame") for content in frames
    ]
    found_names = {name.casefold() for name in frame_names}
    missing_frames = [
        name for name in DOCUMENT_FRAME_NAMES if name.casefold() not in found_names
    ]
    if missing_frames:
        raise RuntimeError(
            "ExcaliDash drawing is missing document source frame(s): "
            + ", ".join(missing_frames)
        )
    removed_count = sum(note.removed_lines for note in cleaned.values())
    document_title = str(drawing.get("name", "ExcaliDash drawing"))
    for content in frames:
        for element in content.text_elements:
            for line in cleaned[element["id"]].lines:
                if line.strip().casefold().startswith(DOCUMENT_TITLE_PREFIX.casefold()):
                    document_title = line.split(":", 1)[1].strip() or document_title
                    break

    body: list[str] = [
        paragraph(document_title, "Title"),
        paragraph("Project analysis", "Subtitle"),
        paragraph(
            "Generated directly from the active text in the ExcaliDash drawing. "
            "Struck-through text is excluded.",
            "Lead",
        ),
        paragraph("", "Spacer"),
        source_line("Drawing", str(drawing.get("name", ""))),
        source_line("Drawing ID", str(drawing.get("id", ""))),
        source_line("Drawing version", str(drawing.get("version", ""))),
        source_line("Drawing updated", str(drawing.get("updatedAt", ""))),
        source_line("Source frames", "; ".join(frame_names)),
        source_line("Excluded struck-through lines", str(removed_count)),
        paragraph("", "PageBreak"),
        heading(1, "Contents"),
        bullet_list(frame_names),
    ]

    for frame_content in frames:
        frame_name = str(frame_content.frame.get("name") or "Untitled frame")
        body.append(paragraph("", "PageBreak"))
        body.append(heading(1, frame_name))
        body.append(frame_body(frame_content, cleaned))

    return "".join(body), removed_count


STYLES_XML = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles {NS_DECL} office:version="1.3">
  <office:font-face-decls>
    <style:font-face style:name="Liberation Sans" svg:font-family="'Liberation Sans'" style:font-family-generic="swiss"/>
  </office:font-face-decls>
  <office:styles>
    <style:default-style style:family="paragraph">
      <style:paragraph-properties fo:margin-top="0cm" fo:margin-bottom="0.18cm" fo:line-height="130%"/>
      <style:text-properties style:font-name="Liberation Sans" fo:font-size="10.5pt" fo:color="#000000"/>
    </style:default-style>
    <style:style style:name="Body" style:family="paragraph" style:parent-style-name="Standard"/>
    <style:style style:name="Title" style:family="paragraph">
      <style:paragraph-properties fo:margin-top="4cm" fo:margin-bottom="0.3cm" fo:text-align="center"/>
      <style:text-properties fo:font-size="24pt" fo:font-weight="bold" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Subtitle" style:family="paragraph">
      <style:paragraph-properties fo:margin-bottom="1cm" fo:text-align="center"/>
      <style:text-properties fo:font-size="15pt" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Lead" style:family="paragraph">
      <style:paragraph-properties fo:margin-left="1cm" fo:margin-right="1cm" fo:margin-bottom="0.6cm" fo:text-align="center"/>
      <style:text-properties fo:font-size="11.5pt" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Heading1" style:family="paragraph" style:parent-style-name="Heading">
      <style:paragraph-properties fo:margin-top="0.65cm" fo:margin-bottom="0.25cm" fo:keep-with-next="always" fo:border-bottom="0.02cm solid #000000" fo:padding-bottom="0.08cm"/>
      <style:text-properties fo:font-size="17pt" fo:font-weight="bold" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Heading2" style:family="paragraph" style:parent-style-name="Heading">
      <style:paragraph-properties fo:margin-top="0.45cm" fo:margin-bottom="0.16cm" fo:keep-with-next="always"/>
      <style:text-properties fo:font-size="13pt" fo:font-weight="bold" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Heading3" style:family="paragraph" style:parent-style-name="Heading">
      <style:paragraph-properties fo:margin-top="0.32cm" fo:margin-bottom="0.12cm" fo:keep-with-next="always"/>
      <style:text-properties fo:font-size="11pt" fo:font-weight="bold" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Heading4" style:family="paragraph" style:parent-style-name="Heading">
      <style:paragraph-properties fo:margin-top="0.25cm" fo:margin-bottom="0.1cm" fo:keep-with-next="always"/>
      <style:text-properties fo:font-size="10.5pt" fo:font-weight="bold" fo:color="#000000"/>
    </style:style>
    <style:style style:name="SourceMeta" style:family="paragraph">
      <style:paragraph-properties fo:margin-left="2.5cm" fo:margin-right="2.5cm" fo:margin-bottom="0.15cm"/>
      <style:text-properties fo:font-size="9.5pt" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Note" style:family="paragraph">
      <style:paragraph-properties fo:margin-bottom="0.3cm" fo:padding-left="0.25cm" fo:border-left="0.03cm solid #000000"/>
      <style:text-properties fo:font-size="10pt" fo:font-style="italic" fo:color="#000000"/>
    </style:style>
    <style:style style:name="Spacer" style:family="paragraph"><style:paragraph-properties fo:margin-bottom="0.7cm"/></style:style>
    <style:style style:name="PageBreak" style:family="paragraph"><style:paragraph-properties fo:break-after="page"/></style:style>
    <style:style style:name="Strong" style:family="text"><style:text-properties fo:font-weight="bold" fo:color="#000000"/></style:style>
    <text:list-style style:name="BulletList">
      <text:list-level-style-bullet text:level="1" text:bullet-char="•"><style:list-level-properties text:space-before="0.65cm" text:min-label-width="0.45cm"/></text:list-level-style-bullet>
    </text:list-style>
  </office:styles>
  <office:automatic-styles>
    <style:page-layout style:name="A4Layout">
      <style:page-layout-properties fo:page-width="21cm" fo:page-height="29.7cm" style:print-orientation="portrait" fo:margin-top="1.8cm" fo:margin-bottom="1.8cm" fo:margin-left="2.2cm" fo:margin-right="2.2cm" fo:background-color="#ffffff"/>
      <style:footer-style><style:header-footer-properties fo:min-height="0.6cm" fo:margin-top="0.3cm"/></style:footer-style>
    </style:page-layout>
  </office:automatic-styles>
  <office:master-styles>
    <style:master-page style:name="Standard" style:page-layout-name="A4Layout">
      <style:footer><text:p text:style-name="Body"><text:span>ExcaliDash source · </text:span><text:page-number text:select-page="current">1</text:page-number></text:p></style:footer>
    </style:master-page>
  </office:master-styles>
</office:document-styles>
"""

META_XML = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-meta {NS_DECL} office:version="1.3"><office:meta>
  <meta:generator>ExcaliDash live drawing ODT generator</meta:generator>
  <dc:language>en</dc:language>
</office:meta></office:document-meta>
"""

SETTINGS_XML = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-settings {NS_DECL} office:version="1.3"><office:settings/></office:document-settings>
"""


def content_xml(body: str) -> str:
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-content {NS_DECL} office:version="1.3">
  <office:automatic-styles/>
  <office:body><office:text>{body}</office:text></office:body>
</office:document-content>
"""


def manifest_xml() -> str:
    return f'''<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="{NS["manifest"]}" manifest:version="1.3">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.3" manifest:media-type="application/vnd.oasis.opendocument.text"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="settings.xml" manifest:media-type="text/xml"/>
</manifest:manifest>
'''


def write_odt(path: Path, body: str) -> None:
    files = {
        "content.xml": content_xml(body).encode(),
        "styles.xml": STYLES_XML.encode(),
        "meta.xml": META_XML.encode(),
        "settings.xml": SETTINGS_XML.encode(),
        "META-INF/manifest.xml": manifest_xml().encode(),
    }
    for payload in files.values():
        ET.fromstring(payload)

    path.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(path, "w") as archive:
        archive.writestr(
            "mimetype",
            "application/vnd.oasis.opendocument.text",
            compress_type=ZIP_STORED,
        )
        for name, payload in files.items():
            archive.writestr(name, payload, compress_type=ZIP_DEFLATED)

    with ZipFile(path) as archive:
        if archive.testzip() is not None:
            raise RuntimeError(f"Generated ODT package is corrupt: {path}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate an ODT from the current ExcaliDash drawing text."
    )
    parser.add_argument("--url", default=os.environ.get("EXCALIDASH_URL", DEFAULT_URL))
    parser.add_argument(
        "--drawing-id",
        default=os.environ.get("EXCALIDASH_DRAWING_ID", DEFAULT_DRAWING_ID),
    )
    parser.add_argument(
        "--api-key-file",
        type=Path,
        default=Path(os.environ.get("EXCALIDASH_API_KEY_FILE", DEFAULT_KEY_FILE)),
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    drawing = fetch_drawing(args.url, args.drawing_id, args.api_key_file)
    body, removed_count = document_body(drawing)
    write_odt(args.output, body)
    print(
        f"Generated {args.output} from ExcaliDash drawing version "
        f"{drawing.get('version')} ({removed_count} struck-through lines excluded)"
    )


if __name__ == "__main__":
    main()
