//! Run with `cargo test --test regression`; append a test-name filter to narrow the run.
//! Covers the implemented 2026-guide subset and interpreter extensions.
//! Records, pointers, sets, OOP, random-access files, WASM replay and CLI flags are excluded.
//! LCASE/UCASE preserve CHAR inputs (guide section 5.5) and STRING inputs (extension).

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        loop {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "cps-regression-{}-{stamp}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("Cannot create test directory: {e}"),
            }
        }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_case(
    name: &str,
    source: &str,
    expected: &str,
    input: &str,
    error: Option<&str>,
    files: &[(&str, &str)],
) {
    let directory = TestDirectory::new();
    let path = &directory.0;
    fs::write(path.join("case.cps"), source).unwrap();
    let stdout_path = path.join("stdout.log");
    let stderr_path = path.join("stderr.log");
    let mut child = Command::new(env!("CARGO_BIN_EXE_cps"))
        .arg("case.cps")
        .current_dir(path)
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(File::create(&stdout_path).unwrap())
        .stderr(File::create(&stderr_path).unwrap())
        .spawn()
        .expect("Cannot start cps");
    let write_result = child.stdin.take().unwrap().write_all(input.as_bytes());
    let deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{name}: interpreter timed out after 8 seconds");
        }
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = fs::read_to_string(stdout_path).unwrap();
    let stderr = fs::read_to_string(stderr_path).unwrap();
    let output = format!("{stdout}{stderr}");
    assert!(
        !output.contains("panicked"),
        "{name}: interpreter panicked: {output}"
    );
    match error {
        Some(expected_error) => {
            assert!(
                status.code().is_some_and(|code| code > 0),
                "{name}: expected an error, got {status}: {output}"
            );
            let diagnostic = output
                .lines()
                .find(|line| line.starts_with("ERROR:"))
                .unwrap_or_else(|| panic!("{name}: missing diagnostic: {output}"));
            assert!(
                diagnostic
                    .to_lowercase()
                    .contains(&expected_error.to_lowercase()),
                "{name}: expected {expected_error:?}: {output}"
            );
        }
        None => {
            write_result.expect("Cannot supply test input");
            assert!(status.success(), "{name}: {status}: {output}");
            assert!(stderr.is_empty(), "{name}: unexpected stderr: {stderr}");
            assert_eq!(stdout, expected, "{name}");
        }
    }
    for (filename, expected_content) in files {
        let content = fs::read_to_string(path.join(filename)).unwrap();
        assert_eq!(content, *expected_content, "{name}: file {filename}");
    }
}

macro_rules! case {
    ($id:ident, $name:expr, $source:expr, $expected:expr, $input:expr, $error:expr, $files:expr) => {
        #[test]
        fn $id() {
            run_case($name, $source, $expected, $input, $error, $files);
        }
    };
}

// /// A bug that has been found but not fixed. These state the behaviour that is wanted, so each one
// /// is red now and turns green on the day the bug is fixed; rename it to `case!` when that happens.
// /// They run by default and are meant to stay visible. To ask the narrower question "have I broken
// /// anything that used to work", skip them: `cargo test --test regression -- --skip known_bug`.
// macro_rules! known_bug {
//     ($id:ident, $name:expr, $source:expr, $expected:expr, $input:expr, $error:expr, $files:expr) => {
//         #[test]
//         fn $id() {
//             run_case($name, $source, $expected, $input, $error, $files);
//         }
//     };
// }

case!(
    case_001_arithmetic_2_3_4,
    "arithmetic 2 + 3 * 4",
    "OUTPUT 2 + 3 * 4\n",
    "14\n",
    "",
    None,
    &[]
);

case!(
    case_002_arithmetic_2_3_4,
    "arithmetic (2 + 3) * 4",
    "OUTPUT (2 + 3) * 4\n",
    "20\n",
    "",
    None,
    &[]
);

case!(
    case_003_arithmetic_20_3_2,
    "arithmetic 20 - 3 - 2",
    "OUTPUT 20 - 3 - 2\n",
    "15\n",
    "",
    None,
    &[]
);

case!(
    case_004_arithmetic_24_3_2,
    "arithmetic 24 / 3 / 2",
    "OUTPUT 24 / 3 / 2\n",
    "4\n",
    "",
    None,
    &[]
);

case!(
    case_005_arithmetic_7_2,
    "arithmetic 7 / 2",
    "OUTPUT 7 / 2\n",
    "3.5\n",
    "",
    None,
    &[]
);

case!(
    case_006_arithmetic_17_div_5,
    "arithmetic 17 DIV 5",
    "OUTPUT 17 DIV 5\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_007_arithmetic_17_mod_5,
    "arithmetic 17 MOD 5",
    "OUTPUT 17 MOD 5\n",
    "2\n",
    "",
    None,
    &[]
);

case!(
    case_008_arithmetic_2_3_2,
    "arithmetic 2 ^ 3 ^ 2",
    "OUTPUT 2 ^ 3 ^ 2\n",
    "512\n",
    "",
    None,
    &[]
);

case!(
    case_009_arithmetic_2_3_2,
    "arithmetic (2 ^ 3) ^ 2",
    "OUTPUT (2 ^ 3) ^ 2\n",
    "64\n",
    "",
    None,
    &[]
);

case!(
    case_010_arithmetic_2_3_4,
    "arithmetic 2 ^ 3 * 4",
    "OUTPUT 2 ^ 3 * 4\n",
    "32\n",
    "",
    None,
    &[]
);

case!(
    case_011_arithmetic_5_2,
    "arithmetic -5 + 2",
    "OUTPUT -5 + 2\n",
    "-3\n",
    "",
    None,
    &[]
);

case!(
    case_012_arithmetic_5_2,
    "arithmetic 5-2",
    "OUTPUT 5-2\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_013_arithmetic_5_2,
    "arithmetic 5 - -2",
    "OUTPUT 5 - -2\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    case_014_arithmetic_1_000_2,
    "arithmetic 1_000 + 2",
    "OUTPUT 1_000 + 2\n",
    "1002\n",
    "",
    None,
    &[]
);

case!(
    case_015_arithmetic_0_900,
    "arithmetic 0 * 900",
    "OUTPUT 0 * 900\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_016_arithmetic_3_25_0_5,
    "arithmetic 3.25 + 0.5",
    "OUTPUT 3.25 + 0.5\n",
    "3.75\n",
    "",
    None,
    &[]
);

case!(
    case_017_logic_false_and_false,
    "logic False AND False",
    "OUTPUT FALSE AND FALSE\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_018_logic_false_or_false,
    "logic False OR False",
    "OUTPUT FALSE OR FALSE\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_019_logic_false_and_true,
    "logic False AND True",
    "OUTPUT FALSE AND TRUE\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_020_logic_false_or_true,
    "logic False OR True",
    "OUTPUT FALSE OR TRUE\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_021_not_false,
    "NOT False",
    "OUTPUT NOT FALSE\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_022_logic_true_and_false,
    "logic True AND False",
    "OUTPUT TRUE AND FALSE\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_023_logic_true_or_false,
    "logic True OR False",
    "OUTPUT TRUE OR FALSE\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_024_logic_true_and_true,
    "logic True AND True",
    "OUTPUT TRUE AND TRUE\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_025_logic_true_or_true,
    "logic True OR True",
    "OUTPUT TRUE OR TRUE\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_026_not_true,
    "NOT True",
    "OUTPUT NOT TRUE\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_027_logical_precedence,
    "logical precedence",
    "OUTPUT TRUE OR FALSE AND FALSE\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_028_comparison_2_3,
    "comparison 2 = 3",
    "OUTPUT 2 = 3\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_029_comparison_2_3,
    "comparison 2 <> 3",
    "OUTPUT 2 <> 3\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_030_comparison_2_3,
    "comparison 2 < 3",
    "OUTPUT 2 < 3\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_031_comparison_2_3,
    "comparison 2 <= 3",
    "OUTPUT 2 <= 3\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_032_comparison_2_3,
    "comparison 2 > 3",
    "OUTPUT 2 > 3\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_033_comparison_2_3,
    "comparison 2 >= 3",
    "OUTPUT 2 >= 3\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_034_comparison_3_0_3,
    "comparison 3.0 = 3",
    "OUTPUT 3.0 = 3\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_035_comparison_3_0_3,
    "comparison 3.0 <> 3",
    "OUTPUT 3.0 <> 3\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_036_comparison_3_0_3,
    "comparison 3.0 < 3",
    "OUTPUT 3.0 < 3\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_037_comparison_3_0_3,
    "comparison 3.0 <= 3",
    "OUTPUT 3.0 <= 3\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_038_comparison_3_0_3,
    "comparison 3.0 > 3",
    "OUTPUT 3.0 > 3\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_039_comparison_3_0_3,
    "comparison 3.0 >= 3",
    "OUTPUT 3.0 >= 3\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_040_comparison_a_b,
    "comparison \"a\" = \"b\"",
    "OUTPUT \"a\" = \"b\"\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_041_comparison_a_b,
    "comparison \"a\" <> \"b\"",
    "OUTPUT \"a\" <> \"b\"\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_042_comparison_a_b,
    "comparison \"a\" < \"b\"",
    "OUTPUT \"a\" < \"b\"\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_043_comparison_a_b,
    "comparison \"a\" <= \"b\"",
    "OUTPUT \"a\" <= \"b\"\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_044_comparison_a_b,
    "comparison \"a\" > \"b\"",
    "OUTPUT \"a\" > \"b\"\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_045_comparison_a_b,
    "comparison \"a\" >= \"b\"",
    "OUTPUT \"a\" >= \"b\"\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_046_comparison_a_b,
    "comparison 'A' = 'B'",
    "OUTPUT 'A' = 'B'\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_047_comparison_a_b,
    "comparison 'A' <> 'B'",
    "OUTPUT 'A' <> 'B'\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_048_comparison_a_b,
    "comparison 'A' < 'B'",
    "OUTPUT 'A' < 'B'\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_049_comparison_a_b,
    "comparison 'A' <= 'B'",
    "OUTPUT 'A' <= 'B'\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_050_comparison_a_b,
    "comparison 'A' > 'B'",
    "OUTPUT 'A' > 'B'\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_051_comparison_a_b,
    "comparison 'A' >= 'B'",
    "OUTPUT 'A' >= 'B'\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_052_comparison_31_12_2025_01_01_2026,
    "comparison 31/12/2025 = 01/01/2026",
    "OUTPUT 31/12/2025 = 01/01/2026\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_053_comparison_31_12_2025_01_01_2026,
    "comparison 31/12/2025 <> 01/01/2026",
    "OUTPUT 31/12/2025 <> 01/01/2026\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_054_comparison_31_12_2025_01_01_2026,
    "comparison 31/12/2025 < 01/01/2026",
    "OUTPUT 31/12/2025 < 01/01/2026\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_055_comparison_31_12_2025_01_01_2026,
    "comparison 31/12/2025 <= 01/01/2026",
    "OUTPUT 31/12/2025 <= 01/01/2026\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_056_comparison_31_12_2025_01_01_2026,
    "comparison 31/12/2025 > 01/01/2026",
    "OUTPUT 31/12/2025 > 01/01/2026\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_057_comparison_31_12_2025_01_01_2026,
    "comparison 31/12/2025 >= 01/01/2026",
    "OUTPUT 31/12/2025 >= 01/01/2026\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_058_declare_assign_integer,
    "declare assign INTEGER",
    "DECLARE V : INTEGER\nV <- 42\nOUTPUT V\n",
    "42\n",
    "",
    None,
    &[]
);

