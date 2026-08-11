//! Fail-closed installation-site coverage audit for the pinned JP-to-EN patch.
//!
//! The English patch is a discovery and comparison reference, never an input
//! to the canonical JP-to-KR ROM build. This module validates the committed
//! semantic coverage manifest on every test run and can additionally compare
//! the 195 original-ROM installation/write sites with an explicitly supplied
//! checkout of the pinned English assembly. The monolithic `org newCodePos`
//! payload origin is a container for those features, not a separate hook.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

const MANIFEST_FILE: &str = "en_patch_coverage.json";
const PINNED_REPOSITORY: &str = "https://github.com/suppertails66/madou1mdtools.git";
const PINNED_COMMIT: &str = "6df898c10f11bc60e82f52665f51090a400cbaa9";
const PINNED_ASM_PATH: &str = "madou1md/asm/madou1md.asm";
const PINNED_ASM_SHA256: &str = "01b8d1b07e772a3a6769727ff445ac626ac9afe50127bb67f1facc22bdcd555f";
const PINNED_INSTALLATION_SITES: usize = 195;
const PINNED_PAYLOAD_ORIGIN: &str = "newCodePos";
const PINNED_TOTAL_ORG_DIRECTIVES: usize = 196;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnPatchCoverageManifest {
    schema_version: u32,
    source: EnPatchSource,
    expected_groups: usize,
    expected_counts: ExpectedCounts,
    groups: Vec<CoverageGroup>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnPatchSource {
    repository: String,
    commit: String,
    path: String,
    sha256: String,
    installation_region_end_marker: String,
    expected_installation_sites: usize,
    payload_origin_sites: Vec<String>,
    expected_total_org_directives: usize,
    role: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCounts {
    kr_reimplemented: usize,
    jp_preserved: usize,
    en_only_drop: usize,
    optional_fix: usize,
    open: usize,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
enum CoverageDisposition {
    KrReimplemented,
    JpPreserved,
    EnOnlyDrop,
    OptionalFix,
    Open,
}

impl CoverageDisposition {
    fn label(self) -> &'static str {
        match self {
            Self::KrReimplemented => "kr_reimplemented",
            Self::JpPreserved => "jp_preserved",
            Self::EnOnlyDrop => "en_only_drop",
            Self::OptionalFix => "optional_fix",
            Self::Open => "open",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoverageGroup {
    id: String,
    title: String,
    disposition: CoverageDisposition,
    decision: String,
    evidence: Vec<String>,
    sites: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnPatchCoverageReport {
    pub source_commit: String,
    pub source_sha256: String,
    pub groups: usize,
    pub sites: usize,
    pub payload_origins: usize,
    pub total_org_directives: usize,
    pub kr_reimplemented: usize,
    pub jp_preserved: usize,
    pub en_only_drop: usize,
    pub optional_fix: usize,
    pub open: usize,
    pub source_verified: bool,
}

impl EnPatchCoverageReport {
    pub fn render(&self) -> String {
        format!(
            "EN patch installation coverage:\n\
             source_commit={}\n\
             source_sha256={}\n\
             groups={}\n\
             installation_sites={}\n\
             payload_origins={}\n\
             total_org_directives={}\n\
             kr_reimplemented={}\n\
             jp_preserved={}\n\
             en_only_drop={}\n\
             optional_fix={}\n\
             open={}\n\
             source_verified={}\n",
            self.source_commit,
            self.source_sha256,
            self.groups,
            self.sites,
            self.payload_origins,
            self.total_org_directives,
            self.kr_reimplemented,
            self.jp_preserved,
            self.en_only_drop,
            self.optional_fix,
            self.open,
            self.source_verified,
        )
    }
}

pub fn audit_en_patch_coverage(
    assets_dir: &Path,
    asm_path: Option<&Path>,
) -> Result<EnPatchCoverageReport, String> {
    let manifest = load_manifest(assets_dir)?;
    let mut report = validate_manifest(assets_dir, &manifest)?;
    if let Some(asm_path) = asm_path {
        validate_pinned_asm(&manifest, asm_path)?;
        report.source_verified = true;
    }
    Ok(report)
}

fn load_manifest(assets_dir: &Path) -> Result<EnPatchCoverageManifest, String> {
    let path = assets_dir.join(MANIFEST_FILE);
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

fn validate_manifest(
    assets_dir: &Path,
    manifest: &EnPatchCoverageManifest,
) -> Result<EnPatchCoverageReport, String> {
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported EN patch coverage schema {}",
            manifest.schema_version
        ));
    }
    validate_source_identity(&manifest.source)?;
    if manifest.expected_groups != manifest.groups.len() {
        return Err(format!(
            "EN patch group count drifted: expected {}, got {}",
            manifest.expected_groups,
            manifest.groups.len()
        ));
    }

    let repository_root = assets_dir.parent().ok_or_else(|| {
        format!(
            "assets directory {} has no repository parent",
            assets_dir.display()
        )
    })?;
    let mut group_ids = BTreeSet::new();
    let mut site_owners = BTreeMap::new();
    let mut counts = BTreeMap::new();

    for group in &manifest.groups {
        if !group_ids.insert(group.id.as_str()) {
            return Err(format!("duplicate EN patch group ID {}", group.id));
        }
        if !group.id.starts_with("ENP-")
            || group.title.trim().is_empty()
            || group.decision.trim().is_empty()
            || group.sites.is_empty()
            || group.evidence.is_empty()
        {
            return Err(format!("incomplete EN patch coverage group {}", group.id));
        }

        for evidence in &group.evidence {
            validate_relative_evidence_path(repository_root, evidence, &group.id)?;
        }

        *counts.entry(group.disposition).or_insert(0usize) += group.sites.len();
        for site in &group.sites {
            let canonical = canonical_org(site)?;
            if let Some(previous) = site_owners.insert(canonical.clone(), group.id.as_str()) {
                return Err(format!(
                    "EN patch site {canonical} is owned by both {previous} and {}",
                    group.id
                ));
            }
        }
    }

    let site_count = site_owners.len();
    if site_count != manifest.source.expected_installation_sites {
        return Err(format!(
            "EN patch site count drifted: expected {}, got {site_count}",
            manifest.source.expected_installation_sites
        ));
    }

    let actual = |disposition| counts.get(&disposition).copied().unwrap_or(0);
    let expected = &manifest.expected_counts;
    let expected_pairs = [
        (
            CoverageDisposition::KrReimplemented,
            expected.kr_reimplemented,
        ),
        (CoverageDisposition::JpPreserved, expected.jp_preserved),
        (CoverageDisposition::EnOnlyDrop, expected.en_only_drop),
        (CoverageDisposition::OptionalFix, expected.optional_fix),
        (CoverageDisposition::Open, expected.open),
    ];
    for (disposition, expected_count) in expected_pairs {
        let actual_count = actual(disposition);
        if actual_count != expected_count {
            return Err(format!(
                "EN patch {} count drifted: expected {expected_count}, got {actual_count}",
                disposition.label()
            ));
        }
    }
    if expected.open != 0 || actual(CoverageDisposition::Open) != 0 {
        return Err("EN patch coverage still contains open sites".to_string());
    }

    Ok(EnPatchCoverageReport {
        source_commit: manifest.source.commit.clone(),
        source_sha256: manifest.source.sha256.clone(),
        groups: manifest.groups.len(),
        sites: site_count,
        payload_origins: manifest.source.payload_origin_sites.len(),
        total_org_directives: manifest.source.expected_total_org_directives,
        kr_reimplemented: actual(CoverageDisposition::KrReimplemented),
        jp_preserved: actual(CoverageDisposition::JpPreserved),
        en_only_drop: actual(CoverageDisposition::EnOnlyDrop),
        optional_fix: actual(CoverageDisposition::OptionalFix),
        open: actual(CoverageDisposition::Open),
        source_verified: false,
    })
}

fn validate_source_identity(source: &EnPatchSource) -> Result<(), String> {
    if source.repository != PINNED_REPOSITORY
        || source.commit != PINNED_COMMIT
        || source.path != PINNED_ASM_PATH
        || source.sha256 != PINNED_ASM_SHA256
        || source.installation_region_end_marker != "* New code"
        || source.expected_installation_sites != PINNED_INSTALLATION_SITES
        || source.payload_origin_sites != [PINNED_PAYLOAD_ORIGIN]
        || source.expected_total_org_directives != PINNED_TOTAL_ORG_DIRECTIVES
        || source.role != "comparison_reference_only_not_jp_kr_build_input"
    {
        return Err("EN patch source identity drifted".to_string());
    }
    Ok(())
}

fn validate_relative_evidence_path(
    repository_root: &Path,
    evidence: &str,
    group_id: &str,
) -> Result<(), String> {
    let path = Path::new(evidence);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!("{group_id} has unsafe evidence path {evidence:?}"));
    }
    if !repository_root.join(path).is_file() {
        return Err(format!(
            "{group_id} evidence path does not exist: {evidence}"
        ));
    }
    Ok(())
}

fn validate_pinned_asm(manifest: &EnPatchCoverageManifest, asm_path: &Path) -> Result<(), String> {
    let bytes = fs::read(asm_path)
        .map_err(|error| format!("failed to read EN assembly {}: {error}", asm_path.display()))?;
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != manifest.source.sha256 {
        return Err(format!(
            "EN assembly SHA-256 drifted: expected {}, got {actual_sha256}",
            manifest.source.sha256
        ));
    }
    let source = std::str::from_utf8(&bytes)
        .map_err(|error| format!("EN assembly is not UTF-8: {error}"))?;
    let (extracted, payload_origins) =
        extract_org_regions(source, &manifest.source.installation_region_end_marker)?;
    let expected = manifest
        .groups
        .iter()
        .flat_map(|group| &group.sites)
        .map(|site| canonical_org(site))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let actual = extracted.into_iter().collect::<BTreeSet<_>>();
    let expected_payload_origins = manifest
        .source
        .payload_origin_sites
        .iter()
        .map(|site| canonical_org(site))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let actual_payload_origins = payload_origins.into_iter().collect::<BTreeSet<_>>();

    if actual != expected {
        let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
        let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();
        return Err(format!(
            "EN assembly coverage differs from the manifest: missing={missing:?} unexpected={unexpected:?}"
        ));
    }
    if actual_payload_origins != expected_payload_origins {
        return Err(format!(
            "EN assembly payload origins differ from the manifest: expected={expected_payload_origins:?} actual={actual_payload_origins:?}"
        ));
    }
    Ok(())
}

fn extract_org_regions(
    source: &str,
    end_marker: &str,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut installation_sites = Vec::new();
    let mut payload_origins = Vec::new();
    let mut found_end = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == end_marker {
            found_end = true;
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('*') {
            continue;
        }
        let mut fields = trimmed.split_whitespace();
        let Some(directive) = fields.next() else {
            continue;
        };
        if !directive.eq_ignore_ascii_case("org") {
            continue;
        }
        let expression = fields
            .next()
            .ok_or_else(|| format!("EN assembly has org without an expression: {line:?}"))?;
        if fields.next().is_some() {
            return Err(format!(
                "EN assembly has unsupported trailing org syntax: {line:?}"
            ));
        }
        let site = canonical_org(expression)?;
        if found_end {
            payload_origins.push(site);
        } else {
            installation_sites.push(site);
        }
    }
    if !found_end {
        return Err(format!(
            "EN assembly active-region marker {end_marker:?} was not found"
        ));
    }
    let total = installation_sites
        .iter()
        .chain(&payload_origins)
        .collect::<Vec<_>>();
    let unique = total.iter().collect::<BTreeSet<_>>();
    if unique.len() != total.len() {
        return Err("EN assembly contains duplicate org sites".to_string());
    }
    Ok((installation_sites, payload_origins))
}

fn canonical_org(expression: &str) -> Result<String, String> {
    let canonical = expression.trim().replace(' ', "").to_ascii_uppercase();
    if canonical.is_empty()
        || !canonical
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'+'))
    {
        return Err(format!("invalid EN assembly org expression {expression:?}"));
    }
    Ok(canonical)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
