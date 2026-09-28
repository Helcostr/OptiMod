//! Parallel JSONL fixture runner for chat monitoring module CI checks.
//!
//! Usage: `cm-fixture-runner fixtures/`

use cm_core::{Action, PLUGIN_MODULE_ID};
use cm_english::EnglishChecker;
use rayon::prelude::*;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const FIXTURE_REPORT_REL: &str = "target/fixture-report.json";

/// One parsed JSONL row ready to execute.
struct LoadedCase {
    file: String,
    line: usize,
    case: FixtureCase,
}

/// JSONL fixture row. When both are set, `input` wins over `input_file`.
#[derive(Debug, Deserialize)]
struct FixtureCase {
    module: String,
    input: Option<String>,
    input_file: Option<String>,
    expect: FixtureExpect,
}

#[derive(Debug, Deserialize)]
struct FixtureExpect {
    action: Action,
    conversion_min: Option<f32>,
    conversion_max: Option<f32>,
    obfuscation_min: Option<f32>,
    obfuscation_max: Option<f32>,
    english_ratio_min: Option<f32>,
    english_ratio_max: Option<f32>,
    word_count_min: Option<u32>,
}

#[derive(Debug, serde::Serialize)]
struct CaseReport {
    file: String,
    line: usize,
    module: String,
    passed: bool,
    message: String,
}

#[derive(Debug, serde::Serialize)]
struct FixtureReport {
    total: usize,
    passed: usize,
    failed: usize,
    cases: Vec<CaseReport>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn main() {
    let repo_root = repo_root();
    let fixtures_root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.join("fixtures"));

    let cases = load_cases(&fixtures_root, &repo_root);
    if cases.is_empty() {
        eprintln!("no fixture cases found under {}", fixtures_root.display());
        std::process::exit(1);
    }

    let reports: Vec<CaseReport> = cases
        .par_iter()
        .map(|loaded| run_case(loaded, &repo_root))
        .collect();

    let passed = reports.iter().filter(|r| r.passed).count();
    let report = FixtureReport {
        total: reports.len(),
        passed,
        failed: reports.len() - passed,
        cases: reports,
    };

    let out_path = repo_root.join(FIXTURE_REPORT_REL);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&report).expect("json");
    fs::write(&out_path, json).expect("write report");

    println!(
        "fixture report: {} passed, {} failed (written to {})",
        report.passed,
        report.failed,
        out_path.display()
    );

    if report.failed > 0 {
        for case_report in &report.cases {
            if !case_report.passed {
                eprintln!(
                    "FAIL {}:{} — {}",
                    case_report.file, case_report.line, case_report.message
                );
            }
        }
        std::process::exit(1);
    }
}

fn load_cases(fixtures_root: &Path, repo_root: &Path) -> Vec<LoadedCase> {
    let mut cases = Vec::new();
    if !fixtures_root.exists() {
        return cases;
    }

    for entry in walk_jsonl_files(fixtures_root) {
        let content = fs::read_to_string(&entry).expect("read fixture");
        let rel = entry
            .strip_prefix(repo_root)
            .unwrap_or(&entry)
            .display()
            .to_string();

        for (line_no, raw_line) in content.lines().enumerate() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let case: FixtureCase = serde_json::from_str(trimmed).expect("parse fixture line");
            cases.push(LoadedCase {
                file: rel.clone(),
                line: line_no + 1,
                case,
            });
        }
    }

    cases
}

fn walk_jsonl_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if dir.is_file() && dir.extension().is_some_and(|e| e == "jsonl") {
        files.push(dir.to_path_buf());
        return files;
    }
    if !dir.is_dir() {
        return files;
    }
    for entry in fs::read_dir(dir).expect("read fixtures dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files.extend(walk_jsonl_files(&path));
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            files.push(path);
        }
    }
    files
}

fn run_case(loaded: &LoadedCase, repo_root: &Path) -> CaseReport {
    let LoadedCase { file, line, case } = loaded;
    let input = match resolve_input(case, repo_root) {
        Ok(text) => text,
        Err(msg) => return fail(file, *line, &case.module, msg),
    };

    match case.module.as_str() {
        PLUGIN_MODULE_ID => run_english_case(file, *line, case, &input),
        other => fail(
            file,
            *line,
            &case.module,
            format!("unknown module: {}", other),
        ),
    }
}