case!(
    case_059_input_integer,
    "input INTEGER",
    "DECLARE V : INTEGER\nINPUT V\nOUTPUT V\n",
    "42\n",
    "42\n",
    None,
    &[]
);

case!(
    case_060_constant_integer,
    "constant INTEGER",
    "CONSTANT K = 42\nOUTPUT K\n",
    "42\n",
    "",
    None,
    &[]
);

case!(
    case_061_declare_assign_real,
    "declare assign REAL",
    "DECLARE V : REAL\nV <- 2.5\nOUTPUT V\n",
    "2.5\n",
    "",
    None,
    &[]
);

case!(
    case_062_input_real,
    "input REAL",
    "DECLARE V : REAL\nINPUT V\nOUTPUT V\n",
    "2.5\n",
    "2.5\n",
    None,
    &[]
);

case!(
    case_063_constant_real,
    "constant REAL",
    "CONSTANT K = 2.5\nOUTPUT K\n",
    "2.5\n",
    "",
    None,
    &[]
);

case!(
    case_064_declare_assign_string,
    "declare assign STRING",
    "DECLARE V : STRING\nV <- \"hello\"\nOUTPUT V\n",
    "hello\n",
    "",
    None,
    &[]
);

case!(
    case_065_input_string,
    "input STRING",
    "DECLARE V : STRING\nINPUT V\nOUTPUT V\n",
    "hello\n",
    "hello\n",
    None,
    &[]
);

case!(
    case_066_constant_string,
    "constant STRING",
    "CONSTANT K = \"hello\"\nOUTPUT K\n",
    "hello\n",
    "",
    None,
    &[]
);

case!(
    case_067_declare_assign_char,
    "declare assign CHAR",
    "DECLARE V : CHAR\nV <- 'Q'\nOUTPUT V\n",
    "Q\n",
    "",
    None,
    &[]
);

case!(
    case_068_input_char,
    "input CHAR",
    "DECLARE V : CHAR\nINPUT V\nOUTPUT V\n",
    "Q\n",
    "Q\n",
    None,
    &[]
);

case!(
    case_069_constant_char,
    "constant CHAR",
    "CONSTANT K = 'Q'\nOUTPUT K\n",
    "Q\n",
    "",
    None,
    &[]
);

case!(
    case_070_declare_assign_boolean,
    "declare assign BOOLEAN",
    "DECLARE V : BOOLEAN\nV <- TRUE\nOUTPUT V\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_071_input_boolean,
    "input BOOLEAN",
    "DECLARE V : BOOLEAN\nINPUT V\nOUTPUT V\n",
    "TRUE\n",
    "TRUE\n",
    None,
    &[]
);

case!(
    case_072_constant_boolean,
    "constant BOOLEAN",
    "CONSTANT K = TRUE\nOUTPUT K\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_073_declare_assign_date,
    "declare assign DATE",
    "DECLARE V : DATE\nV <- 29/02/2024\nOUTPUT V\n",
    "29/02/2024\n",
    "",
    None,
    &[]
);

case!(
    case_074_input_date,
    "input DATE",
    "DECLARE V : DATE\nINPUT V\nOUTPUT V\n",
    "29/02/2024\n",
    "29/02/2024\n",
    None,
    &[]
);

case!(
    case_075_constant_date,
    "constant DATE",
    "CONSTANT K = 29/02/2024\nOUTPUT K\n",
    "29/02/2024\n",
    "",
    None,
    &[]
);

case!(
    case_076_numeric_assignment_conversions,
    "numeric assignment conversions",
    "DECLARE I : INTEGER\nDECLARE R : REAL\nI <- 4.0\nR <- I\nOUTPUT I, \":\", R\n",
    "4:4\n",
    "",
    None,
    &[]
);

case!(
    case_077_comments_and_unicode_arrow,
    "comments and unicode arrow",
    "// comment\nDECLARE V : INTEGER\nV ← 6 // trailing\nOUTPUT V\n",
    "6\n",
    "",
    None,
    &[]
);

case!(
    case_078_concatenation,
    "concatenation",
    "OUTPUT \"hello\" & \" \" & \"world\"\n",
    "hello world\n",
    "",
    None,
    &[]
);

case!(
    case_079_multiple_output_operands,
    "multiple output operands",
    "OUTPUT \"x\", 3, TRUE, 'A'\n",
    "x3TRUEA\n",
    "",
    None,
    &[]
);

case!(
    case_080_builtin_right_cambridge_6,
    "builtin RIGHT(\"Cambridge\", 6)",
    "OUTPUT RIGHT(\"Cambridge\", 6)\n",
    "bridge\n",
    "",
    None,
    &[]
);

case!(
    case_081_builtin_right_abc_0,
    "builtin RIGHT(\"abc\", 0)",
    "OUTPUT RIGHT(\"abc\", 0)\n",
    "\n",
    "",
    None,
    &[]
);

case!(
    case_082_builtin_right_abc_20,
    "builtin RIGHT(\"abc\", 20)",
    "OUTPUT RIGHT(\"abc\", 20)\n",
    "abc\n",
    "",
    None,
    &[]
);

case!(
    case_083_builtin_length,
    "builtin LENGTH(\"\")",
    "OUTPUT LENGTH(\"\")\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_084_builtin_length_caf,
    "builtin LENGTH(\"café\")",
    "OUTPUT LENGTH(\"café\")\n",
    "4\n",
    "",
    None,
    &[]
);

case!(
    case_085_builtin_mid_abcdef_2_3,
    "builtin MID(\"abcdef\", 2, 3)",
    "OUTPUT MID(\"abcdef\", 2, 3)\n",
    "bcd\n",
    "",
    None,
    &[]
);

case!(
    case_086_builtin_mid_abc_4_2,
    "builtin MID(\"abc\", 4, 2)",
    "OUTPUT MID(\"abc\", 4, 2)\n",
    "\n",
    "",
    None,
    &[]
);

case!(
    case_087_builtin_mid_abc_2_20,
    "builtin MID(\"abc\", 2, 20)",
    "OUTPUT MID(\"abc\", 2, 20)\n",
    "bc\n",
    "",
    None,
    &[]
);

case!(
    case_088_builtin_mid_abc_1_0,
    "builtin MID(\"abc\", 1, 0)",
    "OUTPUT MID(\"abc\", 1, 0)\n",
    "\n",
    "",
    None,
    &[]
);

case!(
    case_089_builtin_substring_abcdef_2_3,
    "builtin SUBSTRING(\"abcdef\", 2, 3)",
    "OUTPUT SUBSTRING(\"abcdef\", 2, 3)\n",
    "bcd\n",
    "",
    None,
    &[]
);

case!(
    case_090_builtin_lcase_string,
    "LCASE string input returns STRING",
    "DECLARE Result : STRING\nResult <- LCASE(\"Hello\")\nOUTPUT Result\n",
    "hello\n",
    "",
    None,
    &[]
);

case!(
    case_091_builtin_ucase_string,
    "UCASE string input returns STRING",
    "DECLARE Result : STRING\nResult <- UCASE(\"Hello\")\nOUTPUT Result\n",
    "HELLO\n",
    "",
    None,
    &[]
);

case!(
    builtin_lcase_char_preserves_type,
    "LCASE char input returns CHAR",
    "DECLARE Result : CHAR\nResult <- LCASE('A')\nOUTPUT Result\n",
    "a\n",
    "",
    None,
    &[]
);

case!(
    builtin_ucase_char_preserves_type,
    "UCASE char input returns CHAR",
    "DECLARE Result : CHAR\nResult <- UCASE('a')\nOUTPUT Result\n",
    "A\n",
    "",
    None,
    &[]
);

case!(
    builtin_lcase_single_character_string,
    "LCASE single-character string stays STRING",
    "DECLARE Result : STRING\nResult <- LCASE(\"A\")\nOUTPUT Result\n",
    "a\n",
    "",
    None,
    &[]
);

case!(
    builtin_ucase_single_character_string,
    "UCASE single-character string stays STRING",
    "DECLARE Result : STRING\nResult <- UCASE(\"a\")\nOUTPUT Result\n",
    "A\n",
    "",
    None,
    &[]
);

case!(
    builtin_lcase_empty_string,
    "LCASE empty string returns empty STRING",
    "DECLARE Result : STRING\nResult <- LCASE(\"\")\nOUTPUT Result\n",
    "\n",
    "",
    None,
    &[]
);

case!(
    builtin_ucase_empty_string,
    "UCASE empty string returns empty STRING",
    "DECLARE Result : STRING\nResult <- UCASE(\"\")\nOUTPUT Result\n",
    "\n",
    "",
    None,
    &[]
);

case!(
    builtin_lcase_unicode_char_expansion_rejected,
    "LCASE rejects a CHAR mapping to two Unicode scalars",
    "OUTPUT LCASE('İ')\n",
    "",
    "",
    Some("exactly one character"),
    &[]
);

case!(
    builtin_ucase_unicode_char_expansion_rejected,
    "UCASE rejects a CHAR mapping to two Unicode scalars",
    "OUTPUT UCASE('ß')\n",
    "",
    "",
    Some("exactly one character"),
    &[]
);

case!(
    builtin_ucase_unicode_ligature_expansion_rejected,
    "UCASE rejects a CHAR mapping to three Unicode scalars",
    "OUTPUT UCASE('ﬃ')\n",
    "",
    "",
    Some("exactly one character"),
    &[]
);

case!(
    builtin_lcase_unicode_char_preserved,
    "LCASE accepts a multi-byte single-scalar CHAR result",
    "DECLARE Result : CHAR\nResult <- LCASE('É')\nOUTPUT Result\n",
    "é\n",
    "",
    None,
    &[]
);

case!(
    builtin_ucase_unicode_char_preserved,
    "UCASE accepts a multi-byte single-scalar CHAR result",
    "DECLARE Result : CHAR\nResult <- UCASE('é')\nOUTPUT Result\n",
    "É\n",
    "",
    None,
    &[]
);

case!(
    builtin_lcase_unicode_string_expansion_preserved,
    "LCASE keeps every scalar of an expanded STRING result",
    "DECLARE Result : STRING\nResult <- LCASE(\"İ\")\nOUTPUT Result\n",
    "i̇\n",
    "",
    None,
    &[]
);

