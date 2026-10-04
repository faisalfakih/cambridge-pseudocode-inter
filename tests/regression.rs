//! Run with `cargo test --test regression`; append a test-name filter to narrow the run.
//! Covers the implemented 2026-guide subset and interpreter extensions.
//! Records, pointers, sets, OOP, random-access files and CLI flags are excluded.
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

// DECLARE naming several identifiers of one type on a single line

case!(
    spec_single_line_declare_three_integers,
    "one DECLARE naming three integers",
    "DECLARE A, B, C : INTEGER\nA <- 1\nB <- 2\nC <- 3\nOUTPUT A, B, C\n",
    "123\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_gives_every_name_a_default,
    "every name in the list starts at its type's default",
    "DECLARE A, B, C : INTEGER\nOUTPUT A, B, C\n",
    "000\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_stores_names_separately,
    "writing one name in the list leaves the others alone",
    "DECLARE A, B : INTEGER\nA <- 1\nB <- 2\nA <- 9\nOUTPUT A, \" \", B\n",
    "9 2\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_string,
    "one DECLARE naming two strings",
    "DECLARE X, Y : STRING\nX <- \"a\"\nY <- \"b\"\nOUTPUT X & Y\n",
    "ab\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_real,
    "one DECLARE naming two reals",
    "DECLARE P, Q : REAL\nP <- 1.5\nOUTPUT P, \" \", Q\n",
    "1.5 0\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_boolean,
    "one DECLARE naming two booleans",
    "DECLARE P, Q : BOOLEAN\nP <- TRUE\nOUTPUT P, \" \", Q\n",
    "TRUE FALSE\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_char,
    "one DECLARE naming two chars",
    "DECLARE P, Q : CHAR\nP <- 'a'\nQ <- 'b'\nOUTPUT P, Q\n",
    "ab\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_date,
    "one DECLARE naming two dates",
    "DECLARE P, Q : DATE\nP <- 05/03/2026\nOUTPUT P, \" \", Q\n",
    "05/03/2026 01/01/1900\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_array,
    "one DECLARE naming two arrays of the same bounds",
    "DECLARE A, B : ARRAY[1:2] OF INTEGER\nA[1] <- 7\nB[1] <- 8\nOUTPUT A[1], \" \", B[1]\n",
    "7 8\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_enum_type,
    "one DECLARE naming two variables of an enumerated type",
    "TYPE Season = (Spring, Summer)\nDECLARE A, B : Season\nA <- Spring\nB <- Summer\nOUTPUT A, \" \", B\n",
    "Spring Summer\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_six_names,
    "a longer list of names",
    "DECLARE A, B, C, D, E, F : INTEGER\nF <- 6\nOUTPUT A, F\n",
    "06\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_inside_a_procedure,
    "a list of names declared in a procedure's scope",
    "PROCEDURE P()\nDECLARE L, M : INTEGER\nL <- 1\nM <- 2\nOUTPUT L, M\nENDPROCEDURE\nCALL P()\n",
    "12\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_names_are_case_insensitive,
    "names from a list are reached in any case",
    "DECLARE Alpha, Beta : INTEGER\nALPHA <- 1\nOUTPUT beta, alpha\n",
    "01\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_counter_survives_a_for_loop,
    "a counter declared in a list outlives its loop",
    "DECLARE i, n : INTEGER\nn <- 3\nFOR i <- 1 TO n\nOUTPUT i\nNEXT i\nOUTPUT \"after=\", i\n",
    "1\n2\n3\nafter=3\n",
    "",
    None,
    &[]
);

case!(
    spec_single_line_declare_type_applies_to_every_name,
    "the declared type applies to every name in the list",
    "DECLARE A, B : INTEGER\nB <- \"str\"\n",
    "",
    "",
    Some("type mismatch"),
    &[]
);

case!(
    spec_reject_single_line_declare_repeating_a_name,
    "reject a name listed twice in one DECLARE",
    "DECLARE A, A : INTEGER\n",
    "",
    "",
    Some("already been declared"),
    &[]
);

case!(
    spec_reject_single_line_declare_repeating_a_name_in_another_case,
    "reject names in one DECLARE differing only by case",
    "DECLARE Total, TOTAL : INTEGER\n",
    "",
    "",
    Some("already been declared"),
    &[]
);

case!(
    spec_reject_single_line_declare_colliding_with_an_earlier_one,
    "reject a list containing an already declared name",
    "DECLARE A : INTEGER\nDECLARE B, A : STRING\n",
    "",
    "",
    Some("already been declared"),
    &[]
);

case!(
    spec_reject_single_line_declare_colliding_with_a_constant,
    "reject a list containing a constant's name",
    "CONSTANT Max = 5\nDECLARE A, MAX : INTEGER\n",
    "",
    "",
    Some("constant"),
    &[]
);

case!(
    spec_reject_single_line_declare_trailing_comma,
    "reject a trailing comma in a DECLARE list",
    "DECLARE A, B, : INTEGER\n",
    "",
    "",
    Some("identifier after comma"),
    &[]
);

case!(
    spec_reject_single_line_declare_leading_comma,
    "reject a leading comma in a DECLARE list",
    "DECLARE , A : INTEGER\n",
    "",
    "",
    Some("identifier after declare"),
    &[]
);

case!(
    spec_reject_single_line_declare_repeated_comma,
    "reject two commas in a row in a DECLARE list",
    "DECLARE A,, B : INTEGER\n",
    "",
    "",
    Some("identifier after comma"),
    &[]
);

case!(
    spec_reject_single_line_declare_missing_colon,
    "reject a DECLARE list with no colon before the type",
    "DECLARE A, B INTEGER\n",
    "",
    "",
    Some("colon"),
    &[]
);

case!(
    spec_reject_single_line_declare_keyword_in_the_list,
    "reject a keyword used as a name in a DECLARE list",
    "DECLARE A, WHILE : INTEGER\n",
    "",
    "",
    Some("identifier after comma"),
    &[]
);

#[test]
fn builtin_setdate_valid_calendar_dates() {
    for (args, expected) in [
        ("29, 2, 2024", "29/02/2024"),
        ("29, 2, 2000", "29/02/2000"),
        ("28, 2, 1900", "28/02/1900"),
        ("30, 4, 2024", "30/04/2024"),
        ("31, 12, 9999", "31/12/9999"),
        ("1, 1, 1", "01/01/0001"),
    ] {
        run_case(
            args,
            &format!("DECLARE D : DATE\nD <- SETDATE({args})\nOUTPUT D\nOUTPUT D = {expected}\n"),
            &format!("{expected}\nTRUE\n"),
            "",
            None,
            &[],
        );
    }
}

#[test]
fn builtin_setdate_rejects_invalid_calendar_dates() {
    for args in [
        "29, 2, 2025",
        "29, 2, 1900",
        "29, 2, 2100",
        "31, 4, 2024",
        "0, 1, 2024",
        "32, 1, 2024",
        "1, 0, 2024",
        "1, 13, 2024",
        "-1, 1, 2024",
        "1, -1, 2024",
        "1, 1, -1",
        "1, 1, 10000",
        "65537, 1, 2024",
        "1, 65537, 2024",
        "1, 1, 67560",
    ] {
        run_case(
            args,
            &format!("OUTPUT SETDATE({args})\n"),
            "",
            "",
            Some("SETDATE"),
            &[],
        );
    }
}

#[test]
fn builtin_setdate_requires_three_arguments() {
    for args in ["", "1", "1, 2", "1, 2, 2024, 4"] {
        run_case(
            args,
            &format!("OUTPUT SETDATE({args})\n"),
            "",
            "",
            Some("expects exactly 3 argument(s)"),
            &[],
        );
    }
}

#[test]
fn builtin_setdate_requires_integer_arguments() {
    for args in [
        "1.5, 1, 2024",
        "1, 1.5, 2024",
        "1, 1, 2024.5",
        "TRUE, 1, 2024",
        "1, \"1\", 2024",
        "1, 1, 'x'",
    ] {
        run_case(
            args,
            &format!("OUTPUT SETDATE({args})\n"),
            "",
            "",
            Some("must be an integer"),
            &[],
        );
    }
}

#[test]
fn builtin_today_returns_local_date() {
    use cambridge_pseudocode_interpreter::Inter::{
        builtins::call_builtin,
        cps::{Date, Value},
    };

    let before = chrono::Local::now().format("%d/%m/%Y").to_string();
    let result = call_builtin("TODAY".to_string(), &[]).unwrap();
    let after = chrono::Local::now().format("%d/%m/%Y").to_string();
    match result {
        Some(Value::Date(date)) => {
            assert!(date == Date::parse(&before).unwrap() || date == Date::parse(&after).unwrap())
        }
        other => panic!("TODAY must return DATE, got {other:?}"),
    }
}

case!(
    builtin_today_date_operations,
    "TODAY can be assigned, compared and passed to date built-ins",
    "DECLARE D : DATE\nD <- TODAY()\nOUTPUT D = SETDATE(DAY(D), MONTH(D), YEAR(D))\nOUTPUT DAY(D) >= 1 AND DAY(D) <= 31\nOUTPUT MONTH(D) >= 1 AND MONTH(D) <= 12\n",
    "TRUE\nTRUE\nTRUE\n",
    "",
    None,
    &[]
);

#[test]
fn builtin_today_rejects_arguments() {
    for args in ["1", "1, 2", "1, 2, 2024"] {
        run_case(
            args,
            &format!("OUTPUT TODAY({args})\n"),
            "",
            "",
            Some("expects exactly 0 argument(s)"),
            &[],
        );
    }
}

