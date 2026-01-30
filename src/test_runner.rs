use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
enum Expectation {
    Output { _line: usize, value: String },
    CompileError { _line: usize, message: String },
    RuntimeError { _line: usize, message: String },
}

#[derive(Debug)]
struct TestCase {
    path: PathBuf,
    expectations: Vec<Expectation>,
}

#[derive(Debug)]
enum TestResult {
    Pass,
    Fail { reason: String },
    Skip { reason: String },
}

struct TestStats {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
}

struct Config {
    verbose: bool,
    show_skipped: bool,
    filter: Option<String>,
}

impl TestCase {
    fn parse(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = fs::File::open(&path)?;
        let reader = BufReader::new(file);
        let mut expectations = Vec::new();

        reader.lines().enumerate().try_for_each(
            |(line_num, line)| -> Result<(), std::io::Error> {
                let line = line?;
                let line_number = line_num + 1;

                if let Some(pos) = line.find("// expect:") {
                    let value = line[pos + 10..].trim().to_string();
                    expectations.push(Expectation::Output {
                        _line: line_number,
                        value,
                    });
                }

                if let Some(pos) = line.find("// expect runtime error:") {
                    let message = line[pos + 24..].trim().to_string();
                    expectations.push(Expectation::RuntimeError {
                        _line: line_number,
                        message,
                    });
                }

                if let Some(pos) = line.find("// Error") {
                    let error_part = &line[pos + 3..];
                    expectations.push(Expectation::CompileError {
                        _line: line_number,
                        message: error_part.to_string(),
                    });
                } else if let Some(pos) = line.find("// [line") {
                    let error_part = &line[pos + 3..];
                    expectations.push(Expectation::CompileError {
                        _line: line_number,
                        message: error_part.to_string(),
                    });
                }

                Ok(())
            },
        )?;

        Ok(TestCase { path, expectations })
    }