case!(
    builtin_ucase_unicode_string_expansion_preserved,
    "UCASE keeps every scalar of an expanded STRING result",
    "DECLARE Result : STRING\nResult <- UCASE(\"ßﬃ\")\nOUTPUT Result\n",
    "SSFFI\n",
    "",
    None,
    &[]
);

case!(
    case_092_builtin_int_3_9,
    "builtin INT(3.9)",
    "OUTPUT INT(3.9)\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_093_builtin_int_3_9,
    "builtin INT(-3.9)",
    "OUTPUT INT(-3.9)\n",
    "-3\n",
    "",
    None,
    &[]
);

case!(
    case_094_builtin_num_to_str_12_5,
    "builtin NUM_TO_STR(12.5)",
    "OUTPUT NUM_TO_STR(12.5)\n",
    "12.5\n",
    "",
    None,
    &[]
);

case!(
    case_095_builtin_str_to_num_12_5,
    "builtin STR_TO_NUM(\"12.5\")",
    "OUTPUT STR_TO_NUM(\"12.5\")\n",
    "12.5\n",
    "",
    None,
    &[]
);

case!(
    case_096_builtin_is_num_12_5,
    "builtin IS_NUM(\"-12.5\")",
    "OUTPUT IS_NUM(\"-12.5\")\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_097_builtin_is_num_abc,
    "builtin IS_NUM(\"abc\")",
    "OUTPUT IS_NUM(\"abc\")\n",
    "FALSE\n",
    "",
    None,
    &[]
);

case!(
    case_098_builtin_asc_a,
    "builtin ASC('A')",
    "OUTPUT ASC('A')\n",
    "65\n",
    "",
    None,
    &[]
);

case!(
    case_099_builtin_chr_65,
    "builtin CHR(65)",
    "OUTPUT CHR(65)\n",
    "A\n",
    "",
    None,
    &[]
);

case!(
    case_100_builtin_asc_chr_255,
    "builtin ASC(CHR(255))",
    "OUTPUT ASC(CHR(255))\n",
    "255\n",
    "",
    None,
    &[]
);

case!(
    case_101_builtin_asc_chr_0,
    "builtin ASC(CHR(0))",
    "OUTPUT ASC(CHR(0))\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_102_rand_range_repeated,
    "RAND range repeated",
    "DECLARE R : REAL\nDECLARE I : INTEGER\nDECLARE Valid : BOOLEAN\nValid <- TRUE\nFOR I <- 1 TO 200\nR <- RAND(7)\nIF R < 0 OR R >= 7 THEN\nValid <- FALSE\nENDIF\nNEXT I\nOUTPUT Valid\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_103_if_else_true,
    "IF ELSE TRUE",
    "IF TRUE THEN\nOUTPUT \"yes\"\nELSE\nOUTPUT \"no\"\nENDIF\n",
    "yes\n",
    "",
    None,
    &[]
);

case!(
    case_104_if_else_false,
    "IF ELSE FALSE",
    "IF FALSE THEN\nOUTPUT \"yes\"\nELSE\nOUTPUT \"no\"\nENDIF\n",
    "no\n",
    "",
    None,
    &[]
);

case!(
    case_105_nested_if_and_no_else,
    "nested IF and no ELSE",
    "IF TRUE THEN\nIF FALSE THEN\nOUTPUT \"bad\"\nELSE\nOUTPUT \"ok\"\nENDIF\nENDIF\nIF FALSE THEN\nOUTPUT \"bad\"\nENDIF\n",
    "ok\n",
    "",
    None,
    &[]
);

case!(
    case_106_case_1,
    "CASE 1",
    "DECLARE N : INTEGER\nN <- 1\nCASE OF N\n1 : OUTPUT \"one\"\n2 TO 4 : OUTPUT \"range\"\nOTHERWISE : OUTPUT \"other\"\nENDCASE\n",
    "one\n",
    "",
    None,
    &[]
);

case!(
    case_107_case_2,
    "CASE 2",
    "DECLARE N : INTEGER\nN <- 2\nCASE OF N\n1 : OUTPUT \"one\"\n2 TO 4 : OUTPUT \"range\"\nOTHERWISE : OUTPUT \"other\"\nENDCASE\n",
    "range\n",
    "",
    None,
    &[]
);

case!(
    case_108_case_4,
    "CASE 4",
    "DECLARE N : INTEGER\nN <- 4\nCASE OF N\n1 : OUTPUT \"one\"\n2 TO 4 : OUTPUT \"range\"\nOTHERWISE : OUTPUT \"other\"\nENDCASE\n",
    "range\n",
    "",
    None,
    &[]
);

case!(
    case_109_case_8,
    "CASE 8",
    "DECLARE N : INTEGER\nN <- 8\nCASE OF N\n1 : OUTPUT \"one\"\n2 TO 4 : OUTPUT \"range\"\nOTHERWISE : OUTPUT \"other\"\nENDCASE\n",
    "other\n",
    "",
    None,
    &[]
);

case!(
    case_110_for_1_4_1,
    "FOR 1:4:1",
    "DECLARE I : INTEGER\nDECLARE S : INTEGER\nS <- 0\nFOR I <- 1 TO 4 STEP 1\nS <- S + I\nNEXT I\nOUTPUT S\n",
    "10\n",
    "",
    None,
    &[]
);

case!(
    case_111_for_4_1_1,
    "FOR 4:1:-1",
    "DECLARE I : INTEGER\nDECLARE S : INTEGER\nS <- 0\nFOR I <- 4 TO 1 STEP -1\nS <- S + I\nNEXT I\nOUTPUT S\n",
    "10\n",
    "",
    None,
    &[]
);

case!(
    case_112_for_1_5_2,
    "FOR 1:5:2",
    "DECLARE I : INTEGER\nDECLARE S : INTEGER\nS <- 0\nFOR I <- 1 TO 5 STEP 2\nS <- S + I\nNEXT I\nOUTPUT S\n",
    "9\n",
    "",
    None,
    &[]
);

case!(
    case_113_for_5_1_1,
    "FOR 5:1:1",
    "DECLARE I : INTEGER\nDECLARE S : INTEGER\nS <- 0\nFOR I <- 5 TO 1 STEP 1\nS <- S + I\nNEXT I\nOUTPUT S\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_114_for_1_5_1,
    "FOR 1:5:-1",
    "DECLARE I : INTEGER\nDECLARE S : INTEGER\nS <- 0\nFOR I <- 1 TO 5 STEP -1\nS <- S + I\nNEXT I\nOUTPUT S\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_115_for_3_3_1,
    "FOR 3:3:1",
    "DECLARE I : INTEGER\nDECLARE S : INTEGER\nS <- 0\nFOR I <- 3 TO 3 STEP 1\nS <- S + I\nNEXT I\nOUTPUT S\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_116_while_pretest,
    "WHILE pretest",
    "DECLARE N : INTEGER\nN <- 0\nWHILE N < 3\nN <- N + 1\nENDWHILE\nWHILE FALSE\nN <- 99\nENDWHILE\nOUTPUT N\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_117_repeat_posttest,
    "REPEAT posttest",
    "DECLARE N : INTEGER\nN <- 0\nREPEAT\nN <- N + 1\nUNTIL N = 3\nREPEAT\nN <- N + 1\nUNTIL TRUE\nOUTPUT N\n",
    "4\n",
    "",
    None,
    &[]
);

case!(
    case_118_array_bounds_0_0,
    "array bounds 0:0",
    "DECLARE A : ARRAY[0:0] OF INTEGER\nDECLARE I : INTEGER\nFOR I <- 0 TO 0\nA[I] <- I * 7\nNEXT I\nFOR I <- 0 TO 0\nOUTPUT A[I]\nNEXT I\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_119_array_bounds_1_1,
    "array bounds 1:1",
    "DECLARE A : ARRAY[1:1] OF INTEGER\nDECLARE I : INTEGER\nFOR I <- 1 TO 1\nA[I] <- I * 7\nNEXT I\nFOR I <- 1 TO 1\nOUTPUT A[I]\nNEXT I\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    case_120_array_bounds_1_8,
    "array bounds 1:8",
    "DECLARE A : ARRAY[1:8] OF INTEGER\nDECLARE I : INTEGER\nFOR I <- 1 TO 8\nA[I] <- I * 7\nNEXT I\nFOR I <- 1 TO 8\nOUTPUT A[I]\nNEXT I\n",
    "7\n14\n21\n28\n35\n42\n49\n56\n",
    "",
    None,
    &[]
);

case!(
    case_121_array_bounds_5_9,
    "array bounds 5:9",
    "DECLARE A : ARRAY[5:9] OF INTEGER\nDECLARE I : INTEGER\nFOR I <- 5 TO 9\nA[I] <- I * 7\nNEXT I\nFOR I <- 5 TO 9\nOUTPUT A[I]\nNEXT I\n",
    "35\n42\n49\n56\n63\n",
    "",
    None,
    &[]
);

case!(
    case_122_2d_nonunit_bounds,
    "2D nonunit bounds",
    "DECLARE A : ARRAY[2:3, 4:6] OF INTEGER\nDECLARE I : INTEGER\nDECLARE J : INTEGER\nFOR I <- 2 TO 3\nFOR J <- 4 TO 6\nA[I,J] <- I * 10 + J\nNEXT J\nNEXT I\nFOR I <- 2 TO 3\nFOR J <- 4 TO 6\nOUTPUT A[I,J]\nNEXT J\nNEXT I\n",
    "24\n25\n26\n34\n35\n36\n",
    "",
    None,
    &[]
);

case!(
    case_123_array_deep_copy,
    "array deep copy",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nDECLARE B : ARRAY[1:2] OF INTEGER\nA[1] <- 7\nB <- A\nB[1] <- 9\nOUTPUT A[1], \":\", B[1]\n",
    "7:9\n",
    "",
    None,
    &[]
);

case!(
    case_124_array_input,
    "array INPUT",
    "DECLARE A : ARRAY[1:2, 1:2] OF INTEGER\nINPUT A[2,1]\nOUTPUT A[2,1]\n",
    "42\n",
    "42\n",
    None,
    &[]
);

case!(
    case_125_array_element_string,
    "array element STRING",
    "DECLARE A : ARRAY[1:2] OF STRING\nA[2] <- \"abc\"\nOUTPUT A[2]\n",
    "abc\n",
    "",
    None,
    &[]
);

case!(
    case_126_array_element_boolean,
    "array element BOOLEAN",
    "DECLARE A : ARRAY[1:2] OF BOOLEAN\nA[2] <- TRUE\nOUTPUT A[2]\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_127_array_element_date,
    "array element DATE",
    "DECLARE A : ARRAY[1:2] OF DATE\nA[2] <- 01/01/2026\nOUTPUT A[2]\n",
    "01/01/2026\n",
    "",
    None,
    &[]
);

case!(
    case_128_array_element_char,
    "array element CHAR",
    "DECLARE A : ARRAY[1:2] OF CHAR\nA[2] <- 'Q'\nOUTPUT A[2]\n",
    "Q\n",
    "",
    None,
    &[]
);

