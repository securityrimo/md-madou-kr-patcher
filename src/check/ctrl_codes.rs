//! JP-source-first vs KR control-code comparison.
//!
//! Python `check_ctrl_codes.py`의 Rust 포팅.
//! 심각도별 보고: CRITICAL / HIGH / MEDIUM / LOW

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::build::text::{Token, parse_display_text};
use crate::translation;

// ============================================================
// Control code extraction
// ============================================================

/// Extracted control code: (code, optional parameter)
type CtrlCode = (u16, Option<u16>);

/// Extract control codes from display text via tokenizer.
fn extract_ctrl_codes(text: &str) -> Vec<CtrlCode> {
    let tokens = parse_display_text(text);
    let mut codes = Vec::new();
    for token in tokens {
        match token {
            Token::Ctrl(code) => codes.push((code, None)),
            Token::CtrlParam(code, param) => codes.push((code, Some(param))),
            _ => {}
        }
    }
    codes
}

/// Count occurrences of a specific control code.
fn code_count(codes: &[CtrlCode], target: u16) -> usize {
    codes.iter().filter(|(c, _)| *c == target).count()
}

/// Set of (code, param) for a specific control code.
fn code_set(codes: &[CtrlCode], target: u16) -> HashSet<(u16, Option<u16>)> {
    codes
        .iter()
        .filter(|(c, _)| *c == target)
        .copied()
        .collect()
}

// ============================================================
// Issue types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    fn label(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Issue {
    severity: Severity,
    file: String,
    key: String,
    code: String,
    issue: String,
    source_label: &'static str,
    source_tail: Option<String>,
    kr_tail: Option<String>,
}

// ============================================================
// Translation loading (delegates to shared translation module)
// ============================================================

// ============================================================
// Ending sequence extraction
// ============================================================

/// Extract the ending control code sequence (after last {PAGE}).
fn extract_ending_sequence(text: &str) -> &str {
    if let Some(pos) = text.rfind("{PAGE}") {
        &text[pos + 6..]
    } else {
        text
    }
}

/// JP is authoritative for the JP→KR pipeline. EN remains a fallback only for
/// synthetic entries that do not have a JP source string.
fn control_reference(entry: &translation::PairedEntry) -> (&'static str, &str) {
    if entry.jp.is_empty() {
        ("EN fallback", &entry.en)
    } else {
        ("JP", &entry.jp)
    }
}

// ============================================================
// Main entry point
// ============================================================

