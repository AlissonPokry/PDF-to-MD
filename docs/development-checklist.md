# TTRPG PDF to Markdown Converter Plan

## PDF Analysis

- [x] Read `prompt.md` requirements.
- [x] Inspect repository shape: workspace contains `prompt.md` and `Arquivos-Secretos-01.pdf`; no existing project files.
- [x] Inspect PDF metadata: PDF 1.4, 74 pages, not encrypted, generated through Ghostscript/pdfwrite.
- [x] Sample text layer with PDF.js: most content has extractable text; page 1 has no text; PDF includes many images, so OCR fallback is needed for pages or image regions with missing text.
- [x] Identify document structure from table of contents and headings:
  - Front matter: pages 1-7.
  - A Agatha: starts page 8.
  - Os Transtornados: starts page 20.
  - Ritos & Maldicoes: starts page 42.
  - Conteudos Bonus: starts page 60.
  - Mural dos Agentes: starts page 72.
- [x] Identify layout needs: two-column article pages, large display headings, smaller subsection headings, page numbers at bottom, decorative text/images to ignore except OCR text.

## Architecture

- [x] Use Rust for the command line interface, argument validation, process orchestration, and file output.
- [x] Use Python for PDF text extraction and layout analysis because PyMuPDF and OCR libraries are mature.
- [x] Keep OCR optional and explicit: default extracts embedded text; `--ocr` enables Tesseract fallback for pages with little or no text.
- [x] Write Markdown as one file, preserving title/section/subsection hierarchy, paragraphs, lists, and page-break comments only when requested.

## Implementation Checklist

- [x] Create project scaffolding.
- [x] Add Python unit tests for text normalization, heading detection, column ordering, and Markdown rendering.
- [x] Add Rust unit tests for CLI argument construction.
- [x] Implement Python extraction worker.
- [x] Implement Rust CLI wrapper.
- [x] Add dependency manifests and usage docs.
- [x] Run Python tests.
- [x] Run Rust tests.
- [x] Run converter against `Arquivos-Secretos-01.pdf`.
- [x] Inspect generated Markdown sample for heading/paragraph quality.
- [x] Add folder-mode CLI that scans `PUT_FILE_HERE`, lists all top-level PDFs, accepts space-separated selections or `all`, and writes to `PUT_FILE_HERE/Markdown files`.
- [x] Add collision-safe output names like `File (1).md`.
- [x] Keep direct single-file mode available.

## Verification Notes

- Python 3.13.13 was installed temporarily at `C:\tmp\pdftomd-python`.
- Rust/Cargo 1.95.0 was installed through rustup. Commands use explicit `RUSTUP_HOME` and `CARGO_HOME` process variables because Cargo is not on PATH.
- Python dependencies were installed into local `.venv`.
- PDF analysis used a temporary PDF.js install in `C:\tmp` only for inspection.
- Python tests: `.\.venv\Scripts\python.exe -m pytest -q` -> 9 passed, 1 pytest cache warning.
- Rust tests: `cargo test` -> 5 passed.
- Converter run: `cargo run -- Arquivos-Secretos-01.pdf --output Arquivos-Secretos-01.md --python .\.venv\Scripts\python.exe --keep-page-breaks` -> exit 0.
- Generated Markdown: `Arquivos-Secretos-01.md`, 131,743 bytes.
- Sample inspection: page 9 narrative paragraphs split cleanly; page 40 headings and two-column body text render as Markdown sections/paragraphs.
- OCR path is implemented but not verified because no system `tesseract` executable is installed.
- Folder-mode tests cover no-arg folder mode, space-separated selections, `all`, out-of-range input, unbounded top-level PDF scanning, output-folder exclusion, and collision-safe Markdown names.