case!(
    case_129_array_element_real,
    "array element REAL",
    "DECLARE A : ARRAY[1:2] OF REAL\nA[2] <- 1.5\nOUTPUT A[2]\n",
    "1.5\n",
    "",
    None,
    &[]
);

case!(
    case_130_recursive_function,
    "recursive function",
    "FUNCTION Fact(N : INTEGER) RETURNS INTEGER\nIF N <= 1 THEN\nRETURN 1\nENDIF\nRETURN N * Fact(N - 1)\nENDFUNCTION\nOUTPUT Fact(6)\n",
    "720\n",
    "",
    None,
    &[]
);

case!(
    case_131_byval_byref_and_shadowing,
    "BYVAL BYREF and shadowing",
    "DECLARE N : INTEGER\nPROCEDURE Copy(BYVAL N : INTEGER)\nN <- 99\nENDPROCEDURE\nPROCEDURE Change(BYREF V : INTEGER)\nV <- V + 4\nENDPROCEDURE\nN <- 3\nCALL Copy(N)\nOUTPUT N\nCALL Change(N)\nOUTPUT N\n",
    "3\n7\n",
    "",
    None,
    &[]
);

case!(
    case_132_array_byval_byref,
    "array BYVAL BYREF",
    "PROCEDURE Copy(BYVAL A : ARRAY[1:2] OF INTEGER)\nA[1] <- 99\nENDPROCEDURE\nPROCEDURE Change(BYREF A : ARRAY[1:2] OF INTEGER)\nA[1] <- 8\nENDPROCEDURE\nDECLARE V : ARRAY[1:2] OF INTEGER\nV[1] <- 3\nCALL Copy(V)\nOUTPUT V[1]\nCALL Change(V)\nOUTPUT V[1]\n",
    "3\n8\n",
    "",
    None,
    &[]
);

case!(
    case_133_function_parameter_return_integer,
    "function parameter return INTEGER",
    "FUNCTION Echo(V : INTEGER) RETURNS INTEGER\nRETURN V\nENDFUNCTION\nOUTPUT Echo(7)\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    case_134_function_parameter_return_real,
    "function parameter return REAL",
    "FUNCTION Echo(V : REAL) RETURNS REAL\nRETURN V\nENDFUNCTION\nOUTPUT Echo(1.5)\n",
    "1.5\n",
    "",
    None,
    &[]
);

case!(
    case_135_function_parameter_return_string,
    "function parameter return STRING",
    "FUNCTION Echo(V : STRING) RETURNS STRING\nRETURN V\nENDFUNCTION\nOUTPUT Echo(\"hi\")\n",
    "hi\n",
    "",
    None,
    &[]
);

case!(
    case_136_function_parameter_return_char,
    "function parameter return CHAR",
    "FUNCTION Echo(V : CHAR) RETURNS CHAR\nRETURN V\nENDFUNCTION\nOUTPUT Echo('A')\n",
    "A\n",
    "",
    None,
    &[]
);

case!(
    case_137_function_parameter_return_boolean,
    "function parameter return BOOLEAN",
    "FUNCTION Echo(V : BOOLEAN) RETURNS BOOLEAN\nRETURN V\nENDFUNCTION\nOUTPUT Echo(TRUE)\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_138_function_parameter_return_date,
    "function parameter return DATE",
    "FUNCTION Echo(V : DATE) RETURNS DATE\nRETURN V\nENDFUNCTION\nOUTPUT Echo(01/01/2026)\n",
    "01/01/2026\n",
    "",
    None,
    &[]
);

case!(
    case_139_enum_assignment_equality,
    "enum assignment equality",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nS <- Autumn\nOUTPUT S\nOUTPUT S = Autumn\nOUTPUT S <> Winter\n",
    "Autumn\nTRUE\nTRUE\n",
    "",
    None,
    &[]
);

case!(
    case_140_enum_input,
    "enum input",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nINPUT S\nOUTPUT S\n",
    "Winter\n",
    "Winter\n",
    None,
    &[]
);

case!(
    case_141_enum_array,
    "enum array",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nDECLARE A : ARRAY[1:2] OF Season\nA[1] <- Summer\nOUTPUT A[1]\n",
    "Summer\n",
    "",
    None,
    &[]
);

case!(
    case_142_enum_case,
    "enum CASE",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nS <- Summer\nCASE OF S\nSpring : OUTPUT \"bad\"\nSummer : OUTPUT \"ok\"\nENDCASE\n",
    "ok\n",
    "",
    None,
    &[]
);

case!(
    case_143_enum_parameter_and_return,
    "enum parameter and return",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nFUNCTION Echo(V : Season) RETURNS Season\nRETURN V\nENDFUNCTION\nS <- Echo(Spring)\nOUTPUT S\n",
    "Spring\n",
    "",
    None,
    &[]
);

case!(
    case_144_text_write_append_read_eof,
    "text WRITE APPEND READ EOF",
    "DECLARE Line : STRING\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", \"alpha\"\nWRITEFILE \"data.txt\", \"\"\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR APPEND\nWRITEFILE \"data.txt\", \"omega\"\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nOUTPUT EOF(\"data.txt\")\nWHILE NOT EOF(\"data.txt\")\nREADFILE \"data.txt\", Line\nOUTPUT \"[\", Line, \"]\"\nENDWHILE\nOUTPUT EOF(\"data.txt\")\nCLOSEFILE \"data.txt\"\n",
    "FALSE\n[alpha]\n[]\n[omega]\nTRUE\n",
    "",
    None,
    &[("data.txt", "alpha\n\nomega\n")]
);

case!(
    case_145_write_truncates_and_empty_eof,
    "WRITE truncates and empty EOF",
    "OPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", \"old\"\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR WRITE\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nOUTPUT EOF(\"data.txt\")\nCLOSEFILE \"data.txt\"\n",
    "TRUE\n",
    "",
    None,
    &[("data.txt", "")]
);

case!(
    case_146_typed_readfile_integer,
    "typed READFILE INTEGER",
    "DECLARE V : INTEGER\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", 42\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", V\nCLOSEFILE \"data.txt\"\nOUTPUT V\n",
    "42\n",
    "",
    None,
    &[]
);

case!(
    case_147_typed_readfile_real,
    "typed READFILE REAL",
    "DECLARE V : REAL\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", 1.25\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", V\nCLOSEFILE \"data.txt\"\nOUTPUT V\n",
    "1.25\n",
    "",
    None,
    &[]
);

case!(
    case_148_typed_readfile_boolean,
    "typed READFILE BOOLEAN",
    "DECLARE V : BOOLEAN\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", TRUE\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", V\nCLOSEFILE \"data.txt\"\nOUTPUT V\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    case_149_typed_readfile_char,
    "typed READFILE CHAR",
    "DECLARE V : CHAR\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", 'Q'\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", V\nCLOSEFILE \"data.txt\"\nOUTPUT V\n",
    "Q\n",
    "",
    None,
    &[]
);

case!(
    case_150_typed_readfile_date,
    "typed READFILE DATE",
    "DECLARE V : DATE\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", 01/01/2026\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", V\nCLOSEFILE \"data.txt\"\nOUTPUT V\n",
    "01/01/2026\n",
    "",
    None,
    &[]
);

case!(
    case_151_reject_undefined,
    "reject undefined",
    "OUTPUT Unknown\n",
    "",
    "",
    Some("Undefined"),
    &[]
);

case!(
    case_152_reject_type_mismatch,
    "reject type mismatch",
    "DECLARE V : INTEGER\nV <- \"bad\"\n",
    "",
    "",
    Some("Type mismatch"),
    &[]
);

case!(
    case_153_reject_fractional_integer,
    "reject fractional integer",
    "DECLARE V : INTEGER\nV <- 1.5\n",
    "",
    "",
    Some("Type mismatch"),
    &[]
);

case!(
    case_154_reject_constant_mutation,
    "reject constant mutation",
    "CONSTANT K = 3\nK <- 4\n",
    "",
    "",
    Some("constant"),
    &[]
);

case!(
    case_155_reject_zero_division,
    "reject zero division",
    "OUTPUT 1 / 0\n",
    "",
    "",
    Some("zero"),
    &[]
);

case!(
    case_156_reject_zero_div,
    "reject zero DIV",
    "OUTPUT 1 DIV 0\n",
    "",
    "",
    Some("zero"),
    &[]
);

case!(
    case_157_reject_zero_mod,
    "reject zero MOD",
    "OUTPUT 1 MOD 0\n",
    "",
    "",
    Some("zero"),
    &[]
);

case!(
    case_158_reject_zero_step,
    "reject zero STEP",
    "FOR I <- 1 TO 3 STEP 0\nNEXT I\n",
    "",
    "",
    Some("STEP"),
    &[]
);

case!(
    case_159_reject_array_low,
    "reject array low",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nOUTPUT A[0]\n",
    "",
    "",
    Some("bound"),
    &[]
);

case!(
    case_160_reject_array_high,
    "reject array high",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nA[3] <- 1\n",
    "",
    "",
    Some("bound"),
    &[]
);

case!(
    case_161_reject_array_fractional_index,
    "reject array fractional index",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nOUTPUT A[1.5]\n",
    "",
    "",
    Some("integer"),
    &[]
);

case!(
    case_162_reject_array_column,
    "reject array column",
    "DECLARE A : ARRAY[1:2,1:2] OF INTEGER\nOUTPUT A[1,3]\n",
    "",
    "",
    Some("bound"),
    &[]
);

case!(
    case_163_reject_array_missing_column,
    "reject array missing column",
    "DECLARE A : ARRAY[1:2,1:2] OF INTEGER\nOUTPUT A[1]\n",
    "",
    "",
    Some("column"),
    &[]
);

case!(
    case_164_reject_array_output,
    "reject array output",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nOUTPUT A\n",
    "",
    "",
    Some("whole array"),
    &[]
);

case!(
    case_165_reject_unknown_type,
    "reject unknown type",
    "DECLARE V : Unknown\n",
    "",
    "",
    Some("not been defined"),
    &[]
);

case!(
    case_166_reject_enum_unassigned,
    "reject enum unassigned",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nOUTPUT S\n",
    "",
    "",
    Some("before one has been assigned"),
    &[]
);

case!(
    case_167_reject_enum_ordering,
    "reject enum ordering",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nOUTPUT Spring < Summer\n",
    "",
    "",
    Some("compare"),
    &[]
);

case!(
    case_168_reject_enum_duplicate,
    "reject enum duplicate",
    "TYPE E = (A, A)\n",
    "",
    "",
    Some("twice"),
    &[]
);

case!(
    case_169_reject_cross_enum,
    "reject cross enum",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nTYPE E = (Other)\nS <- Other\n",
    "",
    "",
    Some("Type mismatch"),
    &[]
);

case!(
    case_170_reject_missing_return,
    "reject missing return",
    "FUNCTION F() RETURNS INTEGER\nENDFUNCTION\nOUTPUT F()\n",
    "",
    "",
    Some("RETURN"),
    &[]
);

