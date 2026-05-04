use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type Result<T> = std::result::Result<T, String>;

const DEFAULT_INPUT_DIR: &str = "PUT_FILE_HERE";
const DEFAULT_OUTPUT_DIR: &str = "Markdown files";

#[derive(Debug, PartialEq, Eq)]
struct Cli {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    python: OsString,
    ocr: bool,
    keep_page_breaks: bool,
}

fn main() {
    let cli = match parse_cli(std::env::args_os().skip(1)) {
        Ok(cli) => cli,
        Err(err) => {
            eprintln!("{err}");
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };

    if let Err(err) = run(cli) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match (&cli.input, &cli.output) {
        (Some(input), Some(output)) => run_one(input, output, &cli),
        (None, None) => run_folder_mode(&cli),
        (Some(_), None) => Err("--output is required".to_string()),
        (None, Some(_)) => Err("input PDF is required when --output is set".to_string()),
    }
}

fn run_one(input: &Path, output: &Path, cli: &Cli) -> Result<()> {
    validate_input(input)?;

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create output directory {}: {err}", parent.display()))?;
        }
    }

    let worker = worker_path()?;
    convert_with_worker(&cli.python, &worker, input, output, cli.ocr, cli.keep_page_breaks)
}

fn convert_with_worker(
    python: &OsString,
    worker: &Path,
    input: &Path,
    output: &Path,
    ocr: bool,
    keep_page_breaks: bool,
) -> Result<()> {
    let args = worker_args(worker, input, output, ocr, keep_page_breaks);

    let status = Command::new(python)
        .args(args)
        .stdin(Stdio::null())
        .status()
        .map_err(|err| format!("failed to start Python executable {:?}: {err}", python))?;

    if !status.success() {
        return Err(format!("Python worker failed with status {status}"));
    }

    Ok(())
}

fn run_folder_mode(cli: &Cli) -> Result<()> {
    let input_dir = Path::new(DEFAULT_INPUT_DIR);
    let files = scan_pdf_files(input_dir)?;
    if files.is_empty() {
        return Err(format!("no PDF files found in {}", input_dir.display()));
    }

    println!("PDF files:");
    for (index, file) in files.iter().enumerate() {
        println!("{}: {}", index + 1, file.file_name().unwrap_or_default().to_string_lossy());
    }
    print!("Select files (example: 1 3 5, or all): ");
    io::stdout()
        .flush()
        .map_err(|err| format!("failed to flush prompt: {err}"))?;

    let mut selection = String::new();
    io::stdin()
        .read_line(&mut selection)
        .map_err(|err| format!("failed to read selection: {err}"))?;

    let selected = parse_selection(&selection, files.len())?;
    let output_dir = input_dir.join(DEFAULT_OUTPUT_DIR);
    std::fs::create_dir_all(&output_dir)
        .map_err(|err| format!("failed to create output directory {}: {err}", output_dir.display()))?;

    let worker = worker_path()?;
    for index in selected {
        let input = &files[index];
        let stem = input
            .file_stem()
            .ok_or_else(|| format!("failed to read file stem for {}", input.display()))?
            .to_string_lossy();
        let output = collision_free_output_path(&output_dir, &stem)?;
        println!("Converting {} -> {}", input.display(), output.display());
        convert_with_worker(
            &cli.python,
            &worker,
            input,
            &output,
            cli.ocr,
            cli.keep_page_breaks,
        )?;
    }

    Ok(())
}

fn validate_input(input: &Path) -> Result<()> {
    if !input.exists() {
        return Err(format!("input PDF does not exist: {}", input.display()));
    }
    if !input
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        return Err(format!("input must be a .pdf file: {}", input.display()));
    }
    Ok(())
}

fn worker_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("failed to resolve current executable: {err}"))?;
    let root = exe
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(root.join("python").join("pdftomd").join("worker.py"))
}

fn scan_pdf_files(input_dir: &Path) -> Result<Vec<PathBuf>> {
    let entries = std::fs::read_dir(input_dir)
        .map_err(|err| format!("failed to read {}: {err}", input_dir.display()))?;
    let mut files = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read directory entry: {err}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_pdf = path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
        if is_pdf {
            files.push(path);
        }
    }

    files.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));
    Ok(files)
}