#[test]
fn builtin_today_native_timezones() {
    for (zone, seconds) in [("Etc/GMT-14", 14 * 3600), ("Etc/GMT+12", -12 * 3600)] {
        let directory = TestDirectory::new();
        fs::write(directory.0.join("case.cps"), "OUTPUT TODAY()\n").unwrap();
        let offset = chrono::FixedOffset::east_opt(seconds).unwrap();
        let before = chrono::Utc::now()
            .with_timezone(&offset)
            .format("%d/%m/%Y\n")
            .to_string();
        let result = Command::new(env!("CARGO_BIN_EXE_cps"))
            .arg("case.cps")
            .current_dir(&directory.0)
            .env("TZ", zone)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let after = chrono::Utc::now()
            .with_timezone(&offset)
            .format("%d/%m/%Y\n")
            .to_string();
        assert!(
            result.status.success(),
            "{zone}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let actual = String::from_utf8(result.stdout).unwrap();
        assert!(
            actual == before || actual == after,
            "{zone}: expected {before:?} or {after:?}, got {actual:?}"
        );
    }
}
// New built-ins: direct value/type checks, CLI integration and the web replay engine.
mod new_builtins {
    use super::*;
    use cambridge_pseudocode_interpreter::{
        errortype::ErrorType,
        Inter::{
            builtins::call_builtin,
            cps::{Date, Value},
            interpreter::{Interpreter, ReplayContext},
            step_interpreter::{StepEvent, StepInterpreter},
        },
        Lexer::lexer::Lexer,
        Parser::parser::Parser,
    };
    use chrono::{Datelike, NaiveDate};
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    fn call(name: &str, args: &[Value]) -> Value {
        call_builtin(name.to_owned(), args)
            .unwrap_or_else(|error| panic!("{name}({args:?}): {error}"))
            .unwrap_or_else(|| panic!("{name}({args:?}) did not return a value"))
    }

    fn reject(name: &str, args: &[Value], message: &str) {
        let error = call_builtin(name.to_owned(), args)
            .expect_err(&format!("{name}({args:?}) must be rejected"));
        assert!(
            matches!(error.error_type, ErrorType::Runtime),
            "{name}: {error:?}"
        );
        assert!(
            error.message.contains(name) && error.message.contains(message),
            "{name}({args:?}): expected {message:?}, got {:?}",
            error.message
        );
    }

    fn date_value(date: NaiveDate) -> Value {
        Value::Date(Date {
            day: date.day() as u16,
            month: date.month() as u16,
            year: date.year() as u16,
        })
    }

    #[test]
    fn left_all_prefixes_and_clipping() {
        for text in [
            "",
            "x",
            "Cambridge",
            " a b ",
            "café🙂東京",
            "e\u{301}",
            "\0a\nb",
        ] {
            let chars: Vec<_> = text.chars().collect();
            for length in 0..=chars.len() + 3 {
                let expected = chars[..length.min(chars.len())].iter().collect::<String>();
                assert_eq!(
                    call(
                        "LEFT",
                        &[
                            Value::String(text.to_owned()),
                            Value::Integer(length as i64)
                        ]
                    ),
                    Value::String(expected),
                    "LEFT({text:?}, {length})"
                );
            }
            assert_eq!(
                call(
                    "LEFT",
                    &[Value::String(text.to_owned()), Value::Integer(i64::MAX)]
                ),
                Value::String(text.to_owned()),
                "LEFT({text:?}, maximum INTEGER)"
            );
        }
    }

    #[test]
    fn left_char_input_still_returns_string() {
        for ch in ['A', 'é', '🙂', '\0'] {
            for length in [0, 1, 2] {
                assert_eq!(
                    call("LEFT", &[Value::Char(ch), Value::Integer(length)]),
                    Value::String(if length == 0 {
                        String::new()
                    } else {
                        ch.to_string()
                    })
                );
            }
        }
    }

    #[test]
    fn left_integral_real_length_extension() {
        for length in [0.0, 1.0, 3.0, 10.0] {
            assert_eq!(
                call(
                    "LEFT",
                    &[Value::String("abc".to_owned()), Value::Real(length)]
                ),
                Value::String("abc".chars().take(length as usize).collect())
            );
        }
    }

    #[test]
    fn left_rejects_nontext_first_argument() {
        for value in [
            Value::Integer(2),
            Value::Real(1.5),
            Value::Boolean(true),
            date_value(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            Value::Array {
                array: vec![Value::Char('a')],
                lower_bound: 1,
                bounds_2d: None,
            },
            Value::Enum {
                type_name: "Season".to_owned(),
                variant: Some("Spring".to_owned()),
            },
        ] {
            reject(
                "LEFT",
                &[value, Value::Integer(1)],
                "argument 1 must be a string",
            );
        }
    }

    #[test]
    fn left_negative_length_diagnostic() {
        for length in [-1, -100, i64::MIN] {
            reject(
                "LEFT",
                &[Value::String("abc".to_owned()), Value::Integer(length)],
                "non-negative",
            );
        }
    }

    #[test]
    fn left_invalid_length_type_diagnostic() {
        for length in [
            Value::Real(1.5),
            Value::Real(f64::NAN),
            Value::Real(f64::INFINITY),
            Value::Boolean(false),
            Value::String("1".to_owned()),
            Value::Char('1'),
        ] {
            reject(
                "LEFT",
                &[Value::String("abc".to_owned()), length],
                "argument 2 must be an integer",
            );
        }
    }

    #[test]
    fn left_composes_with_case_and_slicing() {
        run_case(
            "LEFT in expressions, parameters and returns",
            "\
FUNCTION Prefix(S : STRING, N : INTEGER) RETURNS STRING
    RETURN LEFT(S, N)
ENDFUNCTION
DECLARE S : STRING
DECLARE N : INTEGER
S <- \"aBcDé\"
N <- 3
OUTPUT Prefix(S, N)
OUTPUT UCASE(LEFT(S, N))
OUTPUT LCASE(LEFT(S, N))
OUTPUT LEFT(MID(S, 2, 3), 2)
OUTPUT LENGTH(LEFT(S, 0))
OUTPUT S
",
            "aBc\nABC\nabc\nBc\n0\naBcDé\n",
            "",
            None,
            &[],
        );
    }

    #[test]
    fn every_new_builtin_checks_arity_before_indexing_arguments() {
        for (name, arity) in [
            ("LEFT", 2),
            ("DAY", 1),
            ("MONTH", 1),
            ("YEAR", 1),
            ("DAYINDEX", 1),
            ("SETDATE", 3),
            ("TODAY", 0),
        ] {
            for count in 0..=5 {
                if count == arity {
                    continue;
                }
                reject(
                    name,
                    &vec![Value::Boolean(false); count],
                    &format!("expects exactly {arity} argument(s), got {count}"),
                );
            }
        }
    }

    #[test]
    fn date_accessors_reject_every_nondate_type() {
        for name in ["DAY", "MONTH", "YEAR", "DAYINDEX"] {
            for value in [
                Value::Integer(2024),
                Value::Real(1.5),
                Value::Char('1'),
                Value::Boolean(true),
                Value::String("29/02/2024".to_owned()),
                Value::Array {
                    array: vec![date_value(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap())],
                    lower_bound: 1,
                    bounds_2d: None,
                },
                Value::Enum {
                    type_name: "Season".to_owned(),
                    variant: Some("Spring".to_owned()),
                },
            ] {
                reject(name, &[value], "argument 1 must be a date");
            }
        }
    }

    #[test]
    fn day_month_year_full_gregorian_cycle() {
        let mut date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2400, 1, 1).unwrap();
        while date < end {
            let value = date_value(date);
            for (name, expected) in [
                ("DAY", date.day() as i64),
                ("MONTH", date.month() as i64),
                ("YEAR", date.year() as i64),
            ] {
                assert_eq!(
                    call(name, std::slice::from_ref(&value)),
                    Value::Integer(expected),
                    "{name}({date})"
                );
            }
            date = date.succ_opt().unwrap();
        }
    }

    #[test]
    fn dayindex_full_gregorian_cycle() {
        let mut date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2400, 1, 1).unwrap();
        while date < end {
            assert_eq!(
                call("DAYINDEX", &[date_value(date)]),
                Value::Integer(date.weekday().number_from_sunday() as i64),
                "DAYINDEX({date})"
            );
            date = date.succ_opt().unwrap();
        }
    }

    #[test]
    fn dayindex_sunday_one_through_saturday_seven() {
        let mut source = String::new();
        let mut expected = String::new();
        for day in 1..=7 {
            source.push_str(&format!("OUTPUT DAYINDEX(SETDATE({day}, 1, 2023))\n"));
            expected.push_str(&format!("{day}\n"));
        }
        run_case(
            "weekday numbering Sunday=1 through Saturday=7",
            &source,
            &expected,
            "",
            None,
            &[],
        );
    }

    #[test]
    fn date_accessors_year_boundaries() {
        for year in [
            1, 4, 100, 400, 1582, 1600, 1700, 1800, 1899, 1900, 1999, 9999,
        ] {
            for (month, day) in [(1, 1), (2, 28), (3, 1), (12, 31)] {
                let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
                let value = date_value(date);
                for (name, expected) in [
                    ("DAY", day as i64),
                    ("MONTH", month as i64),
                    ("YEAR", year as i64),
                    ("DAYINDEX", date.weekday().number_from_sunday() as i64),
                ] {
                    assert_eq!(
                        call(name, std::slice::from_ref(&value)),
                        Value::Integer(expected),
                        "{name}({date})"
                    );
                }
            }
        }
    }

    #[test]
    fn dayindex_year_zero_january() {
        let date = NaiveDate::from_ymd_opt(0, 1, 1).unwrap();
        run_case(
            "DAYINDEX accepts the year zero supported by SETDATE",
            "OUTPUT DAYINDEX(SETDATE(1, 1, 0))\n",
            &format!("{}\n", date.weekday().number_from_sunday()),
            "",
            None,
            &[],
        );
    }

    #[test]
    fn dayindex_year_zero_february() {
        let date = NaiveDate::from_ymd_opt(0, 2, 29).unwrap();
        run_case(
            "DAYINDEX accepts the year zero leap day supported by SETDATE",
            "OUTPUT DAYINDEX(SETDATE(29, 2, 0))\n",
            &format!("{}\n", date.weekday().number_from_sunday()),
            "",
            None,
            &[],
        );
    }

    #[test]
    fn setdate_every_day_in_full_gregorian_cycle() {
        let mut date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2400, 1, 1).unwrap();
        while date < end {
            assert_eq!(
                call(
                    "SETDATE",
                    &[
                        Value::Integer(date.day() as i64),
                        Value::Integer(date.month() as i64),
                        Value::Integer(date.year() as i64)
                    ]
                ),
                date_value(date),
                "SETDATE({date})"
            );
            date = date.succ_opt().unwrap();
        }
    }

    #[test]
    fn setdate_invalid_days_each_month_and_leap_century() {
        for year in [
            0, 1, 4, 100, 400, 1600, 1700, 1800, 1900, 2000, 2024, 2025, 2100, 2400, 9999,
        ] {
            for month in 1..=12 {
                for day in 0..=32 {
                    if NaiveDate::from_ymd_opt(year, month, day).is_some() {
                        continue;
                    }
                    reject(
                        "SETDATE",
                        &[
                            Value::Integer(day as i64),
                            Value::Integer(month as i64),
                            Value::Integer(year as i64),
                        ],
                        "valid day",
                    );
                }
            }
        }
    }

    #[test]
    fn setdate_invalid_months_and_years_do_not_wrap() {
        for month in [-1, 0, 13, 65537, i64::MIN, i64::MAX] {
            reject(
                "SETDATE",
                &[
                    Value::Integer(1),
                    Value::Integer(month),
                    Value::Integer(2024),
                ],
                "SETDATE",
            );
        }
        for year in [-1, 10000, 65536, 67560, i64::MIN, i64::MAX] {
            reject(
                "SETDATE",
                &[Value::Integer(1), Value::Integer(1), Value::Integer(year)],
                "SETDATE",
            );
        }
        for day in [-1, 65537, i64::MIN, i64::MAX] {
            reject(
                "SETDATE",
                &[Value::Integer(day), Value::Integer(1), Value::Integer(2024)],
                "SETDATE",
            );
        }
    }

    #[test]
    fn setdate_wrong_type_at_each_argument_position() {
        let invalid = [
            Value::Real(1.5),
            Value::Real(f64::NAN),
            Value::Real(f64::INFINITY),
            Value::Real(f64::NEG_INFINITY),
            Value::Boolean(true),
            Value::String("1".to_owned()),
            Value::Char('1'),
            date_value(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            Value::Array {
                array: vec![Value::Integer(1)],
                lower_bound: 1,
                bounds_2d: None,
            },
            Value::Enum {
                type_name: "Season".to_owned(),
                variant: Some("Spring".to_owned()),
            },
        ];
        for position in 0..3 {
            for value in &invalid {
                let mut args = [Value::Integer(1), Value::Integer(1), Value::Integer(2024)];
                args[position] = value.clone();
                reject(
                    "SETDATE",
                    &args,
                    &format!("argument {} must be an integer", position + 1),
                );
            }
        }
    }

    #[test]
    fn setdate_integral_real_argument_extension() {
        for mask in 0..8 {
            let mut args = [Value::Integer(29), Value::Integer(2), Value::Integer(2024)];
            for (position, number) in [29.0, 2.0, 2024.0].into_iter().enumerate() {
                if mask & (1 << position) != 0 {
                    args[position] = Value::Real(number);
                }
            }
            assert_eq!(
                call("SETDATE", &args),
                date_value(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap())
            );
        }
    }

    #[test]
    fn setdate_arguments_preserve_order_and_evaluate_once() {
        run_case(
            "SETDATE evaluates day, month and year once in order",
            "\
DECLARE Calls : INTEGER
Calls <- 0
FUNCTION Part(N : INTEGER) RETURNS INTEGER
    Calls <- Calls + 1
    OUTPUT Calls
    RETURN N
ENDFUNCTION
OUTPUT SETDATE(Part(3), Part(4), Part(2024))
OUTPUT Calls
",
            "1\n2\n3\n03/04/2024\n3\n",
            "",
            None,
            &[],
        );
    }

    #[test]
    fn date_builtins_work_with_arrays_parameters_and_returns() {
        run_case(
            "date built-ins compose with arrays and procedures",
            "\
DECLARE Dates : ARRAY[2:3] OF DATE
FUNCTION Make(D : INTEGER, M : INTEGER, Y : INTEGER) RETURNS DATE
    RETURN SETDATE(D, M, Y)
ENDFUNCTION
PROCEDURE Show(D : DATE)
    DECLARE DayNumber : INTEGER
    DECLARE MonthNumber : INTEGER
    DECLARE YearNumber : INTEGER
    DECLARE WeekdayNumber : INTEGER
    DayNumber <- DAY(D)
    MonthNumber <- MONTH(D)
    YearNumber <- YEAR(D)
    WeekdayNumber <- DAYINDEX(D)
    OUTPUT DayNumber, \":\", MonthNumber, \":\", YearNumber, \":\", WeekdayNumber
ENDPROCEDURE
Dates[2] <- Make(29, 2, 2024)
Dates[3] <- Make(1, 1, 2000)
CALL Show(Dates[2])
CALL Show(Dates[3])
OUTPUT Dates[2] > Dates[3]
",
            "29:2:2024:5\n1:1:2000:7\nTRUE\n",
            "",
            None,
            &[],
        );
    }

    #[test]
    fn new_builtins_are_functions_not_procedures() {
        for (name, args) in [
            ("LEFT", "\"abc\", 1"),
            ("DAY", "01/01/2024"),
            ("MONTH", "01/01/2024"),
            ("YEAR", "01/01/2024"),
            ("DAYINDEX", "01/01/2024"),
            ("SETDATE", "1, 1, 2024"),
            ("TODAY", ""),
        ] {
            run_case(
                name,
                &format!("CALL {name}({args})\n"),
                "",
                "",
                Some("function"),
                &[],
            );
        }
    }

    #[test]
    fn new_builtin_errors_point_to_call_site() {
        for (call, error) in [
            ("LEFT(TRUE, 1)", "LEFT"),
            ("DAY(1)", "DAY"),
            ("MONTH(1)", "MONTH"),
            ("YEAR(1)", "YEAR"),
            ("DAYINDEX(1)", "DAYINDEX"),
            ("SETDATE(31, 4, 2024)", "SETDATE"),
            ("TODAY(1)", "TODAY"),
        ] {
            let source = format!("PROCEDURE P()\n    OUTPUT {call}\nENDPROCEDURE\nCALL P()\n");
            run_case(
                error,
                &source,
                "",
                "",
                Some("Runtime Error at line 2, column 5"),
                &[],
            );
        }
    }

    fn replay(
        source: &str,
        inputs: &[&str],
        output_skip: usize,
        log: &mut Vec<Value>,
    ) -> Result<(), ErrorType> {
        let tokens = Lexer::new(source.to_owned()).tokenize().unwrap();
        let ast = Parser::new(tokens, source.to_owned())
            .parse_statements()
            .unwrap();
        let ctx = Rc::new(RefCell::new(ReplayContext {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            input_pos: 0,
            output_skip,
            builtin_log: log.clone(),
            builtin_pos: 0,
            virtual_fs: Rc::new(RefCell::new(HashMap::new())),
        }));
        let result = Interpreter::new_replay(source.to_owned(), Rc::clone(&ctx)).interpret(ast);
        *log = ctx.borrow().builtin_log.clone();
        result.map_err(|error| error.error_type)
    }

    fn assert_output(result: Result<(), ErrorType>, expected: &str) {
        match result {
            Err(ErrorType::StepOutput(actual)) => assert_eq!(actual, expected),
            other => panic!("expected output {expected:?}, got {other:?}"),
        }
    }

    #[test]
    fn today_replay_keeps_historical_date_and_rand_after_input() {
        let source = "\
DECLARE D : DATE
DECLARE R : REAL
DECLARE Answer : STRING
D <- TODAY()
R <- RAND(10)
OUTPUT D
INPUT Answer
OUTPUT R
OUTPUT D
OUTPUT TODAY()
";
        let original = vec![
            date_value(NaiveDate::from_ymd_opt(2000, 12, 31).unwrap()),
            Value::Real(0.25),
        ];
        let mut log = original.clone();
        assert_output(replay(source, &[], 0, &mut log), "31/12/2000");
        for _ in 0..2 {
            assert!(
                matches!(replay(source, &[], 1, &mut log), Err(ErrorType::StepNeedsInput(name)) if name == "Answer")
            );
            assert_eq!(log, original);
        }
        assert_output(replay(source, &["continue"], 1, &mut log), "0.25");
        assert_output(replay(source, &["continue"], 2, &mut log), "31/12/2000");
        let before = chrono::Local::now().date_naive();
        let result = replay(source, &["continue"], 3, &mut log);
        let after = chrono::Local::now().date_naive();
        assert_eq!(log.len(), 3);
        assert_eq!(&log[..2], original.as_slice());
        assert!(log[2] == date_value(before) || log[2] == date_value(after));
        assert_output(result, &log[2].to_string());
        let saved = log.clone();
        assert!(replay(source, &["continue"], 4, &mut log).is_ok());
        assert_eq!(log, saved);
    }

    #[test]
    fn today_and_rand_nested_arguments_keep_replay_order() {
        let source = "OUTPUT RAND(DAY(TODAY()))\nOUTPUT TODAY()\n";
        let mut log = vec![
            date_value(NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()),
            Value::Real(2.5),
            date_value(NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()),
        ];
        let saved = log.clone();
        assert_output(replay(source, &[], 0, &mut log), "2.5");
        assert_output(replay(source, &[], 1, &mut log), "01/03/2024");
        assert!(replay(source, &[], 2, &mut log).is_ok());
        assert_eq!(log, saved);
    }

    #[test]
    fn all_new_builtins_through_web_step_interpreter() {
        let source = "\
DECLARE D : DATE
DECLARE Text : STRING
DECLARE Now : DATE
D <- SETDATE(29, 2, 2024)
OUTPUT DAY(D)
INPUT Text
OUTPUT LEFT(Text, 3)
OUTPUT MONTH(D)
OUTPUT YEAR(D)
OUTPUT DAYINDEX(D)
Now <- TODAY()
OUTPUT Now = SETDATE(DAY(Now), MONTH(Now), YEAR(Now))
";
        let mut runner = StepInterpreter::new(source).unwrap();
        assert!(matches!(runner.step(), StepEvent::Output { value } if value == "29"));
        assert!(matches!(runner.step(), StepEvent::NeedsInput { variable } if variable == "Text"));
        runner.supply_input("abcde".to_owned());
        for expected in ["abc", "2", "2024", "5", "TRUE"] {
            match runner.step() {
                StepEvent::Output { value } => assert_eq!(value, expected),
                other => panic!("expected {expected:?}, got {other:?}"),
            }
        }
        assert!(matches!(runner.step(), StepEvent::Done));
        assert!(matches!(runner.step(), StepEvent::Done));
    }

    #[test]
    fn invalid_new_builtin_calls_terminate_web_runner() {
        for expression in [
            "LEFT(TRUE, 1)",
            "DAY(1)",
            "MONTH(1)",
            "YEAR(1)",
            "DAYINDEX(1)",
            "SETDATE(31, 4, 2024)",
            "TODAY(1)",
        ] {
            let mut runner =
                StepInterpreter::new(&format!("OUTPUT {expression}\nOUTPUT \"unreachable\"\n"))
                    .unwrap();
            assert!(
                matches!(runner.step(), StepEvent::Error { .. }),
                "{expression}"
            );
            assert!(matches!(runner.step(), StepEvent::Done), "{expression}");
        }
    }
}
mod case_aliases {
    use super::*;
    use cambridge_pseudocode_interpreter::{
        errortype::ErrorType,
        Inter::{
            builtins::call_builtin,
            cps::{Date, Value},
            step_interpreter::{StepEvent, StepInterpreter},
        },
    };

