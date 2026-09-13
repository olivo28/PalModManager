//! FOMOD Version Resolution with Tri-Source Consensus
//!
//! Evaluates up to three distinct sources:
//! 1. `info.xml` inside FOMOD
//! 2. Zip archive filename
//! 3. Nexus API / cached metadata
//!
//! Applies majority consensus ("more =") to resolve discrepancies,
//! falling back to the newest valid version if all sources disagree.

/// Normalizes a version string by stripping leading 'v'/'V' and trimming whitespace.
pub fn normalize_version_token(v: &str) -> String {
    let trimmed = v.trim();
    if trimmed.starts_with('v') || trimmed.starts_with('V') {
        trimmed[1..].trim().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Checks if a version candidate is valid (not empty or placeholder).
pub fn is_valid_version_candidate(v: &str) -> bool {
    let t = v.trim().to_lowercase();
    !t.is_empty() && t != "unknown" && t != "null" && t != "none"
}

/// Resolves the most accurate version using majority consensus among candidate sources.
pub fn resolve_fomod_version_consensus(
    xml_ver: Option<&str>,
    zip_ver: Option<&str>,
    api_ver: Option<&str>,
) -> String {
    let mut candidates: Vec<String> = Vec::new();

    if let Some(x) = xml_ver.filter(|s| is_valid_version_candidate(s)) {
        candidates.push(x.trim().to_string());
    }
    if let Some(z) = zip_ver.filter(|s| is_valid_version_candidate(s)) {
        candidates.push(z.trim().to_string());
    }
    if let Some(a) = api_ver.filter(|s| is_valid_version_candidate(s)) {
        candidates.push(a.trim().to_string());
    }

    if candidates.is_empty() {
        return "1.0.0".to_string();
    }
    if candidates.len() == 1 {
        return candidates[0].clone();
    }

    let norm_xml = xml_ver.filter(|s| is_valid_version_candidate(s)).map(normalize_version_token);
    let norm_zip = zip_ver.filter(|s| is_valid_version_candidate(s)).map(normalize_version_token);
    let norm_api = api_ver.filter(|s| is_valid_version_candidate(s)).map(normalize_version_token);

    // 1. Majority consensus checks:
    // If XML == API -> XML/API wins
    if let (Some(x), Some(a)) = (&norm_xml, &norm_api) {
        if x == a {
            return xml_ver.unwrap().trim().to_string();
        }
    }

    // If XML == ZIP -> XML/ZIP wins
    if let (Some(x), Some(z)) = (&norm_xml, &norm_zip) {
        if x == z {
            return xml_ver.unwrap().trim().to_string();
        }
    }

    // If ZIP == API -> ZIP/API wins
    if let (Some(z), Some(a)) = (&norm_zip, &norm_api) {
        if z == a {
            return zip_ver.unwrap().trim().to_string();
        }
    }

    // 2. Disagreement / No majority -> Pick highest/newest valid candidate
    let mut best = candidates[0].clone();
    for cand in candidates.iter().skip(1) {
        if crate::commands::nexus_commands::is_version_newer(&best, cand) {
            best = cand.clone();
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_xml_api_over_bad_zip() {
        // API=2.2, XML=2.2, ZIP=4 -> 2.2
        let res = resolve_fomod_version_consensus(Some("2.2"), Some("4"), Some("2.2"));
        assert_eq!(res, "2.2");
    }

    #[test]
    fn test_consensus_zip_api_over_stale_xml() {
        // ZIP=2.2, API=2.2, XML=1.0 -> 2.2
        let res = resolve_fomod_version_consensus(Some("1.0"), Some("2.2"), Some("2.2"));
        assert_eq!(res, "2.2");
    }

    #[test]
    fn test_consensus_zip_xml_when_api_missing() {
        // ZIP=2.2, XML=2.2, API=None -> 2.2
        let res = resolve_fomod_version_consensus(Some("2.2"), Some("2.2"), None);
        assert_eq!(res, "2.2");
    }

    #[test]
    fn test_disagreement_picks_highest() {
        // ZIP=2.1, XML=2.2, API=2.0 -> 2.2
        let res = resolve_fomod_version_consensus(Some("2.2"), Some("2.1"), Some("2.0"));
        assert_eq!(res, "2.2");

        // ZIP=3.0, XML=2.2, API=None -> 3.0
        let res = resolve_fomod_version_consensus(Some("2.2"), Some("3.0"), None);
        assert_eq!(res, "3.0");
    }

    #[test]
    fn test_v_prefix_normalization() {
        // ZIP=v2.2, XML=2.2 -> match!
        let res = resolve_fomod_version_consensus(Some("2.2"), Some("v2.2"), None);
        assert_eq!(res, "2.2");
    }

    #[test]
    fn test_single_candidate() {
        let res = resolve_fomod_version_consensus(Some("2.2"), None, None);
        assert_eq!(res, "2.2");

        let res = resolve_fomod_version_consensus(None, Some("1.5.0"), None);
        assert_eq!(res, "1.5.0");
    }
}