case!(
    case_171_reject_bad_return,
    "reject bad return",
    "FUNCTION F() RETURNS INTEGER\nRETURN \"bad\"\nENDFUNCTION\nOUTPUT F()\n",
    "",
    "",
    Some("type mismatch"),
    &[]
);

case!(
    case_172_reject_arity,
    "reject arity",
    "FUNCTION F(V : INTEGER) RETURNS INTEGER\nRETURN V\nENDFUNCTION\nOUTPUT F()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_173_reject_byref_literal,
    "reject BYREF literal",
    "PROCEDURE P(BYREF V : INTEGER)\nENDPROCEDURE\nCALL P(3)\n",
    "",
    "",
    Some("reference"),
    &[]
);

case!(
    case_174_reject_missing_file,
    "reject missing file",
    "OPENFILE \"missing.txt\" FOR READ\n",
    "",
    "",
    Some("file"),
    &[]
);

case!(
    case_175_reject_unopened_read,
    "reject unopened read",
    "DECLARE V : STRING\nREADFILE \"missing.txt\", V\n",
    "",
    "",
    Some("open"),
    &[]
);

case!(
    case_176_reject_missing_then,
    "reject missing THEN",
    "IF TRUE\nOUTPUT 1\nENDIF\n",
    "",
    "",
    Some("THEN"),
    &[]
);

case!(
    case_177_reject_missing_colon,
    "reject missing colon",
    "DECLARE V INTEGER\n",
    "",
    "",
    Some("colon"),
    &[]
);

case!(
    case_178_reject_stray_terminator,
    "reject stray terminator",
    "ENDIF\n",
    "",
    "",
    Some("Unexpected"),
    &[]
);

case!(
    case_179_reject_invalid_character,
    "reject invalid character",
    "OUTPUT @\n",
    "",
    "",
    Some("character"),
    &[]
);

case!(
    case_180_reject_unterminated_string,
    "reject unterminated string",
    "OUTPUT \"abc\n",
    "",
    "",
    Some("closed"),
    &[]
);

case!(
    case_181_reject_input_integer,
    "reject input INTEGER",
    "DECLARE V : INTEGER\nINPUT V\n",
    "",
    "x\n",
    Some("Expected"),
    &[]
);

case!(
    case_182_reject_input_real,
    "reject input REAL",
    "DECLARE V : REAL\nINPUT V\n",
    "",
    "x\n",
    Some("Expected"),
    &[]
);

case!(
    case_183_reject_input_boolean,
    "reject input BOOLEAN",
    "DECLARE V : BOOLEAN\nINPUT V\n",
    "",
    "maybe\n",
    Some("Expected"),
    &[]
);

case!(
    case_184_reject_input_char,
    "reject input CHAR",
    "DECLARE V : CHAR\nINPUT V\n",
    "",
    "ab\n",
    Some("Expected"),
    &[]
);

case!(
    case_185_reject_input_date,
    "reject input DATE",
    "DECLARE V : DATE\nINPUT V\n",
    "",
    "31/02/2026\n",
    Some("valid date"),
    &[]
);

case!(
    case_186_reject_enum_input_spelling,
    "reject enum input spelling",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nINPUT S\n",
    "",
    "Sumer\n",
    Some("variant"),
    &[]
);

case!(
    case_187_reject_right_abc_1,
    "reject RIGHT(\"abc\", -1)",
    "OUTPUT RIGHT(\"abc\", -1)\n",
    "",
    "",
    Some("non-negative"),
    &[]
);

case!(
    case_188_reject_mid_abc_0_1,
    "reject MID(\"abc\", 0, 1)",
    "OUTPUT MID(\"abc\", 0, 1)\n",
    "",
    "",
    Some(">= 1"),
    &[]
);

case!(
    case_189_reject_mid_abc_1_1,
    "reject MID(\"abc\", 1, -1)",
    "OUTPUT MID(\"abc\", 1, -1)\n",
    "",
    "",
    Some("non-negative"),
    &[]
);

case!(
    case_190_reject_chr_256,
    "reject CHR(256)",
    "OUTPUT CHR(256)\n",
    "",
    "",
    Some("range"),
    &[]
);

case!(
    case_191_reject_chr_1,
    "reject CHR(-1)",
    "OUTPUT CHR(-1)\n",
    "",
    "",
    Some("range"),
    &[]
);

case!(
    case_192_reject_rand_0,
    "reject RAND(0)",
    "OUTPUT RAND(0)\n",
    "",
    "",
    Some("positive"),
    &[]
);

case!(
    case_193_reject_int_3,
    "reject INT(\"3\")",
    "OUTPUT INT(\"3\")\n",
    "",
    "",
    Some("cannot be applied"),
    &[]
);

case!(
    case_194_reject_str_to_num_x,
    "reject STR_TO_NUM(\"x\")",
    "OUTPUT STR_TO_NUM(\"x\")\n",
    "",
    "",
    Some("numeric"),
    &[]
);

case!(
    case_195_reject_length_3,
    "reject LENGTH(3)",
    "OUTPUT LENGTH(3)\n",
    "",
    "",
    Some("string"),
    &[]
);

case!(
    case_196_reject_builtin_arity_right,
    "reject builtin arity RIGHT",
    "OUTPUT RIGHT()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_197_reject_builtin_arity_length,
    "reject builtin arity LENGTH",
    "OUTPUT LENGTH()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_198_reject_builtin_arity_mid,
    "reject builtin arity MID",
    "OUTPUT MID()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_199_reject_builtin_arity_substring,
    "reject builtin arity SUBSTRING",
    "OUTPUT SUBSTRING()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_200_reject_builtin_arity_lcase,
    "reject builtin arity LCASE",
    "OUTPUT LCASE()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_201_reject_builtin_arity_ucase,
    "reject builtin arity UCASE",
    "OUTPUT UCASE()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_202_reject_builtin_arity_int,
    "reject builtin arity INT",
    "OUTPUT INT()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_203_reject_builtin_arity_rand,
    "reject builtin arity RAND",
    "OUTPUT RAND()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_204_reject_builtin_arity_num_to_str,
    "reject builtin arity NUM_TO_STR",
    "OUTPUT NUM_TO_STR()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_205_reject_builtin_arity_str_to_num,
    "reject builtin arity STR_TO_NUM",
    "OUTPUT STR_TO_NUM()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_206_reject_builtin_arity_is_num,
    "reject builtin arity IS_NUM",
    "OUTPUT IS_NUM()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_207_reject_builtin_arity_asc,
    "reject builtin arity ASC",
    "OUTPUT ASC()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_208_reject_builtin_arity_chr,
    "reject builtin arity CHR",
    "OUTPUT CHR()\n",
    "",
    "",
    Some("argument"),
    &[]
);

case!(
    case_209_guide_char_return_lcase,
    "guide CHAR return LCASE",
    "DECLARE C : CHAR\nC <- LCASE('W')\nOUTPUT C\n",
    "w\n",
    "",
    None,
    &[]
);

case!(
    case_210_guide_char_return_ucase,
    "guide CHAR return UCASE",
    "DECLARE C : CHAR\nC <- UCASE('h')\nOUTPUT C\n",
    "H\n",
    "",
    None,
    &[]
);

case!(
    case_211_integer_variables_7_1,
    "integer variables -7 + 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 1\nOUTPUT A + B\n",
    "-6\n",
    "",
    None,
    &[]
);

case!(
    case_212_integer_variables_7_1,
    "integer variables -7 - 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 1\nOUTPUT A - B\n",
    "-8\n",
    "",
    None,
    &[]
);

case!(
    case_213_integer_variables_7_1,
    "integer variables -7 * 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 1\nOUTPUT A * B\n",
    "-7\n",
    "",
    None,
    &[]
);

case!(
    case_214_integer_variables_7_3,
    "integer variables -7 + 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 3\nOUTPUT A + B\n",
    "-4\n",
    "",
    None,
    &[]
);

case!(
    case_215_integer_variables_7_3,
    "integer variables -7 - 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 3\nOUTPUT A - B\n",
    "-10\n",
    "",
    None,
    &[]
);

case!(
    case_216_integer_variables_7_3,
    "integer variables -7 * 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 3\nOUTPUT A * B\n",
    "-21\n",
    "",
    None,
    &[]
);

case!(
    case_217_integer_variables_7_8,
    "integer variables -7 + 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 8\nOUTPUT A + B\n",
    "1\n",
    "",
    None,
    &[]
);

case!(
    case_218_integer_variables_7_8,
    "integer variables -7 - 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 8\nOUTPUT A - B\n",
    "-15\n",
    "",
    None,
    &[]
);

case!(
    case_219_integer_variables_7_8,
    "integer variables -7 * 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- -7\nB <- 8\nOUTPUT A * B\n",
    "-56\n",
    "",
    None,
    &[]
);

case!(
    case_220_integer_variables_0_1,
    "integer variables 0 + 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 1\nOUTPUT A + B\n",
    "1\n",
    "",
    None,
    &[]
);

case!(
    case_221_integer_variables_0_1,
    "integer variables 0 - 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 1\nOUTPUT A - B\n",
    "-1\n",
    "",
    None,
    &[]
);

case!(
    case_222_integer_variables_0_1,
    "integer variables 0 * 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 1\nOUTPUT A * B\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_223_integer_variables_0_3,
    "integer variables 0 + 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 3\nOUTPUT A + B\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_224_integer_variables_0_3,
    "integer variables 0 - 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 3\nOUTPUT A - B\n",
    "-3\n",
    "",
    None,
    &[]
);

case!(
    case_225_integer_variables_0_3,
    "integer variables 0 * 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 3\nOUTPUT A * B\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_226_integer_variables_0_8,
    "integer variables 0 + 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 8\nOUTPUT A + B\n",
    "8\n",
    "",
    None,
    &[]
);

case!(
    case_227_integer_variables_0_8,
    "integer variables 0 - 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 8\nOUTPUT A - B\n",
    "-8\n",
    "",
    None,
    &[]
);

case!(
    case_228_integer_variables_0_8,
    "integer variables 0 * 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 0\nB <- 8\nOUTPUT A * B\n",
    "0\n",
    "",
    None,
    &[]
);

case!(
    case_229_integer_variables_5_1,
    "integer variables 5 + 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 1\nOUTPUT A + B\n",
    "6\n",
    "",
    None,
    &[]
);

case!(
    case_230_integer_variables_5_1,
    "integer variables 5 - 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 1\nOUTPUT A - B\n",
    "4\n",
    "",
    None,
    &[]
);

case!(
    case_231_integer_variables_5_1,
    "integer variables 5 * 1",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 1\nOUTPUT A * B\n",
    "5\n",
    "",
    None,
    &[]
);

case!(
    case_232_integer_variables_5_3,
    "integer variables 5 + 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 3\nOUTPUT A + B\n",
    "8\n",
    "",
    None,
    &[]
);

case!(
    case_233_integer_variables_5_3,
    "integer variables 5 - 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 3\nOUTPUT A - B\n",
    "2\n",
    "",
    None,
    &[]
);