    fn convert(name: &str, value: Value) -> Value {
        call_builtin(name.to_owned(), &[value.clone()])
            .unwrap_or_else(|error| panic!("{name}({value:?}): {error}"))
            .expect("case conversion must return a value")
    }

    fn reject(name: &str, args: &[Value], message: &str) {
        let error =
            call_builtin(name.to_owned(), args).expect_err(&format!("{name}({args:?}) must fail"));
        assert!(matches!(error.error_type, ErrorType::Runtime), "{error:?}");
        assert!(error.message.contains(name), "{error:?}");
        assert!(error.message.contains(message), "{error:?}");
    }

    #[test]
    fn every_ascii_char_preserves_type() {
        for code in 0u8..=127 {
            let ch = char::from(code);
            let upper = if code.is_ascii_lowercase() {
                code - 32
            } else {
                code
            };
            let lower = if code.is_ascii_uppercase() {
                code + 32
            } else {
                code
            };
            assert_eq!(
                convert("TO_UPPER", Value::Char(ch)),
                Value::Char(char::from(upper)),
                "U+{code:04X}"
            );
            assert_eq!(
                convert("TO_LOWER", Value::Char(ch)),
                Value::Char(char::from(lower)),
                "U+{code:04X}"
            );
        }
    }

    #[test]
    fn every_ascii_single_character_string_preserves_type() {
        for code in 0u8..=127 {
            let ch = char::from(code);
            let upper = if code.is_ascii_lowercase() {
                code - 32
            } else {
                code
            };
            let lower = if code.is_ascii_uppercase() {
                code + 32
            } else {
                code
            };
            assert_eq!(
                convert("TO_UPPER", Value::String(ch.to_string())),
                Value::String(char::from(upper).to_string()),
                "U+{code:04X}"
            );
            assert_eq!(
                convert("TO_LOWER", Value::String(ch.to_string())),
                Value::String(char::from(lower).to_string()),
                "U+{code:04X}"
            );
        }
    }

    #[test]
    fn strings_preserve_empty_whitespace_and_punctuation() {
        for (input, upper, lower) in [
            ("", "", ""),
            ("Hello", "HELLO", "hello"),
            ("aBc XYZ 019!?", "ABC XYZ 019!?", "abc xyz 019!?"),
            (" \t\r\n", " \t\r\n", " \t\r\n"),
            ("\0a\nZ\0", "\0A\nZ\0", "\0a\nz\0"),
            ("'\"_+-=", "'\"_+-=", "'\"_+-="),
        ] {
            for (name, expected) in [("TO_UPPER", upper), ("TO_LOWER", lower)] {
                assert_eq!(
                    convert(name, Value::String(input.to_owned())),
                    Value::String(expected.to_owned()),
                    "{name}({input:?})"
                );
            }
        }
    }

    #[test]
    fn unicode_chars_preserve_one_scalar_and_type() {
        for (input, upper, lower) in [
            ('é', 'É', 'é'),
            ('É', 'É', 'é'),
            ('ω', 'Ω', 'ω'),
            ('Ж', 'Ж', 'ж'),
            ('я', 'Я', 'я'),
            ('ı', 'I', 'ı'),
            ('🙂', '🙂', '🙂'),
            ('中', '中', '中'),
            ('\u{301}', '\u{301}', '\u{301}'),
            ('𐐨', '𐐀', '𐐨'),
        ] {
            assert_eq!(
                convert("TO_UPPER", Value::Char(input)),
                Value::Char(upper),
                "{input:?}"
            );
            assert_eq!(
                convert("TO_LOWER", Value::Char(input)),
                Value::Char(lower),
                "{input:?}"
            );
        }
    }

    #[test]
    fn upper_rejects_expanding_chars() {
        for ch in ['ß', 'ﬃ', 'ﬀ', 'ΐ'] {
            reject("TO_UPPER", &[Value::Char(ch)], "exactly one character");
        }
    }

    #[test]
    fn lower_rejects_expanding_chars() {
        reject("TO_LOWER", &[Value::Char('İ')], "exactly one character");
    }

    #[test]
    fn unicode_strings_keep_expansions_and_combining_marks() {
        for (input, upper, lower) in [
            ("café Ω Ж🙂中", "CAFÉ Ω Ж🙂中", "café ω ж🙂中"),
            ("Straße ﬃ ﬀ", "STRASSE FFI FF", "straße ﬃ ﬀ"),
            ("İ", "İ", "i\u{307}"),
            ("e\u{301}", "E\u{301}", "e\u{301}"),
            ("ΐ", "Ι\u{308}\u{301}", "ΐ"),
            ("𐐨𐐀", "𐐀𐐀", "𐐨𐐨"),
        ] {
            for (name, expected) in [("TO_UPPER", upper), ("TO_LOWER", lower)] {
                assert_eq!(
                    convert(name, Value::String(input.to_owned())),
                    Value::String(expected.to_owned()),
                    "{name}({input:?})"
                );
            }
        }
    }

