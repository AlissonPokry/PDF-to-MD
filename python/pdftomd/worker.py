from __future__ import annotations

import argparse
import io
import re
from dataclasses import dataclass
from pathlib import Path
from statistics import median
from typing import Iterable

import fitz


PAGE_NUMBER_RE = re.compile(r"^\d{1,4}$")
SPACE_RE = re.compile(r"\s+")


@dataclass(frozen=True)
class Span:
    text: str
    x0: float
    y0: float
    x1: float
    y1: float
    size: float
    flags: int = 0

    @property
    def is_bold(self) -> bool:
        return bool(self.flags & 16)


@dataclass(frozen=True)
class Line:
    text: str
    x0: float
    y0: float
    x1: float
    y1: float
    size: float
    bold: bool = False


@dataclass(frozen=True)
class Page:
    number: int
    width: float
    height: float
    lines: tuple[Line, ...]


def normalize_text(text: str) -> str:
    return SPACE_RE.sub(" ", text.replace("\u00ad", "")).strip()


def is_page_number(line: Line, page_height: float) -> bool:
    return PAGE_NUMBER_RE.match(line.text) is not None and line.y0 > page_height * 0.9


def line_sort_key(line: Line, page_width: float) -> tuple[int, float, float]:
    column = 0 if line.x0 < page_width * 0.48 else 1
    return (column, line.y0, line.x0)


def spans_to_lines(spans: Iterable[Span], page_width: float) -> tuple[Line, ...]:
    grouped: dict[tuple[int, int], list[Span]] = {}
    for span in spans:
        text = normalize_text(span.text)
        if not text:
            continue
        key = (round(span.y0), round(span.size))
        grouped.setdefault(key, []).append(span)

    lines: list[Line] = []
    column_boundary = page_width * 0.48
    for values in grouped.values():
        clusters: list[list[Span]] = []
        for span in sorted(values, key=lambda item: item.x0):
            if not clusters:
                clusters.append([span])
                continue

            previous = clusters[-1][-1]
            crosses_column = previous.x1 < column_boundary <= span.x0
            large_gap = span.x0 >= column_boundary and span.x0 - previous.x1 > page_width * 0.025
            if crosses_column or large_gap:
                clusters.append([span])
            else:
                clusters[-1].append(span)

        for ordered in clusters:
            text = normalize_text(" ".join(span.text for span in ordered))
            if not text:
                continue
            lines.append(
                Line(
                    text=text,
                    x0=min(span.x0 for span in ordered),
                    y0=min(span.y0 for span in ordered),
                    x1=max(span.x1 for span in ordered),
                    y1=max(span.y1 for span in ordered),
                    size=median(span.size for span in ordered),
                    bold=any(span.is_bold for span in ordered),
                )
            )
    return tuple(lines)


def extract_page(doc: fitz.Document, index: int, use_ocr: bool) -> Page:
    pdf_page = doc[index]
    page_dict = pdf_page.get_text("dict", flags=fitz.TEXTFLAGS_TEXT)
    spans: list[Span] = []
    for block in page_dict.get("blocks", []):
        for line in block.get("lines", []):
            for span in line.get("spans", []):
                x0, y0, x1, y1 = span["bbox"]
                spans.append(
                    Span(
                        text=span.get("text", ""),
                        x0=x0,
                        y0=y0,
                        x1=x1,
                        y1=y1,
                        size=float(span.get("size", 0)),
                        flags=int(span.get("flags", 0)),
                    )
                )

    lines = spans_to_lines(spans, pdf_page.rect.width)
    if use_ocr and sum(len(line.text) for line in lines) < 20:
        lines = tuple(
            Line(text=text, x0=0, y0=i * 12, x1=pdf_page.rect.width, y1=i * 12 + 10, size=11)
            for i, text in enumerate(ocr_page(pdf_page))
            if text
        )

    clean_lines = tuple(
        line
        for line in lines
        if not is_page_number(line, pdf_page.rect.height)
    )
    ordered = tuple(sorted(clean_lines, key=lambda line: line_sort_key(line, pdf_page.rect.width)))
    return Page(index + 1, pdf_page.rect.width, pdf_page.rect.height, ordered)