case!(
    case_234_integer_variables_5_3,
    "integer variables 5 * 3",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 3\nOUTPUT A * B\n",
    "15\n",
    "",
    None,
    &[]
);

case!(
    case_235_integer_variables_5_8,
    "integer variables 5 + 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 8\nOUTPUT A + B\n",
    "13\n",
    "",
    None,
    &[]
);

case!(
    case_236_integer_variables_5_8,
    "integer variables 5 - 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 8\nOUTPUT A - B\n",
    "-3\n",
    "",
    None,
    &[]
);

case!(
    case_237_integer_variables_5_8,
    "integer variables 5 * 8",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 5\nB <- 8\nOUTPUT A * B\n",
    "40\n",
    "",
    None,
    &[]
);

case!(
    case_238_early_return_inside_loop,
    "early RETURN inside loop",
    "FUNCTION Find() RETURNS INTEGER\nDECLARE I : INTEGER\nFOR I <- 1 TO 5\nIF I = 3 THEN\nRETURN I\nENDIF\nNEXT I\nRETURN 99\nENDFUNCTION\nOUTPUT Find()\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    case_239_nested_calls_and_local_lifetime,
    "nested calls and local lifetime",
    "DECLARE Global : INTEGER\nPROCEDURE Inner()\nDECLARE Local : INTEGER\nLocal <- 7\nGlobal <- Global + Local\nENDPROCEDURE\nPROCEDURE Outer()\nCALL Inner()\nENDPROCEDURE\nGlobal <- 0\nCALL Outer()\nCALL Outer()\nOUTPUT Global\n",
    "14\n",
    "",
    None,
    &[]
);

case!(
    case_240_function_array_return,
    "function array return",
    "FUNCTION Make() RETURNS ARRAY[1:2] OF INTEGER\nDECLARE A : ARRAY[1:2] OF INTEGER\nA[1] <- 3\nA[2] <- 8\nRETURN A\nENDFUNCTION\nDECLARE B : ARRAY[1:2] OF INTEGER\nB <- Make()\nOUTPUT B[1], \":\", B[2]\n",
    "3:8\n",
    "",
    None,
    &[]
);

case!(
    case_241_case_no_match_without_otherwise,
    "CASE no match without OTHERWISE",
    "DECLARE V : INTEGER\nV <- 8\nCASE OF V\n1 : OUTPUT \"bad\"\nENDCASE\nOUTPUT \"ok\"\n",
    "ok\n",
    "",
    None,
    &[]
);

case!(
    case_242_case_date_range,
    "CASE date range",
    "DECLARE D : DATE\nD <- 29/02/2024\nCASE OF D\n01/01/2024 TO 31/12/2024 : OUTPUT \"ok\"\nOTHERWISE : OUTPUT \"bad\"\nENDCASE\n",
    "ok\n",
    "",
    None,
    &[]
);

case!(
    case_243_case_string,
    "CASE string",
    "DECLARE S : STRING\nS <- \"blue\"\nCASE OF S\n\"red\" : OUTPUT \"bad\"\n\"blue\" : OUTPUT \"ok\"\nENDCASE\n",
    "ok\n",
    "",
    None,
    &[]
);

case!(
    case_244_enum_byref,
    "enum BYREF",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nPROCEDURE Change(BYREF V : Season)\nV <- Winter\nENDPROCEDURE\nS <- Spring\nCALL Change(S)\nOUTPUT S\n",
    "Winter\n",
    "",
    None,
    &[]
);

case!(
    case_245_enum_input_from_child_scope,
    "enum INPUT from child scope",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nPROCEDURE ReadSeason()\nINPUT S\nENDPROCEDURE\nCALL ReadSeason()\nOUTPUT S\n",
    "Summer\n",
    "Summer\n",
    None,
    &[]
);

case!(
    case_246_reject_enum_variable_as_input,
    "reject enum variable as input",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nDECLARE Other : Season\nOther <- Spring\nINPUT S\n",
    "",
    "Other\n",
    Some("variant"),
    &[]
);

case!(
    case_247_typed_array_readfile,
    "typed array READFILE",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", 42\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", A[2]\nCLOSEFILE \"data.txt\"\nOUTPUT A[2]\n",
    "42\n",
    "",
    None,
    &[]
);

case!(
    case_248_enum_readfile,
    "enum READFILE",
    "TYPE Season = (Spring, Summer, Autumn, Winter)\nDECLARE S : Season\nOPENFILE \"data.txt\" FOR WRITE\nWRITEFILE \"data.txt\", Winter\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", S\nCLOSEFILE \"data.txt\"\nOUTPUT S\n",
    "Winter\n",
    "",
    None,
    &[]
);

case!(
    case_249_reject_write_in_read_mode,
    "reject write in READ mode",
    "OPENFILE \"data.txt\" FOR WRITE\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nWRITEFILE \"data.txt\", \"bad\"\n",
    "",
    "",
    Some("writ"),
    &[]
);

case!(
    case_250_reject_reading_beyond_eof,
    "reject reading beyond EOF",
    "DECLARE S : STRING\nOPENFILE \"data.txt\" FOR WRITE\nCLOSEFILE \"data.txt\"\nOPENFILE \"data.txt\" FOR READ\nREADFILE \"data.txt\", S\n",
    "",
    "",
    Some("end"),
    &[]
);

case!(
    case_251_date_input_calendar_29_02_2024,
    "date input calendar 29/02/2024",
    "DECLARE D : DATE\nINPUT D\nOUTPUT D\n",
    "29/02/2024\n",
    "29/02/2024\n",
    None,
    &[]
);

case!(
    case_252_date_input_calendar_29_02_2000,
    "date input calendar 29/02/2000",
    "DECLARE D : DATE\nINPUT D\nOUTPUT D\n",
    "29/02/2000\n",
    "29/02/2000\n",
    None,
    &[]
);

case!(
    case_253_date_input_calendar_31_01_2026,
    "date input calendar 31/01/2026",
    "DECLARE D : DATE\nINPUT D\nOUTPUT D\n",
    "31/01/2026\n",
    "31/01/2026\n",
    None,
    &[]
);

case!(
    case_254_date_input_calendar_30_04_2026,
    "date input calendar 30/04/2026",
    "DECLARE D : DATE\nINPUT D\nOUTPUT D\n",
    "30/04/2026\n",
    "30/04/2026\n",
    None,
    &[]
);

case!(
    case_255_reject_date_calendar_29_02_1900,
    "reject date calendar 29/02/1900",
    "DECLARE D : DATE\nINPUT D\n",
    "",
    "29/02/1900\n",
    Some("date"),
    &[]
);

case!(
    case_256_reject_date_calendar_29_02_2025,
    "reject date calendar 29/02/2025",
    "DECLARE D : DATE\nINPUT D\n",
    "",
    "29/02/2025\n",
    Some("date"),
    &[]
);

case!(
    case_257_reject_date_calendar_31_04_2026,
    "reject date calendar 31/04/2026",
    "DECLARE D : DATE\nINPUT D\n",
    "",
    "31/04/2026\n",
    Some("date"),
    &[]
);

case!(
    case_258_reject_date_calendar_00_01_2026,
    "reject date calendar 00/01/2026",
    "DECLARE D : DATE\nINPUT D\n",
    "",
    "00/01/2026\n",
    Some("date"),
    &[]
);

case!(
    case_259_reject_date_calendar_01_13_2026,
    "reject date calendar 01/13/2026",
    "DECLARE D : DATE\nINPUT D\n",
    "",
    "01/13/2026\n",
    Some("date"),
    &[]
);

case!(
    lcase_unchanged_char_0,
    "LCASE preserves 'a'",
    "DECLARE C : CHAR\nC <- LCASE('a')\nOUTPUT C\n",
    "a\n",
    "",
    None,
    &[]
);

case!(
    lcase_unchanged_char_1,
    "LCASE preserves '7'",
    "DECLARE C : CHAR\nC <- LCASE('7')\nOUTPUT C\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    lcase_unchanged_char_2,
    "LCASE preserves '!'",
    "DECLARE C : CHAR\nC <- LCASE('!')\nOUTPUT C\n",
    "!\n",
    "",
    None,
    &[]
);

case!(
    ucase_unchanged_char_0,
    "UCASE preserves 'A'",
    "DECLARE C : CHAR\nC <- UCASE('A')\nOUTPUT C\n",
    "A\n",
    "",
    None,
    &[]
);

case!(
    ucase_unchanged_char_1,
    "UCASE preserves '7'",
    "DECLARE C : CHAR\nC <- UCASE('7')\nOUTPUT C\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    ucase_unchanged_char_2,
    "UCASE preserves '!'",
    "DECLARE C : CHAR\nC <- UCASE('!')\nOUTPUT C\n",
    "!\n",
    "",
    None,
    &[]
);

case!(
    spec_for_integer_counter,
    "for integer counter",
    "DECLARE I : INTEGER\nFOR I <- 1 TO 2\nOUTPUT I\nNEXT I\n",
    "1\n2\n",
    "",
    None,
    &[]
);

case!(
    spec_reject_call_function,
    "reject call function",
    "FUNCTION F() RETURNS INTEGER\nRETURN 7\nENDFUNCTION\nCALL F()\n",
    "",
    "",
    Some("function"),
    &[]
);

case!(
    spec_reject_procedure_expression,
    "reject procedure expression",
    "PROCEDURE P()\nENDPROCEDURE\nOUTPUT P()\n",
    "",
    "",
    Some("procedure"),
    &[]
);

case!(
    spec_valid_function_and_procedure_calls,
    "valid function and procedure calls",
    "FUNCTION F() RETURNS INTEGER\nRETURN 7\nENDFUNCTION\nPROCEDURE P()\nOUTPUT F()\nENDPROCEDURE\nCALL P()\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    spec_reject_early_otherwise,
    "reject early otherwise",
    "DECLARE X : INTEGER\nX <- 1\nCASE OF X\nOTHERWISE : OUTPUT \"default\"\n1 : OUTPUT \"one\"\nENDCASE\n",
    "",
    "",
    Some("otherwise"),
    &[]
);

case!(
    spec_reject_duplicate_otherwise,
    "reject duplicate otherwise",
    "DECLARE X : INTEGER\nCASE OF X\nOTHERWISE : OUTPUT \"first\"\nOTHERWISE : OUTPUT \"second\"\nENDCASE\n",
    "",
    "",
    Some("otherwise"),
    &[]
);

case!(
    spec_reject_extra_array_index_read,
    "reject extra array index read",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nOUTPUT A[1,999]\n",
    "",
    "",
    Some("array"),
    &[]
);

case!(
    spec_reject_extra_array_index_write,
    "reject extra array index write",
    "DECLARE A : ARRAY[1:2] OF INTEGER\nA[1,999] <- 7\n",
    "",
    "",
    Some("array"),
    &[]
);