    #[test]
    fn lowercase_handles_contextual_greek_sigma() {
        for (input, expected) in [
            ("ΟΣ", "ος"),
            ("ΟΣΑ", "οσα"),
            ("Σ", "σ"),
            ("ΟΣ ΟΣΑ", "ος οσα"),
        ] {
            assert_eq!(
                convert("TO_LOWER", Value::String(input.to_owned())),
                Value::String(expected.to_owned())
            );
        }
        assert_eq!(convert("TO_LOWER", Value::Char('Σ')), Value::Char('σ'));
    }

    #[test]
    fn long_strings_are_not_truncated() {
        let input = Value::String("aßİ🙂\0\n".repeat(4096));
        assert_eq!(
            convert("TO_UPPER", input.clone()),
            Value::String("ASSİ🙂\0\n".repeat(4096))
        );
        assert_eq!(
            convert("TO_LOWER", input),
            Value::String("aßi\u{307}🙂\0\n".repeat(4096))
        );
    }

    #[test]
    fn repeated_conversion_is_idempotent() {
        for name in ["TO_UPPER", "TO_LOWER"] {
            for input in ["", "AbC", "Straße İ ﬃ ΟΣ", "🙂\0\n", "e\u{301}"] {
                let once = convert(name, Value::String(input.to_owned()));
                assert_eq!(convert(name, once.clone()), once, "{name}({input:?})");
            }
        }
    }

    #[test]
    fn aliases_match_originals_without_mutating_arguments() {
        for (alias, original) in [("TO_UPPER", "UCASE"), ("TO_LOWER", "LCASE")] {
            for value in [
                Value::Char('é'),
                Value::Char('🙂'),
                Value::String(String::new()),
                Value::String("AbC ß İ ΟΣ\0".to_owned()),
            ] {
                let args = vec![value];
                let saved = args.clone();
                assert_eq!(
                    call_builtin(alias.to_owned(), &args).unwrap(),
                    call_builtin(original.to_owned(), &args).unwrap()
                );
                assert_eq!(args, saved);
            }
        }
    }

    #[test]
    fn wrong_argument_counts_report_alias_name() {
        for name in ["TO_UPPER", "TO_LOWER"] {
            for count in [0, 2, 3, 10] {
                reject(
                    name,
                    &vec![Value::String("a".to_owned()); count],
                    &format!("expects exactly 1 argument(s), got {count}"),
                );
            }
        }
    }

    #[test]
    fn wrong_argument_types_report_alias_name() {
        for name in ["TO_UPPER", "TO_LOWER"] {
            for value in [
                Value::Integer(65),
                Value::Real(65.5),
                Value::Boolean(true),
                Value::Date(Date {
                    day: 1,
                    month: 1,
                    year: 2024,
                }),
                Value::Array {
                    array: vec![Value::Char('a')],
                    lower_bound: 1,
                    bounds_2d: None,
                },
                Value::Enum {
                    type_name: "Letters".to_owned(),
                    variant: Some("A".to_owned()),
                },
            ] {
                reject(name, &[value], "argument 1");
            }
        }
    }

    case!(
        typed_variables_and_arrays,
        "case aliases preserve types in assignments",
        r#"
DECLARE C : CHAR
DECLARE S : STRING
DECLARE Chars : ARRAY[1:2] OF CHAR
DECLARE Words : ARRAY[1:2] OF STRING
C <- TO_UPPER('a')
S <- TO_LOWER("Hello")
Chars[1] <- TO_LOWER(C)
Chars[2] <- TO_UPPER('é')
Words[1] <- TO_UPPER(S)
Words[2] <- TO_LOWER("")
OUTPUT C, ":", S, ":", Chars[1], ":", Chars[2], ":", Words[1], ":", LENGTH(Words[2])
OUTPUT TO_UPPER("a") = "A", TO_LOWER('A') = 'a'
"#,
        "A:hello:a:É:HELLO:0\nTRUETRUE\n",
        "",
        None,
        &[]
    );

    case!(
        nested_calls_and_function_types,
        "case aliases in nested calls and functions",
        r#"
FUNCTION Upper(C : CHAR) RETURNS CHAR
RETURN TO_UPPER(C)
ENDFUNCTION
FUNCTION Lower(S : STRING) RETURNS STRING
RETURN TO_LOWER(S)
ENDFUNCTION
OUTPUT Upper(TO_LOWER('Z'))
OUTPUT Lower(TO_UPPER("Hello"))
OUTPUT TO_LOWER(TO_UPPER("AbC")), ":", TO_UPPER(TO_LOWER('Q'))
OUTPUT LENGTH(TO_UPPER("ßﬃ")), ":", ASC(TO_UPPER('a'))
"#,
        "Z\nhello\nabc:Q\n5:65\n",
        "",
        None,
        &[]
    );

    case!(
        input_and_original_values,
        "case aliases leave input variables unchanged",
        r#"
DECLARE S : STRING
DECLARE C : CHAR
INPUT S
INPUT C
OUTPUT TO_UPPER(S), ":", TO_LOWER(S), ":", S
OUTPUT TO_UPPER(C), ":", TO_LOWER(C), ":", C
"#,
        "HELLO:hello:Hello\nÉ:é:é\n",
        "Hello\né\n",
        None,
        &[]
    );

    case!(
        unicode_expansions_through_cli,
        "case aliases retain string expansions",
        "OUTPUT TO_UPPER(\"ßﬃ\")\nOUTPUT TO_LOWER(\"İ\")\n",
        "SSFFI\ni\u{307}\n",
        "",
        None,
        &[]
    );

    #[test]
    fn invalid_calls_through_cli() {
        for (expression, message) in [
            (
                "TO_UPPER()",
                "TO_UPPER expects exactly 1 argument(s), got 0",
            ),
            (
                "TO_LOWER()",
                "TO_LOWER expects exactly 1 argument(s), got 0",
            ),
            (
                "TO_UPPER(\"a\", \"b\")",
                "TO_UPPER expects exactly 1 argument(s), got 2",
            ),
            (
                "TO_LOWER('a', 'b')",
                "TO_LOWER expects exactly 1 argument(s), got 2",
            ),
            ("TO_UPPER(TRUE)", "TO_UPPER argument 1"),
            ("TO_LOWER(42)", "TO_LOWER argument 1"),
            (
                "TO_UPPER('ß')",
                "TO_UPPER result must contain exactly one character",
            ),
            (
                "TO_LOWER('İ')",
                "TO_LOWER result must contain exactly one character",
            ),
        ] {
            run_case(
                expression,
                &format!("OUTPUT {expression}\n"),
                "",
                "",
                Some(message),
                &[],
            );
        }
    }

    #[test]
    fn aliases_cannot_be_called_as_procedures() {
        for name in ["TO_UPPER", "TO_LOWER"] {
            run_case(
                name,
                &format!("CALL {name}(\"AbC\")\n"),
                "",
                "",
                Some("as a procedure"),
                &[],
            );
        }
    }

    #[test]
    fn aliases_through_web_step_interpreter() {
        let source = "DECLARE S : STRING\nDECLARE C : CHAR\nINPUT S\nINPUT C\nOUTPUT TO_UPPER(S)\nOUTPUT TO_LOWER(S)\nOUTPUT TO_UPPER(C)\nOUTPUT TO_LOWER(C)\nOUTPUT S\nOUTPUT C\n";
        let mut runner = StepInterpreter::new(source).unwrap();
        for _ in 0..2 {
            assert!(matches!(runner.step(), StepEvent::NeedsInput { variable } if variable == "S"));
        }
        runner.supply_input("Straße İ".to_owned());
        assert!(matches!(runner.step(), StepEvent::NeedsInput { variable } if variable == "C"));
        runner.supply_input("é".to_owned());
        for expected in ["STRASSE İ", "straße i\u{307}", "É", "é", "Straße İ", "é"] {
            match runner.step() {
                StepEvent::Output { value } => assert_eq!(value, expected),
                other => panic!("expected {expected:?}, got {other:?}"),
            }
        }
        assert!(matches!(runner.step(), StepEvent::Done));
    }

    #[test]
    fn invalid_alias_calls_terminate_web_runner() {
        for (name, argument) in [
            ("TO_UPPER", "'ß'"),
            ("TO_LOWER", "'İ'"),
            ("TO_UPPER", "TRUE"),
            ("TO_LOWER", ""),
        ] {
            let mut runner = StepInterpreter::new(&format!(
                "OUTPUT {name}({argument})\nOUTPUT \"unreachable\"\n"
            ))
            .unwrap();
            match runner.step() {
                StepEvent::Error { message } => assert!(message.contains(name), "{message}"),
                other => panic!("expected error for {name}({argument}), got {other:?}"),
            }
            assert!(matches!(runner.step(), StepEvent::Done));
        }
    }
}
mod parameter_passing {
    use super::*;
    use cambridge_pseudocode_interpreter::Inter::step_interpreter::{StepEvent, StepInterpreter};