fn parse_selection(input: &str, file_count: usize) -> Result<Vec<usize>> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("all") {
        return Ok((0..file_count).collect());
    }
    if trimmed.is_empty() {
        return Err("selection is required".to_string());
    }

    let mut selected = Vec::new();
    for token in trimmed.split_whitespace() {
        let number = token
            .parse::<usize>()
            .map_err(|_| format!("invalid selection: {token}"))?;
        if number == 0 || number > file_count {
            return Err(format!("selection {number} is out of range"));
        }
        let index = number - 1;
        if !selected.contains(&index) {
            selected.push(index);
        }
    }
    Ok(selected)
}

fn collision_free_output_path(output_dir: &Path, stem: &str) -> Result<PathBuf> {
    let first = output_dir.join(format!("{stem}.md"));
    if !first.exists() {
        return Ok(first);
    }

    for counter in 1.. {
        let candidate = output_dir.join(format!("{stem} ({counter}).md"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    unreachable!("unbounded counter always returns before overflow")
}

fn parse_cli(args: impl IntoIterator<Item = OsString>) -> Result<Cli> {
    let mut input = None;
    let mut output = None;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut python = default_python_for(&cwd);
    let mut ocr = false;
    let mut keep_page_breaks = false;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "-h" | "--help" => return Err(String::new()),
            "-o" | "--output" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--output requires a path".to_string())?;
                output = Some(PathBuf::from(value));
            }
            "--python" => {
                python = iter
                    .next()
                    .ok_or_else(|| "--python requires an executable path".to_string())?;
            }
            "--ocr" => ocr = true,
            "--keep-page-breaks" => keep_page_breaks = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            _ => {
                if input.is_some() {
                    return Err("only one input PDF is supported".to_string());
                }
                input = Some(PathBuf::from(arg));
            }
        }
    }

    if input.is_some() && output.is_none() {
        return Err("--output is required".to_string());
    }
    if input.is_none() && output.is_some() {
        return Err("input PDF is required when --output is set".to_string());
    }

    Ok(Cli {
        input,
        output,
        python,
        ocr,
        keep_page_breaks,
    })
}

fn usage() -> &'static str {
    "Usage: pdftomd [<input.pdf> --output <output.md>] [--python <python.exe>] [--ocr] [--keep-page-breaks]\nNo input scans PUT_FILE_HERE and writes to PUT_FILE_HERE/Markdown files."
}

fn default_python_for(root: &Path) -> OsString {
    let windows_venv = root.join(".venv").join("Scripts").join("python.exe");
    if windows_venv.exists() {
        return windows_venv.into_os_string();
    }

    let unix_venv = root.join(".venv").join("bin").join("python");
    if unix_venv.exists() {
        return unix_venv.into_os_string();
    }

    OsString::from("python")
}

