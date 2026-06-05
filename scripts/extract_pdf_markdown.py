#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from pathlib import Path

import pdfplumber


LIST_BULLET_RE = re.compile(r"^(\d+[\.\)]|[-*+•])\s+")
HEADING_RE = re.compile(
    r"^((\d+(\.\d+)*)\s+.+|[A-Z][A-Z0-9 /'()\-:,]{5,}|#+\s+.+)$"
)


def normalize_line(line: str) -> str:
    line = line.replace("\t", "    ").rstrip()
    line = re.sub(r"\s+", " ", line)
    return line.strip()


def line_indent(chars: list[dict]) -> float:
    xs = [ch["x0"] for ch in chars if ch.get("text", "").strip()]
    return min(xs) if xs else 0.0


def collect_lines(page: pdfplumber.page.Page) -> list[tuple[str, float]]:
    words = page.extract_words(
        x_tolerance=2,
        y_tolerance=3,
        keep_blank_chars=False,
        use_text_flow=True,
    )
    if not words:
        return []

    grouped: list[list[dict]] = []
    current: list[dict] = []
    current_top: float | None = None

    for word in words:
        top = round(word["top"], 1)
        if current_top is None or abs(top - current_top) <= 3:
            current.append(word)
            current_top = top if current_top is None else current_top
        else:
            grouped.append(current)
            current = [word]
            current_top = top
    if current:
        grouped.append(current)

    lines: list[tuple[str, float]] = []
    for group in grouped:
        group.sort(key=lambda w: (w["x0"], w["top"]))
        text = " ".join(word["text"] for word in group)
        text = normalize_line(text)
        if text:
            lines.append((text, line_indent(group)))
    return lines


def classify_indent(indent: float, base: float) -> int:
    delta = max(0.0, indent - base)
    if delta < 12:
        return 0
    if delta < 36:
        return 1
    return 2


def to_markdown(lines: list[tuple[str, float]], source_name: str) -> str:
    if not lines:
        return f"# {source_name}\n"

    non_empty_indents = [indent for text, indent in lines if text]
    base_indent = min(non_empty_indents) if non_empty_indents else 0.0

    out: list[str] = [f"# {source_name}", ""]
    prev_was_text = False

    for raw_text, indent in lines:
        text = raw_text.strip()
        if not text:
            if out and out[-1] != "":
                out.append("")
            prev_was_text = False
            continue

        level = classify_indent(indent, base_indent)

        if HEADING_RE.match(text) and len(text) <= 120:
            if out and out[-1] != "":
                out.append("")
            out.append(f"## {text}")
            out.append("")
            prev_was_text = False
            continue

        if LIST_BULLET_RE.match(text):
            if out and out[-1] == "":
                pass
            prefix = "  " * level
            out.append(f"{prefix}- {LIST_BULLET_RE.sub('', text, count=1)}")
            prev_was_text = False
            continue

        if prev_was_text:
            out[-1] = f"{out[-1]} {text}"
        else:
            if level > 0:
                prefix = "  " * level
                out.append(f"{prefix}{text}")
            else:
                out.append(text)
        prev_was_text = True

    compacted: list[str] = []
    for line in out:
        if line == "" and compacted and compacted[-1] == "":
            continue
        compacted.append(line)

    return "\n".join(compacted).strip() + "\n"


def extract_pdf(pdf_path: Path) -> str:
    per_page: list[tuple[str, float]] = []
    with pdfplumber.open(pdf_path) as pdf:
        for page_number, page in enumerate(pdf.pages, start=1):
            page_lines = collect_lines(page)
            if page_number > 1 and per_page and page_lines:
                per_page.append(("", 0.0))
            per_page.extend(page_lines)
    return to_markdown(per_page, pdf_path.stem)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("pdfs", nargs="+", type=Path)
    parser.add_argument("--out-dir", required=True, type=Path)
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)

    for pdf_path in args.pdfs:
        markdown = extract_pdf(pdf_path)
        output_path = args.out_dir / f"{pdf_path.stem}.md"
        output_path.write_text(markdown, encoding="utf-8")
        print(f"wrote {output_path}")


if __name__ == "__main__":
    main()