case!(
    spec_reject_array_byval_size,
    "reject array byval size",
    "PROCEDURE P(BYVAL A : ARRAY[1:2] OF INTEGER)\nENDPROCEDURE\nDECLARE B : ARRAY[1:3] OF INTEGER\nCALL P(B)\n",
    "",
    "",
    Some("array"),
    &[]
);

case!(
    spec_reject_array_byval_dimensions,
    "reject array byval dimensions",
    "PROCEDURE P(BYVAL A : ARRAY[1:2] OF INTEGER)\nENDPROCEDURE\nDECLARE B : ARRAY[1:2,1:2] OF INTEGER\nCALL P(B)\n",
    "",
    "",
    Some("array"),
    &[]
);

case!(
    spec_reject_array_byref_size,
    "reject array byref size",
    "PROCEDURE P(BYREF A : ARRAY[1:2] OF INTEGER)\nENDPROCEDURE\nDECLARE B : ARRAY[1:3] OF INTEGER\nCALL P(B)\n",
    "",
    "",
    Some("array"),
    &[]
);

case!(
    spec_reject_array_byref_dimensions,
    "reject array byref dimensions",
    "PROCEDURE P(BYREF A : ARRAY[1:2] OF INTEGER)\nENDPROCEDURE\nDECLARE B : ARRAY[1:2,1:2] OF INTEGER\nCALL P(B)\n",
    "",
    "",
    Some("array"),
    &[]
);

case!(
    spec_reject_array_return_size,
    "reject array return size",
    "FUNCTION F() RETURNS ARRAY[1:2] OF INTEGER\nDECLARE B : ARRAY[1:3] OF INTEGER\nRETURN B\nENDFUNCTION\nPROCEDURE P(A : ARRAY[1:3] OF INTEGER)\nENDPROCEDURE\nCALL P(F())\n",
    "",
    "",
    Some("return"),
    &[]
);

case!(
    spec_reject_array_return_dimensions,
    "reject array return dimensions",
    "FUNCTION F() RETURNS ARRAY[1:2] OF INTEGER\nDECLARE B : ARRAY[1:2,1:2] OF INTEGER\nRETURN B\nENDFUNCTION\nPROCEDURE P(A : ARRAY[1:2,1:2] OF INTEGER)\nENDPROCEDURE\nCALL P(F())\n",
    "",
    "",
    Some("return"),
    &[]
);

case!(
    spec_eof_variable_filename,
    "EOF with a variable filename",
    "DECLARE Name : STRING\nDECLARE Line : STRING\nName <- \"data.txt\"\nOPENFILE Name FOR WRITE\nWRITEFILE Name, \"alpha\"\nWRITEFILE Name, \"omega\"\nCLOSEFILE Name\nOPENFILE Name FOR READ\nOUTPUT EOF(Name)\nWHILE NOT EOF(Name)\nREADFILE Name, Line\nOUTPUT Line\nENDWHILE\nOUTPUT EOF(Name)\nCLOSEFILE Name\n",
    "FALSE\nalpha\nomega\nTRUE\n",
    "",
    None,
    &[("data.txt", "alpha\nomega\n")]
);

case!(
    spec_eof_expression_filename,
    "EOF with an expression filename",
    "DECLARE Stem : STRING\nDECLARE Line : STRING\nStem <- \"data\"\nOPENFILE Stem & \".txt\" FOR WRITE\nWRITEFILE Stem & \".txt\", \"alpha\"\nCLOSEFILE Stem & \".txt\"\nOPENFILE Stem & \".txt\" FOR READ\nWHILE NOT EOF(Stem & \".txt\")\nREADFILE Stem & \".txt\", Line\nOUTPUT Line\nENDWHILE\nOUTPUT EOF(Stem & \".txt\")\nCLOSEFILE Stem & \".txt\"\n",
    "alpha\nTRUE\n",
    "",
    None,
    &[("data.txt", "alpha\n")]
);

case!(
    spec_eof_function_call_filename,
    "EOF with a function call filename",
    "FUNCTION Chosen() RETURNS STRING\nRETURN \"data.txt\"\nENDFUNCTION\nOPENFILE Chosen() FOR WRITE\nWRITEFILE Chosen(), \"alpha\"\nCLOSEFILE Chosen()\nOPENFILE Chosen() FOR READ\nOUTPUT EOF(Chosen())\nCLOSEFILE Chosen()\n",
    "FALSE\n",
    "",
    None,
    &[("data.txt", "alpha\n")]
);

case!(
    spec_reject_eof_non_string_filename,
    "reject EOF with a non-string filename",
    "DECLARE N : INTEGER\nN <- 5\nOUTPUT EOF(N)\n",
    "",
    "",
    Some("must evaluate to a string"),
    &[]
);

case!(
    spec_identifier_letters_digits_underscore,
    "identifier with digits and underscore",
    "DECLARE Total_2 : INTEGER\nTotal_2 <- 5\nOUTPUT Total_2\n",
    "5\n",
    "",
    None,
    &[]
);

case!(
    spec_reject_identifier_starting_with_digit,
    "reject identifier starting with a digit",
    "DECLARE 2Bad : INTEGER\n",
    "",
    "",
    Some("lexical"),
    &[]
);

case!(
    spec_reject_keyword_as_identifier,
    "reject a keyword used as an identifier",
    "DECLARE WHILE : INTEGER\n",
    "",
    "",
    Some("identifier"),
    &[]
);

case!(
    spec_concat_binds_looser_than_addition,
    "& binds looser than +",
    "OUTPUT \"n=\" & 1 + 2\n",
    "n=3\n",
    "",
    None,
    &[]
);

case!(
    spec_concat_binds_tighter_than_comparison,
    "& binds tighter than =",
    "OUTPUT \"a\" & \"b\" = \"ab\"\n",
    "TRUE\n",
    "",
    None,
    &[]
);

case!(
    spec_div_mod_identity_positive,
    "DIV and MOD reconstruct the dividend",
    "OUTPUT (7 DIV 2) * 2 + (7 MOD 2)\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    spec_exact_division_assignable_to_integer,
    "exact division assigns to INTEGER",
    "DECLARE X : INTEGER\nX <- 4 / 2\nOUTPUT X\n",
    "2\n",
    "",
    None,
    &[]
);

case!(
    spec_for_bounds_evaluated_once,
    "FOR bounds are evaluated once",
    "DECLARE n : INTEGER\nDECLARE i : INTEGER\nn <- 3\nFOR i <- 1 TO n\nn <- 10\nOUTPUT i\nNEXT i\n",
    "1\n2\n3\n",
    "",
    None,
    &[]
);

case!(
    spec_for_step_overshoot_terminates,
    "FOR STEP terminates on overshoot",
    "DECLARE i : INTEGER\nFOR i <- 1 TO 10 STEP 4\nOUTPUT i\nNEXT i\n",
    "1\n5\n9\n",
    "",
    None,
    &[]
);

case!(
    spec_for_negative_step,
    "FOR with a negative STEP counts down",
    "DECLARE i : INTEGER\nFOR i <- 5 TO 1 STEP -1\nOUTPUT i\nNEXT i\n",
    "5\n4\n3\n2\n1\n",
    "",
    None,
    &[]
);

case!(
    spec_for_equal_bounds_runs_once,
    "FOR runs once when value1 = value2",
    "DECLARE i : INTEGER\nFOR i <- 4 TO 4\nOUTPUT i\nNEXT i\nOUTPUT \"done\"\n",
    "4\ndone\n",
    "",
    None,
    &[]
);

case!(
    spec_for_counter_write_does_not_derail,
    "writing the FOR counter does not derail the loop",
    "DECLARE i : INTEGER\nFOR i <- 1 TO 3\nOUTPUT i\ni <- 99\nNEXT i\n",
    "1\n2\n3\n",
    "",
    None,
    &[]
);

case!(
    spec_for_expression_bounds,
    "FOR bounds may be expressions",
    "DECLARE i : INTEGER\nFOR i <- 1 + 1 TO 2 * 3\nOUTPUT i\nNEXT i\n",
    "2\n3\n4\n5\n6\n",
    "",
    None,
    &[]
);

case!(
    spec_while_false_runs_zero_times,
    "WHILE runs zero times when false first",
    "DECLARE X : INTEGER\nX <- 0\nWHILE X > 5\nOUTPUT \"no\"\nENDWHILE\nOUTPUT \"done\"\n",
    "done\n",
    "",
    None,
    &[]
);

case!(
    spec_repeat_runs_at_least_once,
    "REPEAT runs once even when already true",
    "DECLARE X : INTEGER\nX <- 99\nREPEAT\nOUTPUT \"once\"\nUNTIL X > 5\n",
    "once\n",
    "",
    None,
    &[]
);

case!(
    spec_case_first_match_wins,
    "CASE takes the first matching clause",
    "DECLARE X : INTEGER\nX <- 5\nCASE OF X\n1 TO 9 : OUTPUT \"first\"\n5 : OUTPUT \"second\"\nENDCASE\n",
    "first\n",
    "",
    None,
    &[]
);

case!(
    spec_reject_byref_in_function,
    "reject BYREF on a function parameter",
    "FUNCTION F(BYREF X : INTEGER) RETURNS INTEGER\nRETURN X\nENDFUNCTION\nDECLARE A : INTEGER\nOUTPUT F(A)\n",
    "",
    "",
    Some("reference"),
    &[]
);

case!(
    spec_byref_applies_to_following_parameters,
    "BYREF carries to later parameters",
    "PROCEDURE SWAP(BYREF X : INTEGER, Y : INTEGER)\nDECLARE T : INTEGER\nT <- X\nX <- Y\nY <- T\nENDPROCEDURE\nDECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 1\nB <- 2\nCALL SWAP(A, B)\nOUTPUT A, \" \", B\n",
    "2 1\n",
    "",
    None,
    &[]
);

case!(
    spec_global_readable_in_procedure,
    "a procedure reads a global",
    "DECLARE G : INTEGER\nG <- 7\nPROCEDURE P()\nOUTPUT G\nENDPROCEDURE\nCALL P()\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    spec_global_writable_in_procedure,
    "a procedure writes a global",
    "DECLARE G : INTEGER\nG <- 1\nPROCEDURE P()\nG <- 2\nENDPROCEDURE\nCALL P()\nOUTPUT G\n",
    "2\n",
    "",
    None,
    &[]
);

case!(
    spec_local_does_not_leak_from_procedure,
    "a procedure local does not leak out",
    "PROCEDURE P()\nDECLARE L : INTEGER\nL <- 1\nENDPROCEDURE\nCALL P()\nOUTPUT L\n",
    "",
    "",
    Some("undefined"),
    &[]
);

case!(
    spec_output_mixed_operand_types,
    "OUTPUT joins operands of mixed types",
    "DECLARE N : INTEGER\nN <- 3\nOUTPUT \"a\", N, TRUE, 1.5, 'x'\n",
    "a3TRUE1.5x\n",
    "",
    None,
    &[]
);