    fn check_modes(markers: &[&str], byref: &[bool]) {
        assert_eq!(markers.len(), byref.len());
        let parameters = markers
            .iter()
            .enumerate()
            .map(|(i, marker)| format!("{marker} P{i} : INTEGER"))
            .collect::<Vec<_>>()
            .join(",\n");
        let mut source = format!("PROCEDURE Change({parameters})\n");
        let mut expected = String::new();
        for i in 0..markers.len() {
            source.push_str(&format!("P{i} <- P{i} + {}\nOUTPUT P{i}\n", 100 + i));
            expected.push_str(&format!("{}\n", 110 + 2 * i));
        }
        source.push_str("ENDPROCEDURE\n");
        for i in 0..markers.len() {
            source.push_str(&format!("DECLARE V{i} : INTEGER\nV{i} <- {}\n", 10 + i));
        }
        let arguments = (0..markers.len())
            .map(|i| format!("V{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        source.push_str(&format!("CALL Change({arguments})\n"));
        for (i, is_ref) in byref.iter().enumerate() {
            source.push_str(&format!("OUTPUT V{i}\n"));
            expected.push_str(&format!("{}\n", if *is_ref { 110 + 2 * i } else { 10 + i }));
        }
        run_case(
            &format!("passing modes {markers:?}"),
            &source,
            &expected,
            "",
            None,
            &[],
        );
    }

    #[test]
    fn unspecified_parameters_default_to_byval() {
        check_modes(&["", "", ""], &[false, false, false]);
    }

    #[test]
    fn byref_applies_to_all_following_parameters() {
        check_modes(&["BYREF", "", ""], &[true, true, true]);
    }

    #[test]
    fn byval_switches_following_parameters_back_to_copies() {
        check_modes(&["BYREF", "", "BYVAL", ""], &[true, true, false, false]);
    }

    #[test]
    fn passing_mode_can_switch_back_to_byref() {
        check_modes(
            &["BYREF", "BYVAL", "", "BYREF", ""],
            &[true, false, false, true, true],
        );
    }

    #[test]
    fn initial_default_ends_at_first_byref() {
        check_modes(&["", "", "BYREF", ""], &[false, false, true, true]);
    }

    #[test]
    fn every_five_parameter_marker_combination() {
        for pattern in 0..243 {
            let mut digits = pattern;
            let mut markers = Vec::new();
            for _ in 0..5 {
                markers.push(["", "BYVAL", "BYREF"][digits % 3]);
                digits /= 3;
            }
            let expected = (0..5)
                .map(|i| markers[..=i].iter().rfind(|marker| !marker.is_empty()) == Some(&"BYREF"))
                .collect::<Vec<_>>();
            check_modes(&markers, &expected);
        }
    }

    case!(
        mode_resets_between_declarations,
        "each declaration starts with BYVAL",
        r#"
PROCEDURE First(BYREF A : INTEGER, B : INTEGER)
A <- 10
B <- 20
ENDPROCEDURE
PROCEDURE Second(A : INTEGER, B : INTEGER)
A <- 30
B <- 40
OUTPUT A, ":", B
ENDPROCEDURE
FUNCTION Third(A : INTEGER, B : INTEGER) RETURNS INTEGER
A <- 50
B <- 60
RETURN A + B
ENDFUNCTION
DECLARE X : INTEGER
DECLARE Y : INTEGER
X <- 1
Y <- 2
CALL First(X, Y)
CALL Second(X, Y)
OUTPUT Third(X, Y)
OUTPUT X, ":", Y
"#,
        "30:40\n110\n10:20\n",
        "",
        None,
        &[]
    );

    case!(
        same_variable_has_live_aliases_and_independent_copies,
        "BYREF aliases share storage and BYVAL takes a snapshot",
        r#"
PROCEDURE Change(BYREF A : INTEGER, B : INTEGER, BYVAL C : INTEGER, D : INTEGER)
A <- 10
OUTPUT B, ":", C, ":", D
B <- B + 1
C <- 20
OUTPUT A, ":", B, ":", C, ":", D
ENDPROCEDURE
DECLARE X : INTEGER
X <- 1
CALL Change(X, X, X, X)
OUTPUT X
"#,
        "10:1:1\n11:11:20:1\n11\n",
        "",
        None,
        &[]
    );

    case!(
        nested_forwarding_respects_each_callee_mode,
        "references can be forwarded and copies stay local",
        r#"
PROCEDURE Inner(BYREF A : INTEGER, B : INTEGER)
A <- A + 10
B <- B + 20
ENDPROCEDURE
PROCEDURE Copy(A : INTEGER, B : INTEGER)
A <- 100
B <- 200
ENDPROCEDURE
PROCEDURE Outer(BYREF A : INTEGER, BYVAL B : INTEGER)
CALL Inner(A, B)
CALL Copy(A, B)
OUTPUT A, ":", B
ENDPROCEDURE
DECLARE X : INTEGER
DECLARE Y : INTEGER
X <- 1
Y <- 2
CALL Outer(X, Y)
OUTPUT X, ":", Y
CALL Outer(X, Y)
OUTPUT X, ":", Y
"#,
        "11:22\n11:2\n21:22\n21:2\n",
        "",
        None,
        &[]
    );

    case!(
        arrays_follow_inherited_modes,
        "array references alias while array copies stay independent",
        r#"
PROCEDURE Change(BYREF A : ARRAY[1:2] OF INTEGER, B : ARRAY[1:2] OF INTEGER,
BYVAL C : ARRAY[1:2] OF INTEGER, D : ARRAY[1:2] OF INTEGER)
A[1] <- 10
B[2] <- 20
C[1] <- 30
D[2] <- 40
OUTPUT A[1], ":", B[2], ":", C[1], ":", C[2], ":", D[1], ":", D[2]
ENDPROCEDURE
DECLARE V : ARRAY[1:2] OF INTEGER
V[1] <- 1
V[2] <- 2
CALL Change(V, V, V, V)
OUTPUT V[1], ":", V[2]
"#,
        "10:20:30:2:1:40\n10:20\n",
        "",
        None,
        &[]
    );

    case!(
        mode_survives_parameter_type_changes,
        "passing mode carries across different parameter types",
        r#"
TYPE Season = (Spring, Summer)
PROCEDURE Change(BYREF A : INTEGER, B : STRING, C : Season, BYVAL D : CHAR, E : BOOLEAN)
A <- 7
B <- "changed"
C <- Summer
D <- 'Z'
E <- TRUE
OUTPUT D, ":", E
ENDPROCEDURE
DECLARE N : INTEGER
DECLARE S : STRING
DECLARE SeasonValue : Season
DECLARE C : CHAR
DECLARE Flag : BOOLEAN
S <- "original"
SeasonValue <- Spring
C <- 'a'
Flag <- FALSE
CALL Change(N, S, SeasonValue, C, Flag)
OUTPUT N, ":", S, ":", SeasonValue, ":", C, ":", Flag
"#,
        "Z:TRUE\n7:changed:Summer:a:FALSE\n",
        "",
        None,
        &[]
    );

    case!(
        byval_after_byref_accepts_expressions,
        "explicit BYVAL permits literals expressions and function results",
        r#"
FUNCTION Number() RETURNS INTEGER
RETURN 8
ENDFUNCTION
PROCEDURE Change(BYREF A : INTEGER, BYVAL B : INTEGER, C : INTEGER, D : INTEGER)
A <- B + C + D
B <- 99
C <- 99
D <- 99
ENDPROCEDURE
DECLARE X : INTEGER
X <- 1
CALL Change(X, 2, X + 3, Number())
OUTPUT X
"#,
        "14\n",
        "",
        None,
        &[]
    );

    #[test]
    fn inherited_byref_rejects_nonvariable_arguments() {
        for argument in ["2", "X + 1", "Number()", "Fixed"] {
            let source = format!("CONSTANT Fixed = 2\nFUNCTION Number() RETURNS INTEGER\nRETURN 2\nENDFUNCTION\nPROCEDURE Change(BYREF A : INTEGER, B : INTEGER)\nENDPROCEDURE\nDECLARE X : INTEGER\nCALL Change(X, {argument})\n");
            run_case(argument, &source, "", "", Some("BYREF"), &[]);
        }
    }

    case!(inherited_byref_requires_exact_type, "inherited BYREF rejects REAL for INTEGER", "PROCEDURE Change(BYREF A : INTEGER, B : INTEGER)\nENDPROCEDURE\nDECLARE X : INTEGER\nDECLARE Y : REAL\nY <- 2\nCALL Change(X, Y)\n", "", "", Some("Type mismatch for BYREF parameter 'B'"), &[]);

    case!(
        functions_copy_every_parameter,
        "default and explicit BYVAL function parameters do not modify callers",
        r#"
FUNCTION Change(A : INTEGER, BYVAL B : INTEGER, C : INTEGER) RETURNS INTEGER
A <- 10
B <- 20
C <- 30
RETURN A + B + C
ENDFUNCTION
DECLARE X : INTEGER
X <- 1
OUTPUT Change(X, X, X)
OUTPUT X
OUTPUT Change(2, 3 + 4, 5)
"#,
        "60\n1\n60\n",
        "",
        None,
        &[]
    );

    case!(
        functions_copy_array_parameters,
        "function arrays are copied for all BYVAL parameters",
        r#"
FUNCTION Change(A : ARRAY[1:2] OF INTEGER, BYVAL B : ARRAY[1:2] OF INTEGER,
C : ARRAY[1:2] OF INTEGER) RETURNS INTEGER
A[1] <- 10
B[2] <- 20
C[1] <- 30
RETURN A[1] + A[2] + B[1] + B[2] + C[1] + C[2]
ENDFUNCTION
DECLARE V : ARRAY[1:2] OF INTEGER
V[1] <- 1
V[2] <- 2
OUTPUT Change(V, V, V)
OUTPUT V[1], ":", V[2]
"#,
        "65\n1:2\n",
        "",
        None,
        &[]
    );

    #[test]
    fn functions_reject_byref_at_every_parameter_position() {
        for parameters in [
            "BYREF A : INTEGER, B : INTEGER, C : INTEGER",
            "A : INTEGER, BYREF B : INTEGER, C : INTEGER",
            "BYVAL A : INTEGER, B : INTEGER, BYREF C : INTEGER",
        ] {
            run_case(
                parameters,
                &format!("FUNCTION F({parameters}) RETURNS INTEGER\nRETURN 1\nENDFUNCTION\n"),
                "",
                "",
                Some("Parameters should not be passed by reference to a function"),
                &[],
            );
        }
    }

    #[test]
    fn web_replay_preserves_mixed_modes_across_input() {
        let source = "PROCEDURE Change(BYREF A : INTEGER, B : INTEGER, BYVAL C : INTEGER, D : INTEGER)\nA <- A + 10\nB <- B + 20\nINPUT C\nD <- D + C\nOUTPUT A, \":\", B, \":\", C, \":\", D\nENDPROCEDURE\nDECLARE X : INTEGER\nDECLARE Y : INTEGER\nDECLARE Z : INTEGER\nDECLARE W : INTEGER\nX <- 1\nY <- 2\nZ <- 3\nW <- 4\nCALL Change(X, Y, Z, W)\nOUTPUT X, \":\", Y, \":\", Z, \":\", W\n";
        let mut runner = StepInterpreter::new(source).unwrap();
        for _ in 0..2 {
            assert!(matches!(runner.step(), StepEvent::NeedsInput { variable } if variable == "C"));
        }
        runner.supply_input("5".to_owned());
        for expected in ["11:22:5:9", "11:22:3:4"] {
            match runner.step() {
                StepEvent::Output { value } => assert_eq!(value, expected),
                other => panic!("expected {expected:?}, got {other:?}"),
            }
        }
        assert!(matches!(runner.step(), StepEvent::Done));
    }
}
mod grouped_parameters {
    use super::*;

    case!(
        function_names_share_type_and_keep_order,
        "grouped function parameters default to BYVAL",
        r#"
FUNCTION Digits(A, B, C, D : INTEGER) RETURNS INTEGER
A <- A + 1
RETURN A * 1000 + B * 100 + C * 10 + D
ENDFUNCTION
DECLARE X : INTEGER
X <- 1
OUTPUT Digits(X, 2, 3, 4)
OUTPUT X
"#,
        "2234\n1\n",
        "",
        None,
        &[]
    );

    case!(
        procedure_groups_inherit_and_switch_modes,
        "grouped types preserve passing mode inheritance",
        r#"
PROCEDURE Change(A, B : INTEGER, BYREF C, D : INTEGER,
BYVAL E, F : INTEGER, BYREF G, H : INTEGER)
A <- 1
B <- 2
C <- 3
D <- 4
E <- 5
F <- 6
G <- 7
H <- 8
ENDPROCEDURE
DECLARE A, B, C, D, E, F, G, H : INTEGER
CALL Change(A, B, C, D, E, F, G, H)
OUTPUT A, B, C, D, E, F, G, H
"#,
        "00340078\n",
        "",
        None,
        &[]
    );

    case!(
        passing_modes_carry_across_type_groups,
        "passing mode persists after each grouped type",
        r#"
PROCEDURE Change(BYREF A, B : INTEGER, C, D : STRING)
A <- 1
B <- 2
C <- "three"
D <- "four"
ENDPROCEDURE
DECLARE X, Y : INTEGER
DECLARE S, T : STRING
CALL Change(X, Y, S, T)
OUTPUT X, ":", Y, ":", S, ":", T
"#,
        "1:2:three:four\n",
        "",
        None,
        &[]
    );

    case!(
        mode_switch_within_a_type_group,
        "each grouped name keeps its own passing mode",
        r#"
PROCEDURE Change(BYREF A, BYVAL B, C, BYREF D, E : INTEGER)
A <- 1
B <- 2
C <- 3
D <- 4
E <- 5
ENDPROCEDURE
DECLARE V, W, X, Y, Z : INTEGER
CALL Change(V, W, X, Y, Z)
OUTPUT V, W, X, Y, Z
"#,
        "10045\n",
        "",
        None,
        &[]
    );

    case!(
        mixed_grouped_and_individual_types,
        "grouped names work alongside individually typed parameters",
        r#"
TYPE Season = (Spring, Summer)
FUNCTION Describe(A, B : STRING, C : INTEGER, BYVAL D, E : Season) RETURNS STRING
D <- Summer
RETURN A & B & NUM_TO_STR(C) & D & E
ENDFUNCTION
OUTPUT Describe("a", "b", 3, Spring, Spring)
"#,
        "ab3SummerSpring\n",
        "",
        None,
        &[]
    );

    case!(
        grouped_array_parameters,
        "grouped array parameters preserve copy and reference semantics",
        r#"
PROCEDURE Change(BYREF A, B : ARRAY[1:2] OF INTEGER, BYVAL C, D : ARRAY[1:2] OF INTEGER)
A[1] <- 10
B[2] <- 20
C[1] <- 30
D[2] <- 40
OUTPUT C[1], ":", C[2], ":", D[1], ":", D[2]
ENDPROCEDURE
DECLARE V : ARRAY[1:2] OF INTEGER
V[1] <- 1
V[2] <- 2
CALL Change(V, V, V, V)
OUTPUT V[1], ":", V[2]
"#,
        "30:2:1:40\n10:20\n",
        "",
        None,
        &[]
    );

    #[test]
    fn every_grouped_name_has_the_declared_type() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            for index in 0..4 {
                let mut arguments = ["1", "2", "3", "4"];
                arguments[index] = "TRUE";
                let source = if kind == "PROCEDURE" {
                    format!(
                        "PROCEDURE P(A, B, C, D : INTEGER)\nENDPROCEDURE\nCALL P({})\n",
                        arguments.join(", ")
                    )
                } else {
                    format!("FUNCTION F(A, B, C, D : INTEGER) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\nOUTPUT F({})\n", arguments.join(", "))
                };
                let name = ["A", "B", "C", "D"][index];
                run_case(
                    &format!("{kind} parameter {name}"),
                    &source,
                    "",
                    "",
                    Some(&format!("Type mismatch for parameter '{name}'")),
                    &[],
                );
            }
        }
    }