    fn run(&self, interpreter: &Path) -> TestResult {
        if self.expectations.is_empty() {
            return TestResult::Skip {
                reason: "No expectations found".to_string(),
            };
        }

        let output = match Command::new(interpreter)
            .arg(&self.path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(out) => out,
            Err(e) => {
                return TestResult::Fail {
                    reason: format!("Failed to execute interpreter: {}", e),
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        let has_compile_error = self
            .expectations
            .iter()
            .any(|e| matches!(e, Expectation::CompileError { .. }));
        let has_runtime_error = self
            .expectations
            .iter()
            .any(|e| matches!(e, Expectation::RuntimeError { .. }));
        let output_expectations: Vec<_> = self
            .expectations
            .iter()
            .filter_map(|e| {
                if let Expectation::Output { value, .. } = e {
                    Some(value.as_str())
                } else {
                    None
                }
            })
            .collect();

        let expected_exit = if has_compile_error {
            65
        } else if has_runtime_error {
            70
        } else {
            0
        };

        if exit_code != expected_exit {
            return TestResult::Fail {
                reason: format!("Expected exit code {} but got {}", expected_exit, exit_code),
            };
        }

        if has_compile_error
            && let Some(Expectation::CompileError { message, .. }) =
                self.expectations.iter().find(|exp| {
                    matches!(exp, Expectation::CompileError { message, .. }
                        if !stderr.contains(message) && !stdout.contains(message))
                })
        {
            return TestResult::Fail {
                reason: format!("Expected compile error '{}' not found", message),
            };
        }

        if has_runtime_error
            && let Some(Expectation::RuntimeError { message, .. }) =
                self.expectations.iter().find(|exp| {
                    matches!(exp, Expectation::RuntimeError { message, .. }
                        if !stderr.contains(message) && !stdout.contains(message))
                })
        {
            return TestResult::Fail {
                reason: format!("Expected runtime error '{}' not found", message),
            };
        }

        if !output_expectations.is_empty() {
            let output_lines: Vec<_> = stdout.lines().collect();

            if output_lines.len() != output_expectations.len() {
                return TestResult::Fail {
                    reason: format!(
                        "Expected {} output lines but got {}",
                        output_expectations.len(),
                        output_lines.len()
                    ),
                };
            }

            if let Some((i, (expected, actual))) = output_expectations
                .iter()
                .zip(output_lines.iter())
                .enumerate()
                .find(|(_, (expected, actual))| expected != actual)
            {
                return TestResult::Fail {
                    reason: format!(
                        "Line {}: expected '{}' but got '{}'",
                        i + 1,
                        expected,
                        actual
                    ),
                };
            }
        }

        TestResult::Pass
    }
}

fn is_scanner_only_test(path: &Path) -> bool {
    let scanner_only_tests = [
        "expressions/evaluate.lox",
        "expressions/parse.lox",
        "scanning/identifiers.lox",
        "scanning/keywords.lox",
        "scanning/numbers.lox",
        "scanning/punctuators.lox",
        "scanning/strings.lox",
        "scanning/whitespace.lox",
    ];

    let path_str = path.to_string_lossy();
    scanner_only_tests
        .iter()
        .any(|test| path_str.contains(test))
}

fn find_tests(test_dir: &Path) -> Vec<PathBuf> {
    let mut tests = Vec::new();

    if let Ok(entries) = fs::read_dir(test_dir) {
        entries.flatten().for_each(|entry| {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) != Some("benchmark") {
                    tests.extend(find_tests(&path));
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("lox")
                && !is_scanner_only_test(&path)
            {
                tests.push(path);
            }
        });
    }

    tests.sort();
    tests
}

fn print_usage() {
    eprintln!("Rust-Native Lox Test Runner");
    eprintln!();
    eprintln!("Usage: test_runner [OPTIONS] <interpreter_path> <test_directory>");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <interpreter_path>  Path to the Lox interpreter executable");
    eprintln!("  <test_directory>    Path to the test directory");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -v, --verbose       Show all passing tests");
    eprintln!("  -s, --show-skipped  Show skipped tests");
    eprintln!("  -f, --filter <text> Only run tests matching filter");
    eprintln!("  -h, --help          Show this help message");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  test_runner rlox test");
    eprintln!("  test_runner -v rlox test");
    eprintln!("  test_runner --filter closure rlox test");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut config = Config {
        verbose: false,
        show_skipped: false,
        filter: None,
    };

    let mut interpreter_path = None;
    let mut test_dir_path = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-v" | "--verbose" => config.verbose = true,
            "-s" | "--show-skipped" => config.show_skipped = true,
            "-f" | "--filter" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --filter requires an argument");
                    std::process::exit(1);
                }
                config.filter = Some(args[i].clone());
            }
            arg => {
                if interpreter_path.is_none() {
                    interpreter_path = Some(arg.to_string());
                } else if test_dir_path.is_none() {
                    test_dir_path = Some(arg.to_string());
                } else {
                    eprintln!("Error: Unexpected argument '{}'", arg);
                    print_usage();
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let interpreter = match interpreter_path {
        Some(path) => Path::new(&path).to_path_buf(),
        None => {
            eprintln!("Error: Missing interpreter path");
            print_usage();
            std::process::exit(1);
        }
    };

    let test_dir = match test_dir_path {
        Some(path) => Path::new(&path).to_path_buf(),
        None => {
            eprintln!("Error: Missing test directory path");
            print_usage();
            std::process::exit(1);
        }
    };

    if !interpreter.exists() {
        eprintln!("Error: Interpreter '{}' not found", interpreter.display());
        std::process::exit(1);
    }

    if !test_dir.exists() {
        eprintln!("Error: Test directory '{}' not found", test_dir.display());
        std::process::exit(1);
    }

    println!("🧪 Lox Test Suite");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Interpreter: {}", interpreter.display());
    println!("Test directory: {}", test_dir.display());
    if let Some(ref filter) = config.filter {
        println!("Filter: {}", filter);
    }
    println!();

    let mut test_files = find_tests(&test_dir);

    if let Some(ref filter) = config.filter {
        test_files.retain(|path| path.to_string_lossy().contains(filter));
    }

    let mut stats = TestStats {
        total: test_files.len(),
        passed: 0,
        failed: 0,
        skipped: 0,
    };

    let mut failures = Vec::new();

    test_files.iter().for_each(|test_file| {
        let test_case = match TestCase::parse(test_file.clone()) {
            Ok(tc) => tc,
            Err(e) => {
                println!("✗ {} - Failed to parse: {}", test_file.display(), e);
                stats.failed += 1;
                return;
            }
        };

        let result = test_case.run(&interpreter);

        match result {
            TestResult::Pass => {
                stats.passed += 1;
                if config.verbose {
                    println!("✓ {}", test_file.display());
                }
            }
            TestResult::Fail { reason } => {
                stats.failed += 1;
                println!("✗ {}", test_file.display());
                println!("  {}", reason);
                failures.push((test_file.display().to_string(), reason));
            }
            TestResult::Skip { reason } => {
                stats.skipped += 1;
                if config.show_skipped {
                    println!("⊘ {} - {}", test_file.display(), reason);
                }
            }
        }
    });

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total tests: {}", stats.total);

    let pass_percent = if stats.total > 0 {
        (stats.passed * 100) / stats.total
    } else {
        0
    };

    println!("✓ Passed: {} ({}%)", stats.passed, pass_percent);
    if stats.failed > 0 {
        println!("✗ Failed: {}", stats.failed);
    }
    if stats.skipped > 0 {
        println!("⊘ Skipped: {}", stats.skipped);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if stats.failed == 0 {
        println!("\n🎉 All tests passed!");
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}