case!(
    spec_empty_string_and_length,
    "the empty string has length zero",
    "DECLARE S : STRING\nS <- \"\"\nOUTPUT \"[\", S, \"] len=\", LENGTH(S)\n",
    "[] len=0\n",
    "",
    None,
    &[]
);

case!(
    spec_guide_string_function_examples,
    "the guide's own string function examples",
    "OUTPUT RIGHT(\"ABCDEFGH\", 3)\nOUTPUT LENGTH(\"Happy Days\")\nOUTPUT MID(\"ABCDEFGH\", 2, 3)\nOUTPUT \"Summer\" & \" \" & \"Pudding\"\n",
    "FGH\n10\nBCD\nSummer Pudding\n",
    "",
    None,
    &[]
);

case!(
    spec_append_creates_missing_file,
    "APPEND creates a file that does not exist",
    "DECLARE L : STRING\nOPENFILE \"d.txt\" FOR APPEND\nWRITEFILE \"d.txt\", \"x\"\nCLOSEFILE \"d.txt\"\nOPENFILE \"d.txt\" FOR READ\nREADFILE \"d.txt\", L\nCLOSEFILE \"d.txt\"\nOUTPUT L\n",
    "x\n",
    "",
    None,
    &[("d.txt", "x\n")]
);

case!(
    spec_reject_opening_an_open_file,
    "reject opening a file that is already open",
    "OPENFILE \"d.txt\" FOR WRITE\nOPENFILE \"d.txt\" FOR READ\n",
    "",
    "",
    Some("already open"),
    &[]
);

case!(
    spec_reject_closing_unopened_file,
    "reject closing a file that is not open",
    "CLOSEFILE \"d.txt\"\n",
    "",
    "",
    Some("not open"),
    &[]
);

case!(
    spec_reject_eof_on_write_mode_file,
    "reject EOF on a file opened for WRITE",
    "OPENFILE \"d.txt\" FOR WRITE\nOUTPUT EOF(\"d.txt\")\n",
    "",
    "",
    Some("read mode"),
    &[]
);

case!(
    spec_variable_name_is_case_insensitive,
    "a variable is reached in any case",
    "DECLARE Num1 : INTEGER\nNum1 <- 2\nOUTPUT num1\n",
    "2\n",
    "",
    None,
    &[]
);

case!(
    spec_array_name_is_case_insensitive,
    "an array is reached in any case",
    "DECLARE Arr : ARRAY[1:2] OF INTEGER\nARR[1] <- 9\nOUTPUT arr[1]\n",
    "9\n",
    "",
    None,
    &[]
);

case!(
    spec_constant_name_is_case_insensitive,
    "a constant is reached in any case",
    "CONSTANT Max = 5\nOUTPUT mAx\n",
    "5\n",
    "",
    None,
    &[]
);

case!(
    spec_procedure_name_is_case_insensitive,
    "a procedure is called in any case",
    "PROCEDURE Foo()\nOUTPUT \"hi\"\nENDPROCEDURE\nCALL foo()\n",
    "hi\n",
    "",
    None,
    &[]
);

case!(
    spec_function_name_is_case_insensitive,
    "a function is called in any case",
    "FUNCTION Bar() RETURNS INTEGER\nRETURN 7\nENDFUNCTION\nOUTPUT bAr()\n",
    "7\n",
    "",
    None,
    &[]
);

case!(
    spec_type_name_is_case_insensitive,
    "a type name is used in any case",
    "TYPE Season = (Spring, Summer)\nDECLARE S : season\nS <- SPRING\nOUTPUT S\n",
    "Spring\n",
    "",
    None,
    &[]
);

case!(
    spec_for_counter_is_case_insensitive,
    "a FOR counter is the same variable in any case",
    "DECLARE i : INTEGER\nFOR I <- 1 TO 2\nOUTPUT i\nNEXT I\n",
    "1\n2\n",
    "",
    None,
    &[]
);

case!(
    spec_byref_argument_is_case_insensitive,
    "a BYREF argument is matched in any case",
    "PROCEDURE P(BYREF X : INTEGER)\nX <- 9\nENDPROCEDURE\nDECLARE A : INTEGER\nCALL P(a)\nOUTPUT A\n",
    "9\n",
    "",
    None,
    &[]
);

case!(
    spec_enum_input_is_case_insensitive,
    "enum input is case insensitive and stored as declared",
    "TYPE Season = (Spring, Summer)\nDECLARE S : Season\nINPUT S\nOUTPUT S\nOUTPUT S = Summer\n",
    "Summer\nTRUE\n",
    "summer\n",
    None,
    &[]
);

case!(
    spec_enum_input_uppercase_stored_as_declared,
    "enum input in upper case is stored as declared",
    "TYPE Season = (Spring, Summer)\nDECLARE S : Season\nINPUT S\nOUTPUT S\n",
    "Summer\n",
    "SUMMER\n",
    None,
    &[]
);

case!(
    spec_reject_enum_variants_differing_only_by_case,
    "reject enum variants differing only by case",
    "TYPE E = (Red, RED)\n",
    "",
    "",
    Some("twice"),
    &[]
);

case!(
    spec_reject_constant_mutation_in_another_case,
    "reject writing a constant spelled in another case",
    "CONSTANT Max = 5\nmax <- 6\n",
    "",
    "",
    Some("constant"),
    &[]
);

case!(
    spec_filenames_remain_case_sensitive,
    "file names are strings, not identifiers",
    "OPENFILE \"Data.txt\" FOR WRITE\nCLOSEFILE \"data.txt\"\n",
    "",
    "",
    Some("not open"),
    &[]
);

case!(
    spec_for_counter_is_scoped_to_the_loop,
    "a counter the loop declared can be declared after it",
    "FOR idx <- 1 TO 10\nNEXT idx\n\nDECLARE Idx : INTEGER\nIdx <- 5\nOUTPUT Idx\n",
    "5\n",
    "",
    None,
    &[]
);

case!(
    spec_reject_reading_implicit_counter_after_loop,
    "a counter the loop declared does not outlive it",
    "FOR idx <- 1 TO 3\nNEXT idx\nOUTPUT idx\n",
    "",
    "",
    Some("undefined"),
    &[]
);

case!(
    spec_declared_counter_survives_the_loop,
    "a counter declared before the loop outlives it",
    "DECLARE Idx : INTEGER\nFOR Idx <- 1 TO 3\nNEXT Idx\nOUTPUT Idx\n",
    "3\n",
    "",
    None,
    &[]
);

case!(
    spec_two_implicit_loops_share_a_counter_name,
    "two loops may each declare the same counter",
    "FOR i <- 1 TO 2\nOUTPUT i\nNEXT i\nFOR i <- 5 TO 6\nOUTPUT i\nNEXT i\n",
    "1\n2\n5\n6\n",
    "",
    None,
    &[]
);

case!(
    spec_implicit_counter_shadows_a_global,
    "a loop counter shadows a global and leaves it alone",
    "DECLARE i : INTEGER\ni <- 100\nPROCEDURE P()\nFOR i <- 1 TO 3\nNEXT i\nENDPROCEDURE\nCALL P()\nOUTPUT i\n",
    "100\n",
    "",
    None,
    &[]
);

case!(
    spec_reject_duplicate_declaration,
    "reject two declarations of one name",
    "DECLARE Total : INTEGER\nDECLARE total : STRING\n",
    "",
    "",
    Some("already been declared"),
    &[]
);

case!(
    spec_reject_duplicate_parameter_names,
    "reject two parameters sharing a name",
    "PROCEDURE P(Value : INTEGER, VALUE : STRING)\nENDPROCEDURE\nCALL P(1, \"a\")\n",
    "",
    "",
    Some("error"),
    &[]
);

case!(
    spec_reject_declaration_colliding_with_constant,
    "reject a declaration colliding with a constant",
    "CONSTANT Max = 5\nDECLARE MAX : INTEGER\n",
    "",
    "",
    Some("constant"),
    &[]
);

case!(
    spec_shadowing_a_global_remains_allowed,
    "a local may shadow a global of the same name",
    "DECLARE X : INTEGER\nX <- 1\nPROCEDURE P()\nDECLARE X : STRING\nX <- \"in\"\nOUTPUT X\nENDPROCEDURE\nCALL P()\nOUTPUT X\n",
    "in\n1\n",
    "",
    None,
    &[]
);

case!(
    spec_div_truncates_negative_quotient,
    "DIV truncates toward zero, -7 DIV 2 is -3",
    "DECLARE A : INTEGER\nA <- 0 - 7\nOUTPUT A DIV 2\n",
    "-3\n",
    "",
    None,
    &[]
);

case!(
    spec_div_truncates_negative_quotient_wider,
    "DIV truncates toward zero, -9 DIV 4 is -2",
    "DECLARE A : INTEGER\nA <- 0 - 9\nOUTPUT A DIV 4\n",
    "-2\n",
    "",
    None,
    &[]
);

case!(
    spec_div_mod_identity_holds_for_negatives,
    "(A DIV B) * B + (A MOD B) reconstructs A",
    "DECLARE A : INTEGER\nA <- 0 - 7\nOUTPUT (A DIV 2) * 2 + (A MOD 2)\n",
    "-7\n",
    "",
    None,
    &[]
);

case!(
    spec_integer_multiply_overflow_is_reported,
    "multiplication past INTEGER range is an error",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 4000000000\nB <- 4000000000\nOUTPUT A * B\n",
    "",
    "",
    Some("overflow"),
    &[]
);

case!(
    spec_integer_add_overflow_is_reported,
    "addition past INTEGER range is an error",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 9000000000000000000\nB <- 9000000000000000000\nOUTPUT A + B\n",
    "",
    "",
    Some("overflow"),
    &[]
);

case!(
    negative_integer_exponent_does_not_crash,
    "a negative INTEGER exponent is handled, not a panic",
    "DECLARE A : INTEGER\nDECLARE B : INTEGER\nA <- 2\nB <- 0 - 1\nOUTPUT A ^ B\n",
    "",
    "",
    Some("error"),
    &[]
);

// ignore these two cases, this is a feature in the lagnauge, not a bug.
// known_bug!(
//     known_bug_reject_builtin_redefinition_lowercase,
//     "a builtin cannot be redefined in another case",
//     "FUNCTION length(S : STRING) RETURNS INTEGER\nRETURN 99\nENDFUNCTION\n",
//     "",
//     "",
//     Some("builtin"),
//     &[]
// );
//
// known_bug!(
//     known_bug_reject_builtin_redefinition_mixed_case,
//     "a builtin cannot be shadowed by a procedure in another case",
//     "PROCEDURE Length()\nENDPROCEDURE\n",
//     "",
//     "",
//     Some("builtin"),
//     &[]
// );

case!(
    duplicate_parameters_rejected_at_definition,
    "two parameters sharing a name are rejected at the definition",
    "PROCEDURE P(Value : INTEGER, VALUE : STRING)\nENDPROCEDURE\nOUTPUT \"defined\"\n",
    "",
    "",
    Some("error"),
    &[]
);
