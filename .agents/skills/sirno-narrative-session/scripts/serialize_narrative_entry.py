#!/usr/bin/env python3
"""Serialize a Sirno narrative session summary into a Markdown entry."""

from __future__ import annotations

import argparse
import json
import re
import sys
import textwrap
from pathlib import Path
from typing import Any


ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
FIELD_RE = re.compile(r"^[A-Za-z][A-Za-z0-9_-]*$")
WRAP_WIDTH = 96
RESERVED_FIELDS = {"name", "desc", "frozen"}


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def scalar(value: str) -> str:
    if "\n" in value:
        fail("frontmatter scalar values must not contain newlines")
    if re.match(r"^[A-Za-z0-9][A-Za-z0-9 .,'`/-]*$", value):
        return value
    return json.dumps(value, ensure_ascii=True)


def string_list_value(value: Any, key: str) -> list[str]:
    if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
        fail(f"{key} must be a list of strings")
    for item in value:
        if not ID_RE.match(item):
            fail(f"{key} values must be lowercase kebab-case ids")
    return value


def structural_fields(data: dict[str, Any]) -> dict[str, list[str]]:
    value = data.get("structural", {})
    if value is None:
        return {}
    if not isinstance(value, dict):
        fail("structural must be an object from field names to entry-id lists")

    fields: dict[str, list[str]] = {}
    for field, targets in value.items():
        if not isinstance(field, str) or not FIELD_RE.match(field):
            fail("structural field names must be ASCII identifiers")
        if field in RESERVED_FIELDS:
            fail(f"{field} is reserved metadata, not a structural field")
        fields[field] = string_list_value(targets, f"structural.{field}")
    return fields


def required_string(data: dict[str, Any], key: str) -> str:
    value = data.get(key)
    if not isinstance(value, str) or not value.strip():
        fail(f"{key} must be a non-empty string")
    return value.strip()


def wrap_paragraph(paragraph: str) -> str:
    paragraph = paragraph.strip()
    if not paragraph:
        return ""
    preserved = ("-", "1.", ">", "#", "```")
    if paragraph.startswith(preserved):
        return paragraph
    return textwrap.fill(
        paragraph,
        width=WRAP_WIDTH,
        break_long_words=False,
        break_on_hyphens=False,
    )


def render_list_field(name: str, values: list[str]) -> list[str]:
    if not values:
        return []
    lines = [f"{name}:"]
    lines.extend(f"  - {item}" for item in values)
    return lines


def render_entry(data: dict[str, Any]) -> str:
    entry_id = required_string(data, "id")
    if not ID_RE.match(entry_id):
        fail("id must be lowercase kebab-case")

    name = required_string(data, "name")
    desc = required_string(data, "desc")
    body = data.get("body")
    if not isinstance(body, list) or not all(isinstance(item, str) for item in body):
        fail("body must be a list of paragraph strings")

    lines = [
        "---",
        f"name: {scalar(name)}",
        f"desc: {scalar(desc)}",
    ]
    for field, targets in structural_fields(data).items():
        lines.extend(render_list_field(field, targets))
    lines.extend(["---", ""])

    paragraphs = [wrap_paragraph(item) for item in body if item.strip()]
    lines.append("\n\n".join(paragraphs))
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Serialize a Sirno narrative session summary into a lake entry."
    )
    parser.add_argument("--lake", default="sirno-docs", help="Sirno Lake directory")
    parser.add_argument("--input", required=True, help="JSON session summary, or - for stdin")
    parser.add_argument("--force", action="store_true", help="overwrite an existing entry")
    parser.add_argument("--dry-run", action="store_true", help="print the entry without writing")
    args = parser.parse_args()

    raw_input = sys.stdin.read() if args.input == "-" else Path(args.input).read_text(encoding="utf-8")
    data = json.loads(raw_input)
    if not isinstance(data, dict):
        fail("input JSON must be an object")

    output = render_entry(data)
    entry_id = required_string(data, "id")
    output_path = Path(args.lake) / f"{entry_id}.md"

    if args.dry_run:
        print(output, end="")
        return

    if output_path.exists() and not args.force:
        fail(f"{output_path} already exists; pass --force to overwrite")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(output, encoding="utf-8")
    print(output_path)


if __name__ == "__main__":
    main()