fn run_english_case(file: &str, line: usize, case: &FixtureCase, input: &str) -> CaseReport {
    let checker = EnglishChecker::default();
    let result = checker.evaluate(input);

    if result.action != case.expect.action {
        return fail(
            file,
            line,
            &case.module,
            format!(
                "action: expected {}, got {} (conversion_ratio={:.3} english_ratio={:.3})",
                action_str(case.expect.action),
                action_str(result.action),
                result.conversion_ratio,
                result.english_ratio
            ),
        );
    }

    if let Some(report) = check_f32_min(
        file,
        line,
        &case.module,
        "conversion_ratio",
        result.conversion_ratio,
        case.expect.conversion_min,
    ) {
        return report;
    }
    if let Some(report) = check_f32_max(
        file,
        line,
        &case.module,
        "conversion_ratio",
        result.conversion_ratio,
        case.expect.conversion_max,
    ) {
        return report;
    }
    if let Some(report) = check_f32_min(
        file,
        line,
        &case.module,
        "obfuscation_ratio",
        result.obfuscation_ratio,
        case.expect.obfuscation_min,
    ) {
        return report;
    }
    if let Some(report) = check_f32_max(
        file,
        line,
        &case.module,
        "obfuscation_ratio",
        result.obfuscation_ratio,
        case.expect.obfuscation_max,
    ) {
        return report;
    }
    if let Some(report) = check_f32_min(
        file,
        line,
        &case.module,
        "english_ratio",
        result.english_ratio,
        case.expect.english_ratio_min,
    ) {
        return report;
    }
    if let Some(report) = check_f32_max(
        file,
        line,
        &case.module,
        "english_ratio",
        result.english_ratio,
        case.expect.english_ratio_max,
    ) {
        return report;
    }
    if let Some(min) = case.expect.word_count_min {
        if result.obfuscated_word_count < min {
            return fail(
                file,
                line,
                &case.module,
                format!(
                    "obfuscated_word_count {} < min {}",
                    result.obfuscated_word_count, min
                ),
            );
        }
    }

    case_report(
        true,
        file,
        line,
        &case.module,
        format!(
            "ok action={} obfuscation_ratio={:.3} english_ratio={:.3} obfuscated_word_count={}",
            action_str(result.action),
            result.obfuscation_ratio,
            result.english_ratio,
            result.obfuscated_word_count
        ),
    )
}

fn check_f32_min(
    file: &str,
    line: usize,
    module: &str,
    label: &str,
    actual: f32,
    bound: Option<f32>,
) -> Option<CaseReport> {
    bound.filter(|min| actual < *min).map(|min| {
        fail(
            file,
            line,
            module,
            format!("{} {:.3} < min {:.3}", label, actual, min),
        )
    })
}

fn check_f32_max(
    file: &str,
    line: usize,
    module: &str,
    label: &str,
    actual: f32,
    bound: Option<f32>,
) -> Option<CaseReport> {
    bound.filter(|max| actual > *max).map(|max| {
        fail(
            file,
            line,
            module,
            format!("{} {:.3} > max {:.3}", label, actual, max),
        )
    })
}

fn resolve_input(case: &FixtureCase, repo_root: &Path) -> Result<String, String> {
    if let Some(text) = &case.input {
        return Ok(text.clone());
    }
    if let Some(path) = &case.input_file {
        let full = repo_root.join(path);
        return fs::read_to_string(&full)
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("read {}: {}", path, e));
    }
    Err("fixture must specify input or input_file".into())
}

fn case_report(passed: bool, file: &str, line: usize, module: &str, message: String) -> CaseReport {
    CaseReport {
        file: file.to_string(),
        line,
        module: module.to_string(),
        passed,
        message,
    }
}

fn fail(file: &str, line: usize, module: &str, message: String) -> CaseReport {
    case_report(false, file, line, module, message)
}

fn action_str(action: Action) -> &'static str {
    match action {
        Action::Pass => "pass",
        Action::Flag => "flag",
        Action::Block => "block",
    }
}
