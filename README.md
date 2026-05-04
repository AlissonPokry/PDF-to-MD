# PDF to Markdown

Command line converter for PDFs. Rust owns the CLI and file orchestration; Python owns PDF extraction, layout ordering, heading detection, Markdown rendering, and optional OCR.

## Requirements

- Rust toolchain with `cargo`.
- Python 3.11+.
- Python packages in `requirements.txt`.
- Optional OCR: system Tesseract with Portuguese and English language data.

## Setup

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install -r requirements.txt
cargo build
```

## Usage

Folder mode scans `PUT_FILE_HERE` for all top-level PDF files, asks which files to convert, and writes Markdown to `PUT_FILE_HERE/Markdown files`.

```powershell
cargo run --
```

The CLI uses `.\.venv\Scripts\python.exe` automatically when it exists.

At the prompt, type one number, several numbers separated by spaces, or `all`:

```text
Select files (example: 1 3 5, or all): 1 3 5
```

If output name already exists, the converter writes `File (1).md`, `File (2).md`, and so on.

Direct mode still works:

```powershell
cargo run -- fileName.pdf --output fileName.md
```

Enable OCR fallback for pages with little embedded text:

```powershell
cargo run -- fileName.pdf --output fileName.md --ocr
```

## Tests

```powershell
pytest -q
cargo test
```