def ocr_page(page: fitz.Page) -> list[str]:
    try:
        import pytesseract
        from PIL import Image
    except ImportError as exc:
        raise RuntimeError("OCR requires pillow, pytesseract, and system Tesseract") from exc

    pixmap = page.get_pixmap(matrix=fitz.Matrix(2, 2), alpha=False)
    image = Image.open(io.BytesIO(pixmap.tobytes("png")))
    text = pytesseract.image_to_string(image, lang="por+eng")
    return [normalize_text(line) for line in text.splitlines()]


def detect_heading_level(line: Line, body_size: float) -> int | None:
    text = line.text.strip()
    if len(text) < 2:
        return None
    if line.size >= body_size * 2.4:
        return 1
    if line.size >= body_size * 1.45:
        return 2
    if line.size >= body_size * 1.25 or (line.bold and text.isupper()):
        return 3
    return None


def merge_paragraph_lines(lines: Iterable[str]) -> str:
    paragraph = ""
    for raw in lines:
        line = normalize_text(raw)
        if not line:
            continue
        if not paragraph:
            paragraph = line
        elif paragraph.endswith("-") and not paragraph.endswith(" -"):
            paragraph = paragraph[:-1] + line
        else:
            paragraph += " " + line
    return paragraph


def render_markdown(pages: Iterable[Page], keep_page_breaks: bool = False) -> str:
    all_lines = [line for page in pages for line in page.lines]
    body_candidates = [line.size for line in all_lines if 7 <= line.size <= 14]
    body_size = median(body_candidates) if body_candidates else 11

    output: list[str] = []
    paragraph: list[str] = []
    last_body_line: Line | None = None

    def flush_paragraph() -> None:
        if not paragraph:
            return
        merged = merge_paragraph_lines(paragraph)
        if merged:
            output.extend([merged, ""])
        paragraph.clear()

    for page in pages:
        if keep_page_breaks:
            flush_paragraph()
            last_body_line = None
            output.extend([f"<!-- page {page.number} -->", ""])

        for line in page.lines:
            level = detect_heading_level(line, body_size)
            if level:
                flush_paragraph()
                last_body_line = None
                marker = "#" * level
                output.extend([f"{marker} {normalize_text(line.text)}", ""])
                continue

            if line.text.startswith(("\u2022", "-", "o ")):
                flush_paragraph()
                last_body_line = None
                output.append("- " + normalize_text(line.text.lstrip("\u2022-o ")))
                continue

            if last_body_line is not None:
                previous_column = 0 if last_body_line.x0 < page.width * 0.48 else 1
                current_column = 0 if line.x0 < page.width * 0.48 else 1
                vertical_gap = line.y0 - last_body_line.y0
                if previous_column != current_column or vertical_gap > max(body_size * 1.75, 18):
                    flush_paragraph()

            paragraph.append(line.text)
            last_body_line = line

    flush_paragraph()
    return "\n".join(output).strip() + "\n"


def convert_pdf(input_pdf: Path, output_md: Path, use_ocr: bool, keep_page_breaks: bool) -> None:
    with fitz.open(input_pdf) as doc:
        pages = [extract_page(doc, index, use_ocr) for index in range(doc.page_count)]
    markdown = render_markdown(pages, keep_page_breaks=keep_page_breaks)
    output_md.write_text(markdown, encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Convert a PDF into Markdown.")
    parser.add_argument("input", type=Path)
    parser.add_argument("--output", "-o", required=True, type=Path)
    parser.add_argument("--ocr", action="store_true")
    parser.add_argument("--keep-page-breaks", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    convert_pdf(args.input, args.output, args.ocr, args.keep_page_breaks)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