fn worker_args(
    worker: &Path,
    input: &Path,
    output: &Path,
    ocr: bool,
    keep_page_breaks: bool,
) -> Vec<OsString> {
    let mut args = vec![
        worker.as_os_str().to_os_string(),
        input.as_os_str().to_os_string(),
        OsString::from("--output"),
        output.as_os_str().to_os_string(),
    ];

    if ocr {
        args.push(OsString::from("--ocr"));
    }
    if keep_page_breaks {
        args.push(OsString::from("--keep-page-breaks"));
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_args_include_required_paths() {
        let args = worker_args(
            Path::new("python/pdftomd/worker.py"),
            Path::new("book.pdf"),
            Path::new("book.md"),
            false,
            false,
        );

        assert_eq!(
            args,
            vec![
                OsString::from("python/pdftomd/worker.py"),
                OsString::from("book.pdf"),
                OsString::from("--output"),
                OsString::from("book.md"),
            ]
        );
    }

    #[test]
    fn worker_args_include_optional_flags() {
        let args = worker_args(
            Path::new("worker.py"),
            Path::new("book.pdf"),
            Path::new("book.md"),
            true,
            true,
        );

        assert!(args.contains(&OsString::from("--ocr")));
        assert!(args.contains(&OsString::from("--keep-page-breaks")));
    }

    #[test]
    fn validate_input_rejects_missing_file() {
        let err = validate_input(Path::new("missing.pdf")).unwrap_err().to_string();

        assert!(err.contains("does not exist"));
    }

    #[test]
    fn parse_cli_reads_required_args_and_flags() {
        let cli = parse_cli([
            OsString::from("book.pdf"),
            OsString::from("--output"),
            OsString::from("book.md"),
            OsString::from("--python"),
            OsString::from(".venv/Scripts/python.exe"),
            OsString::from("--ocr"),
        ])
        .unwrap();

        assert_eq!(
            cli,
            Cli {
                input: Some(PathBuf::from("book.pdf")),
                output: Some(PathBuf::from("book.md")),
                python: OsString::from(".venv/Scripts/python.exe"),
                ocr: true,
                keep_page_breaks: false,
            }
        );
    }

    #[test]
    fn parse_cli_requires_output() {
        let err = parse_cli([OsString::from("book.pdf")]).unwrap_err();

        assert_eq!(err, "--output is required");
    }

    #[test]
    fn parse_cli_without_input_uses_folder_mode() {
        let cli = parse_cli([]).unwrap();

        assert_eq!(cli.input, None);
        assert_eq!(cli.output, None);
    }

    #[test]
    fn default_python_uses_local_venv_when_available() {
        let dir = std::env::temp_dir().join(format!("pdftomd-python-test-{}", std::process::id()));
        let scripts = dir.join(".venv").join("Scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        std::fs::write(scripts.join("python.exe"), "").unwrap();

        let python = default_python_for(&dir);

        assert_eq!(python, OsString::from(dir.join(".venv").join("Scripts").join("python.exe")));
        std::fs::remove_file(scripts.join("python.exe")).unwrap();
        std::fs::remove_dir(scripts).unwrap();
        std::fs::remove_dir(dir.join(".venv")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn parse_selection_accepts_space_separated_indexes() {
        let selected = parse_selection("1 3 5", 5).unwrap();

        assert_eq!(selected, vec![0, 2, 4]);
    }

    #[test]
    fn parse_selection_accepts_all() {
        let selected = parse_selection("all", 3).unwrap();

        assert_eq!(selected, vec![0, 1, 2]);
    }

    #[test]
    fn parse_selection_rejects_out_of_range_index() {
        let err = parse_selection("4", 3).unwrap_err();

        assert_eq!(err, "selection 4 is out of range");
    }

    #[test]
    fn collision_path_adds_number_before_extension() {
        let dir = std::env::temp_dir().join(format!("pdftomd-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = dir.join("File.md");
        let second = dir.join("File (1).md");
        std::fs::write(&first, "").unwrap();
        std::fs::write(&second, "").unwrap();

        let next = collision_free_output_path(&dir, "File").unwrap();

        assert_eq!(next.file_name().unwrap(), "File (2).md");
        std::fs::remove_file(first).unwrap();
        std::fs::remove_file(second).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn scan_pdf_files_lists_all_top_level_pdfs_and_ignores_output_dir() {
        let dir = std::env::temp_dir().join(format!("pdftomd-scan-test-{}", std::process::id()));
        let output_dir = dir.join(DEFAULT_OUTPUT_DIR);
        std::fs::create_dir_all(&output_dir).unwrap();
        for index in 1..=12 {
            std::fs::write(dir.join(format!("Book {index}.pdf")), "").unwrap();
        }
        std::fs::write(dir.join("notes.txt"), "").unwrap();
        std::fs::write(output_dir.join("Converted.pdf"), "").unwrap();

        let files = scan_pdf_files(&dir).unwrap();

        assert_eq!(files.len(), 12);
        assert!(files.iter().all(|path| path.parent() == Some(dir.as_path())));

        for file in files {
            std::fs::remove_file(file).unwrap();
        }
        std::fs::remove_file(dir.join("notes.txt")).unwrap();
        std::fs::remove_file(output_dir.join("Converted.pdf")).unwrap();
        std::fs::remove_dir(output_dir).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
