# Overview

A Rust implementation of the Lox bytecode virtual machine from "Crafting Interpreters" by Robert Nystrom.

## Building and Running

```bash
# Build the project
cargo build

# Run the REPL
cargo run --bin rlox

# Execute a Lox file
cargo run --bin rlox <file.lox>

# Build optimized release version
cargo build --release
```

## Enable Debug Tracing in rlox

Build with:

```bash
cargo build --features debug_trace
# Or
cargo build --release --features debug_trace
```

### Performance Considerations

The current implementation prioritizes correctness and clarity over performance.

## Rust-Native Test Runner

A comprehensive test runner for the Lox programming language test suite, written in pure Rust with zero dependencies.

### Building

```bash
cargo build --release --bin test_runner
```

### Usage

#### Basic Usage

```bash
cargo run --release --bin test_runner -- <interpreter_path> <test_directory>
```

Example:

```bash
cargo run --release --bin test_runner -- target/release/rlox ./test
```

#### Command-Line Options

```
-v, --verbose       Show all passing tests
-s, --show-skipped  Show skipped tests
-f, --filter <text> Only run tests matching filter
-h, --help          Show help message
```

#### Examples

Run all tests:

```bash
cargo run --release --bin test_runner -- target/release/rlox ./test
```

Run all tests verbosely:

```bash
cargo run --release --bin test_runner -- -v target/release/rlox ./test
```

Run only closure tests:

```bash
cargo run --release --bin test_runner -- --filter closure target/release/rlox ./test
```

Run class tests with verbose output:

```bash
cargo run --release --bin test_runner -- -v --filter class target/release/rlox ./test
```

Show skipped tests:

```bash
cargo run --release --bin test_runner -- -s target/release/rlox ./test
```

### Test Format

The test runner understands the Crafting Interpreters test format:

#### Expected Output

```lox
print "hello";  // expect: hello
```

#### Compile Errors

```lox
var 1 = "bad";  // Error at '1': Expect variable name.
// OR
var 1 = "bad";  // [line 1] Error at '1': Expect variable name.
```

#### Runtime Errors

```lox
print undefined;  // expect runtime error: Undefined variable 'undefined'.
```

### Test Results

Example output:

```
🧪 Lox Test Suite
━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Interpreter: target/release/rlox
Test directory: ./test

━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total tests: 246
✓ Passed: 243 (98%)
⊘ Skipped: 3
━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🎉 All tests passed!
```

**Note:** 8 jlox scanner-only tests are automatically excluded (not counted in total).

### Automatically Excluded Tests

The test runner automatically excludes 8 jlox-specific tests that are not applicable to rlox:

**Scanner-only tests (jlox chapters 4-6):**

- `test/expressions/evaluate.lox` - Scanner-only test
- `test/expressions/parse.lox` - Parser-only test
- `test/scanning/identifiers.lox` - Scanner-only test
- `test/scanning/keywords.lox` - Scanner-only test
- `test/scanning/numbers.lox` - Scanner-only test
- `test/scanning/punctuators.lox` - Scanner-only test
- `test/scanning/strings.lox` - Scanner-only test
- `test/scanning/whitespace.lox` - Scanner-only test

These tests are for jlox's standalone scanner mode, which rlox don't support as this is complete VM implementations. They are automatically skipped by the test runner.

## Integration

Add to your CI/CD:

```yaml
- name: Test Lox interpreter
  run: |
    cargo build --release --manifest-path=rlox/Cargo.toml
    cargo build --release --bin test_runner --manifest-path=rlox/Cargo.toml
    rlox/target/release/test_runner rlox/target/release/rlox rlox/test
```

---

## References

- Book: "Crafting Interpreters" by Robert Nystrom
- Website: https://craftinginterpreters.com/

## License

This implementation follows the same license as the original "Crafting Interpreters" code.