    #[test]
    fn invalid_parameter_groups_are_rejected() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            for (parameters, message) in [
                ("A, A : INTEGER", "2 parameters with the same name"),
                ("A, a : INTEGER", "2 parameters with the same name"),
                (
                    "A, B : INTEGER, C, a : STRING",
                    "2 parameters with the same name",
                ),
                ("A, B", "Expected ':'"),
                ("A,", "Expected parameter name"),
                ("A, : INTEGER", "Expected parameter name"),
                ("A,, B : INTEGER", "Expected parameter name"),
                (", A : INTEGER", "Expected parameter name"),
                ("A, OUTPUT : INTEGER", "Expected parameter name"),
            ] {
                let source = if kind == "PROCEDURE" {
                    format!("PROCEDURE P({parameters})\nENDPROCEDURE\n")
                } else {
                    format!("FUNCTION F({parameters}) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n")
                };
                run_case(
                    &format!("{kind}({parameters})"),
                    &source,
                    "",
                    "",
                    Some(message),
                    &[],
                );
            }
        }
    }

    case!(
        grouped_function_still_rejects_byref,
        "grouped function parameters cannot switch to BYREF",
        "FUNCTION F(A, BYREF B, C : INTEGER) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n",
        "",
        "",
        Some("Parameters should not be passed by reference to a function"),
        &[]
    );
}
mod grouped_parameter_coverage {
    use super::*;
    use cambridge_pseudocode_interpreter::Inter::step_interpreter::{StepEvent, StepInterpreter};

    fn expect_outputs(source: &str, expected: &[&str]) {
        let mut runner =
            StepInterpreter::new(source).unwrap_or_else(|error| panic!("{error}\n{source}"));
        for expected in expected {
            match runner.step() {
                StepEvent::Output { value } => assert_eq!(value, *expected, "{source}"),
                other => panic!("expected {expected:?}, got {other:?}\n{source}"),
            }
        }
        assert!(matches!(runner.step(), StepEvent::Done), "{source}");
    }

    fn signature(markers: &[&str], boundaries: usize) -> String {
        markers
            .iter()
            .enumerate()
            .map(|(i, marker)| {
                let type_ = if i == markers.len() - 1 || boundaries & (1 << i) != 0 {
                    " : INTEGER"
                } else {
                    ""
                };
                format!("{marker} P{i}{type_}")
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[test]
    fn procedures_all_group_boundaries_and_passing_modes() {
        for pattern in 0..243 {
            let mut digits = pattern;
            let markers: Vec<_> = (0..5)
                .map(|_| {
                    let marker = ["", "BYVAL", "BYREF"][digits % 3];
                    digits /= 3;
                    marker
                })
                .collect();
            let expected = (0..5)
                .map(|i| {
                    let byref =
                        markers[..=i].iter().rfind(|marker| !marker.is_empty()) == Some(&"BYREF");
                    if byref {
                        (i + 11).to_string()
                    } else {
                        (i + 1).to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(":");
            for boundaries in 0..16 {
                let parameters = signature(&markers, boundaries);
                let source = format!("PROCEDURE P({parameters})\nP0 <- P0 + 10\nP1 <- P1 + 10\nP2 <- P2 + 10\nP3 <- P3 + 10\nP4 <- P4 + 10\nOUTPUT P0, \":\", P1, \":\", P2, \":\", P3, \":\", P4\nENDPROCEDURE\nDECLARE A, B, C, D, E : INTEGER\nA <- 1\nB <- 2\nC <- 3\nD <- 4\nE <- 5\nCALL P(A, B, C, D, E)\nOUTPUT A, \":\", B, \":\", C, \":\", D, \":\", E\n");
                expect_outputs(&source, &["11:12:13:14:15", &expected]);
            }
        }
    }

    #[test]
    fn functions_all_group_boundaries_and_byval_markers() {
        for pattern in 0..32 {
            let markers: Vec<_> = (0..5)
                .map(|i| if pattern & (1 << i) != 0 { "BYVAL" } else { "" })
                .collect();
            for boundaries in 0..16 {
                let parameters = signature(&markers, boundaries);
                let source = format!("FUNCTION F({parameters}) RETURNS INTEGER\nP0 <- P0 + 1\nP1 <- P1 + 1\nP2 <- P2 + 1\nP3 <- P3 + 1\nP4 <- P4 + 1\nRETURN P0 * 10000 + P1 * 1000 + P2 * 100 + P3 * 10 + P4\nENDFUNCTION\nDECLARE A, B, C, D, E : INTEGER\nA <- 1\nB <- 2\nC <- 3\nD <- 4\nE <- 5\nOUTPUT F(A, B, C, D, E)\nOUTPUT A, B, C, D, E\n");
                expect_outputs(&source, &["23456", "12345"]);
            }
        }
    }

    #[test]
    fn every_scalar_and_enum_group_preserves_passing_modes() {
        for (type_, initial, changed, old_output, new_output) in [
            ("INTEGER", "1", "2", "1", "2"),
            ("REAL", "1.25", "2.75", "1.25", "2.75"),
            ("STRING", "\"old\"", "\"new\"", "old", "new"),
            ("CHAR", "'a'", "'z'", "a", "z"),
            ("BOOLEAN", "FALSE", "TRUE", "FALSE", "TRUE"),
            (
                "DATE",
                "01/01/2024",
                "29/02/2024",
                "01/01/2024",
                "29/02/2024",
            ),
            ("Season", "Spring", "Summer", "Spring", "Summer"),
        ] {
            let source = format!("TYPE Season = (Spring, Summer)\nPROCEDURE Change(BYREF A, B : {type_}, BYVAL C, D : {type_})\nA <- {changed}\nB <- {changed}\nC <- {changed}\nD <- {changed}\nOUTPUT A, \":\", B, \":\", C, \":\", D\nENDPROCEDURE\nDECLARE W, X, Y, Z : {type_}\nW <- {initial}\nX <- {initial}\nY <- {initial}\nZ <- {initial}\nCALL Change(W, X, Y, Z)\nOUTPUT W, \":\", X, \":\", Y, \":\", Z\n");
            expect_outputs(
                &source,
                &[
                    &[new_output; 4].join(":"),
                    &format!("{new_output}:{new_output}:{old_output}:{old_output}"),
                ],
            );
        }
    }

    #[test]
    fn wrong_types_are_rejected_in_every_group_position() {
        for (type_, value, wrong) in [
            ("INTEGER", "1", "TRUE"),
            ("REAL", "1.25", "TRUE"),
            ("STRING", "\"a\"", "TRUE"),
            ("CHAR", "'a'", "\"a\""),
            ("BOOLEAN", "TRUE", "1"),
            ("DATE", "01/01/2024", "\"01/01/2024\""),
            ("Season", "Spring", "OtherSpring"),
        ] {
            for kind in ["PROCEDURE", "FUNCTION"] {
                for position in 0..4 {
                    let mut args = [value; 4];
                    args[position] = wrong;
                    let args = args.join(", ");
                    let header = "TYPE Season = (Spring, Summer)\nTYPE OtherSeason = (OtherSpring, OtherSummer)\n";
                    let source = if kind == "PROCEDURE" {
                        format!("{header}PROCEDURE P(A, B, C, D : {type_})\nENDPROCEDURE\nCALL P({args})\n")
                    } else {
                        format!("{header}FUNCTION F(A, B, C, D : {type_}) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\nOUTPUT F({args})\n")
                    };
                    run_case(
                        &format!("{kind} {type_} argument {position}"),
                        &source,
                        "",
                        "",
                        Some(&format!(
                            "Type mismatch for parameter '{}'",
                            ["A", "B", "C", "D"][position]
                        )),
                        &[],
                    );
                }
            }
        }
    }

    #[test]
    fn arity_counts_names_not_type_groups() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            for count in [0, 1, 2, 3, 5, 8] {
                let args = vec!["1"; count].join(", ");
                let source = if kind == "PROCEDURE" {
                    format!("PROCEDURE P(A, B : INTEGER, C, D : INTEGER)\nENDPROCEDURE\nCALL P({args})\n")
                } else {
                    format!("FUNCTION F(A, B : INTEGER, C, D : INTEGER) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\nOUTPUT F({args})\n")
                };
                run_case(
                    kind,
                    &source,
                    "",
                    "",
                    Some(&format!("expected 4 arguments, got {count}")),
                    &[],
                );
            }
        }
    }

    #[test]
    fn byref_requires_variables_at_every_grouped_position() {
        for position in 0..4 {
            for invalid in ["1", "X + 1", "GetNumber()", "Fixed"] {
                let mut args = ["X"; 4];
                args[position] = invalid;
                let source = format!("CONSTANT Fixed = 1\nFUNCTION GetNumber() RETURNS INTEGER\nRETURN 1\nENDFUNCTION\nPROCEDURE P(BYREF A, B, C, D : INTEGER)\nENDPROCEDURE\nDECLARE X : INTEGER\nCALL P({})\n", args.join(", "));
                run_case(invalid, &source, "", "", Some("BYREF"), &[]);
            }
        }
    }

    #[test]
    fn byref_checks_exact_type_at_every_grouped_position() {
        for position in 0..4 {
            let mut args = ["X"; 4];
            args[position] = "Y";
            let source = format!("PROCEDURE P(BYREF A, B, C, D : INTEGER)\nENDPROCEDURE\nDECLARE X : INTEGER\nDECLARE Y : REAL\nY <- 1\nCALL P({})\n", args.join(", "));
            run_case(
                "grouped BYREF exact types",
                &source,
                "",
                "",
                Some(&format!(
                    "Type mismatch for BYREF parameter '{}'",
                    ["A", "B", "C", "D"][position]
                )),
                &[],
            );
        }
    }

    case!(
        two_dimensional_arrays_keep_bounds_and_copies,
        "commas in 2D types do not split parameter groups",
        r#"
PROCEDURE P(BYREF A, B : ARRAY[2:3,4:5] OF INTEGER, BYVAL C, D : ARRAY[2:3,4:5] OF INTEGER)
A[2,4] <- 10
B[3,5] <- 20
C[2,4] <- 30
D[3,5] <- 40
OUTPUT C[2,4], ":", C[3,5], ":", D[2,4], ":", D[3,5]
ENDPROCEDURE
DECLARE V : ARRAY[2:3,4:5] OF INTEGER
V[2,4] <- 1
V[3,5] <- 2
CALL P(V, V, V, V)
OUTPUT V[2,4], ":", V[3,5]
"#,
        "30:2:1:40\n10:20\n",
        "",
        None,
        &[]
    );

    #[test]
    fn grouped_arrays_validate_every_arguments_shape() {
        for mode in ["BYVAL", "BYREF"] {
            for wrong_type in [
                "ARRAY[1:3] OF INTEGER",
                "ARRAY[1:2,1:2] OF INTEGER",
                "ARRAY[1:2] OF STRING",
            ] {
                for position in 0..3 {
                    let mut args = ["V"; 3];
                    args[position] = "W";
                    let source = format!("PROCEDURE P({mode} A, B, C : ARRAY[1:2] OF INTEGER)\nENDPROCEDURE\nDECLARE V : ARRAY[1:2] OF INTEGER\nDECLARE W : {wrong_type}\nCALL P({})\n", args.join(", "));
                    run_case(
                        &format!("{mode} {wrong_type} at {position}"),
                        &source,
                        "",
                        "",
                        Some("Type mismatch"),
                        &[],
                    );
                }
            }
        }
    }

    case!(
        nested_calls_and_recursive_grouped_parameters,
        "grouped parameters remain separate across recursive frames",
        r#"
PROCEDURE Add(BYREF A, B : INTEGER)
A <- A + 1
B <- B + 2
ENDPROCEDURE
PROCEDURE Recur(BYREF A, B : INTEGER, BYVAL N, Step : INTEGER)
IF N > 0 THEN
CALL Add(A, B)
CALL Recur(A, B, N - Step, Step)
ENDIF
ENDPROCEDURE
DECLARE X, Y, N, S : INTEGER
N <- 3
S <- 1
CALL Recur(X, Y, N, S)
OUTPUT X, ":", Y, ":", N, ":", S
"#,
        "3:6:3:1\n",
        "",
        None,
        &[]
    );

    case!(
        grouped_function_arguments_evaluated_once_in_order,
        "each grouped actual argument is evaluated once in order",
        r#"
DECLARE Count : INTEGER
FUNCTION Next() RETURNS INTEGER
Count <- Count + 1
RETURN Count
ENDFUNCTION
FUNCTION Digits(A, B, C, D : INTEGER) RETURNS INTEGER
RETURN A * 1000 + B * 100 + C * 10 + D
ENDFUNCTION
OUTPUT Digits(Next(), Next(), Next(), Next())
OUTPUT Count
"#,
        "1234\n4\n",
        "",
        None,
        &[]
    );

    case!(
        grouped_modes_reset_for_next_declaration,
        "type groups and passing modes do not leak between declarations",
        r#"
PROCEDURE P(BYREF A, B : INTEGER)
A <- 10
B <- 20
ENDPROCEDURE
PROCEDURE Q(A, B : INTEGER)
A <- 30
B <- 40
ENDPROCEDURE
FUNCTION F(A, B : INTEGER) RETURNS INTEGER
A <- 50
B <- 60
RETURN A + B
ENDFUNCTION
DECLARE X, Y : INTEGER
CALL P(X, Y)
CALL Q(X, Y)
OUTPUT F(X, Y)
OUTPUT X, ":", Y
"#,
        "110\n10:20\n",
        "",
        None,
        &[]
    );

    case!(
        comments_and_line_breaks_inside_groups,
        "grouped parameters allow whitespace and comments",
        r#"
FUNCTION F(
BYVAL A, // first name
B // second name
: INTEGER,
C,
D : STRING
) RETURNS STRING
RETURN NUM_TO_STR(A + B) & C & D
ENDFUNCTION
OUTPUT F(1, 2, "x", "y")
"#,
        "3xy\n",
        "",
        None,
        &[]
    );

    #[test]
    fn long_groups_keep_all_parameter_names_and_order() {
        let names: Vec<_> = (0..64).map(|i| format!("P{i}")).collect();
        let args: Vec<_> = (0..64).map(|i| i.to_string()).collect();
        let body = names
            .iter()
            .map(|name| format!("OUTPUT {name}\n"))
            .collect::<String>();
        let source = format!(
            "PROCEDURE P({} : INTEGER)\n{body}ENDPROCEDURE\nCALL P({})\n",
            names.join(", "),
            args.join(", ")
        );
        run_case(
            "64 grouped parameters",
            &source,
            &(args.join("\n") + "\n"),
            "",
            None,
            &[],
        );
    }

    #[test]
    fn duplicate_names_rejected_across_every_pair_of_groups() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            for first in 0..6 {
                for second in first + 1..6 {
                    let mut names: Vec<_> = (0..6).map(|i| format!("P{i}")).collect();
                    names[second] = names[first].to_lowercase();
                    let params = format!(
                        "{}, {} : INTEGER, {}, {} : STRING, {}, {} : BOOLEAN",
                        names[0], names[1], names[2], names[3], names[4], names[5]
                    );
                    let source = if kind == "PROCEDURE" {
                        format!("PROCEDURE P({params})\nENDPROCEDURE\n")
                    } else {
                        format!("FUNCTION F({params}) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n")
                    };
                    run_case(
                        &params,
                        &source,
                        "",
                        "",
                        Some("2 parameters with the same name"),
                        &[],
                    );
                }
            }
        }
    }

    #[test]
    fn functions_reject_byref_in_all_group_positions() {
        for position in 0..5 {
            for boundaries in 0..16 {
                let mut markers = [""; 5];
                markers[position] = "BYREF";
                let params = signature(&markers, boundaries);
                let source =
                    format!("FUNCTION F({params}) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n");
                let error = match StepInterpreter::new(&source) {
                    Ok(_) => panic!("accepted {source}"),
                    Err(error) => error,
                };
                assert!(
                    error.contains("Parameters should not be passed by reference to a function"),
                    "{error}\n{source}"
                );
            }
        }
    }

    #[test]
    fn malformed_groups_fail_without_panicking() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            for params in [
                "A, B :",
                "A, B : INTEGER, C, D",
                "A, B : INTEGER, C,",
                "A, B : INTEGER,, C : STRING",
                "BYVAL",
                "BYVAL A, BYVAL : INTEGER",
                "A B : INTEGER",
                "A, B : INTEGER C, D : STRING",
                "A, B :: INTEGER",
                "A, B : ARRAY[1:2,] OF INTEGER",
            ] {
                let source = if kind == "PROCEDURE" {
                    format!("PROCEDURE P({params})\nENDPROCEDURE\n")
                } else {
                    format!("FUNCTION F({params}) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n")
                };
                run_case(
                    &format!("{kind}({params})"),
                    &source,
                    "",
                    "",
                    Some("Syntax Error"),
                    &[],
                );
            }
        }
    }