pub fn run(assets_dir: &Path) -> Result<(), String> {
    println!("{}", "=".repeat(70));
    println!("JP 우선 원문 vs KR 제어코드 비교 검사");
    println!("{}", "=".repeat(70));

    let paired_entries = translation::load_paired_entries(assets_dir)?;

    println!("\npaired entries: {}", paired_entries.len());

    let mut issues: Vec<Issue> = Vec::new();

    for entry in &paired_entries {
        let key = &entry.key;
        let fname = &entry.file;
        let (source_label, source_text) = control_reference(entry);
        let kr_text = &entry.ko;

        if source_text.is_empty() {
            continue;
        }

        let source_codes = extract_ctrl_codes(source_text);
        let kr_codes = extract_ctrl_codes(kr_text);

        // --- CRITICAL: Missing FFCC ---
        let source_ffcc = code_count(&source_codes, 0xFFCC);
        let kr_ffcc = code_count(&kr_codes, 0xFFCC);
        if source_ffcc > kr_ffcc {
            let source_tail: String = source_text
                .chars()
                .rev()
                .take(120)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let kr_tail: String = kr_text
                .chars()
                .rev()
                .take(120)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            issues.push(Issue {
                severity: Severity::Critical,
                file: fname.clone(),
                key: key.clone(),
                code: "FFCC".to_string(),
                issue: format!("FFCC (컷씬 종료) 누락: {source_label}={source_ffcc} KR={kr_ffcc}"),
                source_label,
                source_tail: Some(source_tail),
                kr_tail: Some(kr_tail),
            });
        }

        // --- HIGH: Missing FF84 branch codes ---
        let source_ff84 = code_set(&source_codes, 0xFF84);
        let kr_ff84 = code_set(&kr_codes, 0xFF84);
        let missing_ff84: HashSet<_> = source_ff84.difference(&kr_ff84).copied().collect();
        if !missing_ff84.is_empty() {
            let mut params: Vec<String> = missing_ff84
                .iter()
                .map(|(_, p)| match p {
                    Some(v) => format!("{v:04X}"),
                    None => "????".to_string(),
                })
                .collect();
            params.sort();
            issues.push(Issue {
                severity: Severity::High,
                file: fname.clone(),
                key: key.clone(),
                code: "FF84".to_string(),
                issue: format!("FF84 (이벤트 트리거) 누락: {}", params.join(", ")),
                source_label,
                source_tail: None,
                kr_tail: None,
            });
        }

        // --- HIGH: Missing FFAC animation ---
        let source_ffac = code_count(&source_codes, 0xFFAC);
        let kr_ffac = code_count(&kr_codes, 0xFFAC);
        if source_ffac > kr_ffac {
            issues.push(Issue {
                severity: Severity::High,
                file: fname.clone(),
                key: key.clone(),
                code: "FFAC".to_string(),
                issue: format!("FFAC (애니메이션) 누락: {source_label}={source_ffac} KR={kr_ffac}"),
                source_label,
                source_tail: None,
                kr_tail: None,
            });
        }

        // --- MEDIUM: Missing scene transitions ---
        for (scode, sname) in [
            (0xFF60u16, "FF60 씬열기"),
            (0xFF64, "FF64 씬닫기"),
            (0xFF68, "FF68 씬열기2"),
            (0xFF6C, "FF6C 씬닫기2"),
        ] {
            let source_n = code_count(&source_codes, scode);
            let kr_n = code_count(&kr_codes, scode);
            if source_n > kr_n {
                issues.push(Issue {
                    severity: Severity::Medium,
                    file: fname.clone(),
                    key: key.clone(),
                    code: format!("{scode:04X}"),
                    issue: format!("{sname} 누락: {source_label}={source_n} KR={kr_n}"),
                    source_label,
                    source_tail: None,
                    kr_tail: None,
                });
            }
        }

        // --- MEDIUM: Missing FFB0 display control ---
        let source_ffb0 = code_count(&source_codes, 0xFFB0);
        let kr_ffb0 = code_count(&kr_codes, 0xFFB0);
        if source_ffb0 > kr_ffb0 {
            issues.push(Issue {
                severity: Severity::Medium,
                file: fname.clone(),
                key: key.clone(),
                code: "FFB0".to_string(),
                issue: format!("FFB0 (디스플레이) 누락: {source_label}={source_ffb0} KR={kr_ffb0}"),
                source_label,
                source_tail: None,
                kr_tail: None,
            });
        }

        // --- LOW: Missing FFF4 ---
        let source_fff4 = code_count(&source_codes, 0xFFF4);
        let kr_fff4 = code_count(&kr_codes, 0xFFF4);
        if source_fff4 > kr_fff4 {
            issues.push(Issue {
                severity: Severity::Low,
                file: fname.clone(),
                key: key.clone(),
                code: "FFF4".to_string(),
                issue: format!("FFF4 누락: {source_label}={source_fff4} KR={kr_fff4}"),
                source_label,
                source_tail: None,
                kr_tail: None,
            });
        }

        // --- LOW: Missing FFB8 ---
        let source_ffb8 = code_count(&source_codes, 0xFFB8);
        let kr_ffb8 = code_count(&kr_codes, 0xFFB8);
        if source_ffb8 > kr_ffb8 {
            issues.push(Issue {
                severity: Severity::Low,
                file: fname.clone(),
                key: key.clone(),
                code: "FFB8".to_string(),
                issue: format!(
                    "FFB8 (텍스트 표시) 누락: {source_label}={source_ffb8} KR={kr_ffb8}"
                ),
                source_label,
                source_tail: None,
                kr_tail: None,
            });
        }

        // --- HIGH: Ending sequence mismatch (FFCC/FF84/FFAC) ---
        let source_ending = extract_ending_sequence(source_text);
        let kr_ending = extract_ending_sequence(kr_text);
        if source_ending != kr_ending {
            let source_end_codes: HashSet<u16> = extract_ctrl_codes(source_ending)
                .iter()
                .map(|(c, _)| *c)
                .collect();
            let kr_end_codes: HashSet<u16> = extract_ctrl_codes(kr_ending)
                .iter()
                .map(|(c, _)| *c)
                .collect();
            let diff_codes: HashSet<_> = source_end_codes
                .difference(&kr_end_codes)
                .copied()
                .collect();
            let critical_codes: HashSet<u16> = [0xFFCC, 0xFF84, 0xFFAC].into();
            if !diff_codes.is_disjoint(&critical_codes) {
                issues.push(Issue {
                    severity: Severity::High,
                    file: fname.clone(),
                    key: key.clone(),
                    code: "ENDING".to_string(),
                    issue: "종료 시퀀스 불일치".to_string(),
                    source_label,
                    source_tail: Some(source_ending.to_string()),
                    kr_tail: Some(kr_ending.to_string()),
                });
            }
        }
    }

    // Sort by severity then key
    issues.sort_by(|a, b| a.severity.cmp(&b.severity).then_with(|| a.key.cmp(&b.key)));

    // Summary
    let mut counts: HashMap<Severity, usize> = HashMap::new();
    for issue in &issues {
        *counts.entry(issue.severity).or_insert(0) += 1;
    }

    println!("\n{}", "=".repeat(70));
    println!("결과 요약");
    println!("{}", "=".repeat(70));
    println!(
        "  CRITICAL: {}건",
        counts.get(&Severity::Critical).unwrap_or(&0)
    );
    println!(
        "  HIGH:     {}건",
        counts.get(&Severity::High).unwrap_or(&0)
    );
    println!(
        "  MEDIUM:   {}건",
        counts.get(&Severity::Medium).unwrap_or(&0)
    );
    println!("  LOW:      {}건", counts.get(&Severity::Low).unwrap_or(&0));
    println!("  총:       {}건", issues.len());

    // Detail
    for sev in [
        Severity::Critical,
        Severity::High,
        Severity::Medium,
        Severity::Low,
    ] {
        let sev_issues: Vec<_> = issues.iter().filter(|i| i.severity == sev).collect();
        if sev_issues.is_empty() {
            continue;
        }
        println!("\n{}", "=".repeat(70));
        println!("[{}] ({}건)", sev, sev_issues.len());
        println!("{}", "=".repeat(70));
        for issue in sev_issues {
            println!("\n  {} / {}", issue.file, issue.key);
            println!("    {}", issue.issue);
            if let (Some(source_tail), Some(kr_tail)) = (&issue.source_tail, &issue.kr_tail) {
                let source_display: String = source_tail
                    .chars()
                    .rev()
                    .take(100)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                let kr_display: String = kr_tail
                    .chars()
                    .rev()
                    .take(100)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                println!("    {}: ...{source_display}", issue.source_label);
                println!("    KR: ...{kr_display}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "ctrl_codes_tests.rs"]
mod tests;