    #[test]
    fn truncated_groups_fail_without_panicking_or_hanging() {
        for source in [
            "PROCEDURE P(A,",
            "PROCEDURE P(A, B :",
            "FUNCTION F(A,",
            "FUNCTION F(A, B :",
            "PROCEDURE P(A, B : INTEGER, C,",
        ] {
            run_case(source, source, "", "", Some("Syntax Error"), &[]);
        }
    }

    case!(
        reject_procedure_trailing_comma_after_typed_group,
        "procedure parameter list must not end in a comma",
        "PROCEDURE P(A, B : INTEGER,)\nENDPROCEDURE\n",
        "",
        "",
        Some("Syntax Error"),
        &[]
    );

    case!(
        reject_function_trailing_comma_after_typed_group,
        "function parameter list must not end in a comma",
        "FUNCTION F(A, B : INTEGER,) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n",
        "",
        "",
        Some("Syntax Error"),
        &[]
    );

    #[test]
    fn grouped_input_targets_preserve_modes_during_replay() {
        let source = "PROCEDURE P(BYREF A, B : INTEGER, BYVAL C, D : INTEGER)\nINPUT A\nINPUT B\nINPUT C\nINPUT D\nOUTPUT A, \":\", B, \":\", C, \":\", D\nENDPROCEDURE\nDECLARE X, Y, Z, W : INTEGER\nCALL P(X, Y, Z, W)\nOUTPUT X, \":\", Y, \":\", Z, \":\", W\n";
        let mut runner = StepInterpreter::new(source).unwrap();
        for (name, value) in [("A", "1"), ("B", "2"), ("C", "3"), ("D", "4")] {
            for _ in 0..2 {
                assert!(
                    matches!(runner.step(), StepEvent::NeedsInput { variable } if variable == name)
                );
            }
            runner.supply_input(value.to_owned());
        }
        for expected in ["1:2:3:4", "1:2:0:0"] {
            assert!(matches!(runner.step(), StepEvent::Output { value } if value == expected));
        }
        assert!(matches!(runner.step(), StepEvent::Done));
    }
}
mod grouped_parameter_edges {
    use super::*;
    use cambridge_pseudocode_interpreter::{
        errortype::{CPSError, ErrorType},
        Inter::{
            cps::Type,
            step_interpreter::{StepEvent, StepInterpreter},
        },
        Lexer::lexer::Lexer,
        Parser::{
            ast::{Ast, PassingValue, Stmt},
            parser::Parser,
        },
    };

    fn parse(source: &str) -> Result<Vec<Ast>, CPSError> {
        let tokens = Lexer::new(source.to_owned()).tokenize()?;
        Parser::new(tokens, source.to_owned()).parse_statements()
    }

    fn outputs(source: &str, expected: &[String]) {
        let mut runner =
            StepInterpreter::new(source).unwrap_or_else(|error| panic!("{error}\n{source}"));
        for expected in expected {
            match runner.step() {
                StepEvent::Output { value } => assert_eq!(&value, expected, "{source}"),
                other => panic!("expected {expected:?}, got {other:?}\n{source}"),
            }
        }
        assert!(matches!(runner.step(), StepEvent::Done), "{source}");
    }

    #[test]
    fn all_three_type_group_combinations_keep_names_types_and_modes() {
        let types = [
            ("INTEGER", Type::Integer),
            ("REAL", Type::Real),
            ("STRING", Type::String),
            ("CHAR", Type::Char),
            ("BOOLEAN", Type::Boolean),
            ("DATE", Type::Date),
            ("Season", Type::Named("Season".to_owned())),
        ];
        for function in [false, true] {
            for (a, first) in &types {
                for (b, second) in &types {
                    for (c, third) in &types {
                        let (source, modes) = if function {
                            (format!("FUNCTION F(A, B, C : {a}, BYVAL D, E : {b}, F : {c}) RETURNS BOOLEAN\nRETURN TRUE\nENDFUNCTION\n"), [PassingValue::ByVal; 6])
                        } else {
                            (format!("PROCEDURE P(BYREF A, B, C : {a}, BYVAL D, E : {b}, BYREF F : {c})\nOUTPUT 1\nENDPROCEDURE\n"), [PassingValue::ByRef, PassingValue::ByRef, PassingValue::ByRef, PassingValue::ByVal, PassingValue::ByVal, PassingValue::ByRef])
                        };
                        let ast = parse(&source).unwrap();
                        assert_eq!(ast.len(), 1);
                        let statement = match &ast[0] {
                            Ast::Stmt(Stmt::At { inner, .. }) => inner.as_ref(),
                            other => panic!("{other:?}"),
                        };
                        let parameters = match statement {
                            Stmt::Function {
                                parameters,
                                return_type,
                                body,
                                ..
                            } => {
                                assert_eq!(*return_type, Type::Boolean);
                                assert_eq!(body.statements.len(), 1);
                                parameters
                            }
                            Stmt::Procedure {
                                parameters, body, ..
                            } => {
                                assert_eq!(body.statements.len(), 1);
                                parameters
                            }
                            other => panic!("{other:?}"),
                        };
                        let expected: Vec<_> = ["A", "B", "C", "D", "E", "F"]
                            .iter()
                            .zip([first, first, first, second, second, third])
                            .zip(modes)
                            .map(|((name, ty), mode)| ((*name).to_owned(), ty.clone(), mode))
                            .collect();
                        assert_eq!(*parameters, expected, "{source}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_shared_argument_partition_and_copy_mode() {
        for pattern in 0..256 {
            let slots = [
                pattern % 4,
                (pattern / 4) % 4,
                (pattern / 16) % 4,
                (pattern / 64) % 4,
            ];
            if slots[0] != 0 || (1..4).any(|i| slots[i] > 1 + slots[..i].iter().max().unwrap()) {
                continue;
            }
            for modes in 0..16 {
                let mut caller = [1, 2, 3, 4];
                let mut copies = slots.map(|slot| caller[slot]);
                for i in 0..4 {
                    if modes & (1 << i) != 0 {
                        caller[slots[i]] += 10 + i;
                    } else {
                        copies[i] += 10 + i;
                    }
                }
                let inside = (0..4)
                    .map(|i| {
                        if modes & (1 << i) != 0 {
                            caller[slots[i]]
                        } else {
                            copies[i]
                        }
                    })
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(":");
                let after = caller
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(":");
                let parameters = (0..4)
                    .map(|i| {
                        format!(
                            "{} P{i}",
                            if modes & (1 << i) != 0 {
                                "BYREF"
                            } else {
                                "BYVAL"
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let arguments = slots.map(|slot| format!("V{slot}")).join(", ");
                let source = format!("PROCEDURE P({parameters} : INTEGER)\nP0 <- P0 + 10\nP1 <- P1 + 11\nP2 <- P2 + 12\nP3 <- P3 + 13\nOUTPUT P0, \":\", P1, \":\", P2, \":\", P3\nENDPROCEDURE\nDECLARE V0, V1, V2, V3 : INTEGER\nV0 <- 1\nV1 <- 2\nV2 <- 3\nV3 <- 4\nCALL P({arguments})\nOUTPUT V0, \":\", V1, \":\", V2, \":\", V3\n");
                outputs(&source, &[inside, after]);
            }
        }
    }

    case!(
        whole_array_assignment_keeps_references_and_copies,
        "whole array writes preserve grouped parameter storage",
        r#"
PROCEDURE P(BYREF A, B : ARRAY[1:2] OF INTEGER, BYVAL C, D : ARRAY[1:2] OF INTEGER)
DECLARE Replacement : ARRAY[1:2] OF INTEGER
Replacement[1] <- 7
Replacement[2] <- 8
A <- Replacement
OUTPUT B[1], ":", B[2], ":", C[1], ":", C[2]
C <- Replacement
C[1] <- 99
OUTPUT A[1], ":", C[1], ":", D[1]
ENDPROCEDURE
DECLARE V : ARRAY[1:2] OF INTEGER
V[1] <- 1
V[2] <- 2
CALL P(V, V, V, V)
OUTPUT V[1], ":", V[2]
"#,
        "7:8:1:2\n7:99:1\n7:8\n",
        "",
        None,
        &[]
    );

    case!(
        array_of_enums_keeps_grouped_copies_independent,
        "enum array groups preserve base types and isolation",
        r#"
TYPE Season = (Spring, Summer, Winter)
PROCEDURE P(BYREF A, B : ARRAY[3:4] OF Season, BYVAL C, D : ARRAY[3:4] OF Season)
A[3] <- Summer
B[4] <- Winter
C[3] <- Winter
D[4] <- Summer
OUTPUT C[3], ":", C[4], ":", D[3], ":", D[4]
ENDPROCEDURE
DECLARE V : ARRAY[3:4] OF Season
V[3] <- Spring
V[4] <- Spring
CALL P(V, V, V, V)
OUTPUT V[3], ":", V[4]
"#,
        "Winter:Spring:Spring:Summer\nSummer:Winter\n",
        "",
        None,
        &[]
    );

    case!(
        grouped_function_can_return_an_array_copy,
        "grouped function parameters remain independent of the returned array",
        r#"
FUNCTION F(A, B : ARRAY[1:2] OF INTEGER) RETURNS ARRAY[1:2] OF INTEGER
A[1] <- 9
OUTPUT B[1], ":", B[2]
RETURN A
ENDFUNCTION
DECLARE V, R : ARRAY[1:2] OF INTEGER
V[1] <- 1
V[2] <- 2
R <- F(V, V)
R[2] <- 8
OUTPUT V[1], ":", V[2], ":", R[1], ":", R[2]
"#,
        "1:2\n1:2:9:8\n",
        "",
        None,
        &[]
    );

    case!(
        recursive_byval_arrays_are_isolated_per_frame,
        "recursive calls copy grouped arrays at each level",
        r#"
FUNCTION F(A, B : ARRAY[1:1] OF INTEGER, N : INTEGER) RETURNS INTEGER
IF N = 0 THEN
RETURN A[1] + B[1]
ENDIF
A[1] <- A[1] + N
RETURN F(A, B, N - 1) + A[1]
ENDFUNCTION
DECLARE V : ARRAY[1:1] OF INTEGER
V[1] <- 1
OUTPUT F(V, V, 2)
OUTPUT V[1]
"#,
        "12\n1\n",
        "",
        None,
        &[]
    );

    case!(
        grouped_parameter_names_shadow_globals,
        "parameter names do not overwrite unrelated globals",
        r#"
DECLARE A, B, X, Y : INTEGER
A <- 100
B <- 200
X <- 1
Y <- 2
PROCEDURE P(BYREF A, B : INTEGER)
A <- A + 10
B <- B + 20
ENDPROCEDURE
CALL P(X, Y)
OUTPUT A, ":", B, ":", X, ":", Y
"#,
        "100:200:11:22\n",
        "",
        None,
        &[]
    );

    case!(
        byref_and_byval_bind_in_argument_evaluation_order,
        "grouped references remain live while later arguments have side effects",
        r#"
DECLARE X : INTEGER
X <- 1
FUNCTION Change() RETURNS INTEGER
X <- 9
RETURN X
ENDFUNCTION
PROCEDURE P(BYREF A, BYVAL B, C : INTEGER)
OUTPUT A, ":", B, ":", C
ENDPROCEDURE
CALL P(X, X, Change())
OUTPUT X
"#,
        "9:1:9\n9\n",
        "",
        None,
        &[]
    );

    case!(
        byval_numeric_conversion_applies_to_each_grouped_name,
        "all grouped REAL parameters accept INTEGER copies",
        r#"
PROCEDURE P(A, B, C : REAL)
A <- A + 0.5
B <- B + 0.25
C <- C + 0.75
OUTPUT A, ":", B, ":", C
ENDPROCEDURE
DECLARE X, Y, Z : INTEGER
X <- 1
Y <- 2
Z <- 3
CALL P(X, Y, Z)
OUTPUT X, ":", Y, ":", Z
"#,
        "1.5:2.25:3.75\n1:2:3\n",
        "",
        None,
        &[]
    );

    #[test]
    fn same_length_arrays_with_different_bounds_are_rejected() {
        for mode in ["BYVAL", "BYREF"] {
            for position in 0..3 {
                let mut args = ["V"; 3];
                args[position] = "Wrong";
                let source = format!("PROCEDURE P({mode} A, B, C : ARRAY[2:3,4:5] OF INTEGER)\nENDPROCEDURE\nDECLARE V : ARRAY[2:3,4:5] OF INTEGER\nDECLARE Wrong : ARRAY[1:2,4:5] OF INTEGER\nCALL P({})\n", args.join(", "));
                run_case(
                    "grouped arrays require identical bounds",
                    &source,
                    "",
                    "",
                    Some("Type mismatch"),
                    &[],
                );
            }
        }
    }

    #[test]
    fn unknown_type_in_any_group_rejects_the_definition() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            for position in 0..3 {
                let mut types = ["INTEGER"; 3];
                types[position] = "MissingType";
                let params = format!(
                    "A, B : {}, C, D : {}, E, F : {}",
                    types[0], types[1], types[2]
                );
                let source = if kind == "PROCEDURE" {
                    format!("PROCEDURE P({params})\nENDPROCEDURE\n")
                } else {
                    format!("FUNCTION F({params}) RETURNS INTEGER\nRETURN 0\nENDFUNCTION\n")
                };
                run_case(kind, &source, "", "", Some("has not been defined"), &[]);
            }
        }
    }

    #[test]
    fn zero_and_single_parameter_declarations_still_work_after_groups() {
        let source = "PROCEDURE P(BYREF A, B : INTEGER)\nENDPROCEDURE\nPROCEDURE Q()\nOUTPUT 7\nENDPROCEDURE\nFUNCTION F(A : INTEGER) RETURNS INTEGER\nRETURN A\nENDFUNCTION\nFUNCTION G() RETURNS INTEGER\nRETURN 9\nENDFUNCTION\nCALL Q()\nOUTPUT F(8)\nOUTPUT G()\n";
        outputs(source, &["7".to_owned(), "8".to_owned(), "9".to_owned()]);
    }

    #[test]
    fn duplicate_parameter_error_points_to_repeated_name() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            let suffix = if kind == "PROCEDURE" {
                "ENDPROCEDURE\n"
            } else {
                "RETURN 0\nENDFUNCTION\n"
            };
            let returns = if kind == "FUNCTION" {
                " RETURNS INTEGER"
            } else {
                ""
            };
            let source = format!(
                "{kind} P(First, Second : INTEGER,\n    Third, fIrSt : STRING){returns}\n{suffix}"
            );
            let error = parse(&source).expect_err("duplicate name must be rejected");
            assert!(matches!(error.error_type, ErrorType::Syntax));
            assert!(error.message.contains("fIrSt"), "{error:?}");
            assert_eq!((error.line, error.column), (2, 12));
            assert_eq!(error.source.as_deref(), Some(source.as_str()));
        }
    }

    #[test]
    fn trailing_comma_diagnostic_points_to_the_comma() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            let source = format!(
                "{kind} P(A, B : INTEGER,\n C, D : STRING,\n){}\nEND{kind}\n",
                if kind == "FUNCTION" {
                    " RETURNS INTEGER"
                } else {
                    ""
                }
            );
            let error = parse(&source).expect_err("trailing comma must be rejected");
            assert!(matches!(error.error_type, ErrorType::Syntax));
            assert!(error.message.contains("Trailing comma"), "{error:?}");
            assert_eq!((error.line, error.column), (2, 15));
            assert_eq!(error.source.as_deref(), Some(source.as_str()));
        }
    }

    #[test]
    fn doubled_or_dangling_passing_markers_are_rejected() {
        for params in [
            "BYVAL BYVAL A, B : INTEGER",
            "BYREF BYVAL A, B : INTEGER",
            "A, BYREF BYREF B : INTEGER",
            "A, BYVAL BYREF B : INTEGER",
            "A, B : INTEGER, BYVAL",
            "A, B : INTEGER, BYREF",
            "A, BYREF : INTEGER",
        ] {
            let source = format!("PROCEDURE P({params})\nENDPROCEDURE\n");
            run_case(
                params,
                &source,
                "",
                "",
                Some("Expected parameter name"),
                &[],
            );
        }
    }

    #[test]
    fn every_token_boundary_in_an_incomplete_header_is_rejected() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            let header = format!(
                "{kind} P ( BYVAL A , B : ARRAY [ 1 : 2 , 3 : 4 ] OF INTEGER , C , D : STRING"
            );
            let tokens: Vec<_> = header.split_whitespace().collect();
            for end in 1..=tokens.len() {
                let source = tokens[..end].join(" ");
                run_case(
                    &format!("header truncated at token {end}"),
                    &source,
                    "",
                    "",
                    Some("Syntax Error"),
                    &[],
                );
            }
        }
    }

    #[test]
    fn incomplete_second_declaration_does_not_reuse_previous_parameters() {
        for kind in ["PROCEDURE", "FUNCTION"] {
            let source =
                format!("PROCEDURE Valid(A, B : INTEGER)\nENDPROCEDURE\n{kind} Broken(C, D :\n");
            run_case(kind, &source, "", "", Some("valid data type"), &[]);
        }
    }

    #[test]
    fn grouped_byref_input_errors_terminate_replay() {
        let source = "PROCEDURE P(BYREF A, B : INTEGER)\nINPUT B\nOUTPUT \"unreachable\"\nENDPROCEDURE\nDECLARE X, Y : INTEGER\nCALL P(X, Y)\n";
        let mut runner = StepInterpreter::new(source).unwrap();
        assert!(matches!(runner.step(), StepEvent::NeedsInput { variable } if variable == "B"));
        runner.supply_input("not an integer".to_owned());
        assert!(matches!(runner.step(), StepEvent::Error { .. }));
        assert!(matches!(runner.step(), StepEvent::Done));
    }
}
