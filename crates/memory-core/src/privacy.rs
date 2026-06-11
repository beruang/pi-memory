use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use super::observation::{MemorySensitivity, Observation};

static SECRET_PATTERNS: LazyLock<Vec<(Regex, &str)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r#"(?i)(api[_-]?key|apikey|api_secret)\s*[:=]\s*['\x22]?[\w\-_]{20,}['\x22]?"#).unwrap(),
            "API key pattern",
        ),
        (
            Regex::new(r#"(?i)(access[_-]?token|auth[_-]?token)\s*[:=]\s*['\x22]?[\w\-_\.]{20,}['\x22]?"#).unwrap(),
            "Access token pattern",
        ),
        (
            Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*['\x22]?[^\s'\x22]{4,}['\x22]?"#).unwrap(),
            "Password pattern",
        ),
        (
            Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
            "Private key header",
        ),
        (
            Regex::new(r"eyJ[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+\.?[A-Za-z0-9\-_.+/=]*").unwrap(),
            "JWT pattern",
        ),
        (
            Regex::new(r"(?i)(postgres(ql)?|mysql|mongodb(\+srv)?|redis)://[^\s]+@[^\s]+").unwrap(),
            "Database connection string with credentials",
        ),
        (
            Regex::new(r"(?i)(aws_access_key_id|aws_secret_access_key|session_token)\s*[:=]").unwrap(),
            "AWS credential keys",
        ),
        (
            Regex::new(r"(?i)sk-[A-Za-z0-9]{32,}").unwrap(),
            "OpenAI-style API key",
        ),
    ]
});

static PRIVATE_BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<private>.*?</private>").unwrap());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMatch {
    pub pattern_name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateBlockRange {
    pub start: usize,
    pub end: usize,
}

pub fn scan_for_secrets(input: &str) -> Vec<SecretMatch> {
    let mut matches = Vec::new();
    for (regex, name) in SECRET_PATTERNS.iter() {
        for m in regex.find_iter(input) {
            matches.push(SecretMatch {
                pattern_name: name.to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
    }
    matches.sort_by_key(|m| m.start);
    matches
}

pub fn contains_secrets(input: &str) -> bool {
    !scan_for_secrets(input).is_empty()
}

pub fn strip_private_blocks(input: &str) -> (String, Vec<PrivateBlockRange>) {
    let mut ranges = Vec::new();
    let mut cleaned = String::with_capacity(input.len());
    let mut last_end = 0;

    for m in PRIVATE_BLOCK_RE.find_iter(input) {
        ranges.push(PrivateBlockRange {
            start: m.start(),
            end: m.end(),
        });
        cleaned.push_str(&input[last_end..m.start()]);
        last_end = m.end();
    }
    cleaned.push_str(&input[last_end..]);

    (cleaned, ranges)
}

pub fn classify_sensitivity(input: &str, is_explicitly_private: bool) -> MemorySensitivity {
    if contains_secrets(input) {
        MemorySensitivity::Secret
    } else if is_explicitly_private {
        MemorySensitivity::Private
    } else {
        MemorySensitivity::Internal
    }
}

pub fn is_observation_safe(obs: &Observation) -> bool {
    if obs.sensitivity == MemorySensitivity::Secret {
        return false;
    }
    if contains_secrets(&obs.summary) {
        return false;
    }
    for (key, value) in obs.metadata.as_object().iter().flat_map(|m| m.iter()) {
        if let Some(v) = value.as_str() {
            if contains_secrets(key) || contains_secrets(v) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_api_key() {
        let input = "API_KEY=sk-abcdefghijklmnopqrstuvwxyz123456";
        let matches = scan_for_secrets(input);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_scan_jwt() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let matches = scan_for_secrets(input);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_scan_private_key() {
        let input = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC";
        let matches = scan_for_secrets(input);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_scan_connection_string() {
        let input = "DATABASE_URL=postgres://user:password@localhost:5432/db";
        let matches = scan_for_secrets(input);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_no_false_positive_on_normal_text() {
        let input = "The project uses PostgreSQL and pgvector for search.";
        let matches = scan_for_secrets(input);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_strip_private_blocks() {
        let input = "visible <private>hidden content</private> visible again";
        let (cleaned, ranges) = strip_private_blocks(input);
        assert_eq!(cleaned, "visible  visible again");
        assert_eq!(ranges.len(), 1);
    }

    #[test]
    fn test_strip_multiple_private_blocks() {
        let input = "a <private>x</private> b <private>y</private> c";
        let (cleaned, ranges) = strip_private_blocks(input);
        assert_eq!(cleaned, "a  b  c");
        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_no_private_blocks() {
        let input = "no blocks here";
        let (cleaned, ranges) = strip_private_blocks(input);
        assert_eq!(cleaned, "no blocks here");
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_classify_secret() {
        let sensitivity = classify_sensitivity("API_KEY=sk-abcdefghijklmnopqrstuvwxyz123456", false);
        assert_eq!(sensitivity, MemorySensitivity::Secret);
    }

    #[test]
    fn test_classify_private() {
        let sensitivity = classify_sensitivity("normal text", true);
        assert_eq!(sensitivity, MemorySensitivity::Private);
    }

    #[test]
    fn test_classify_internal() {
        let sensitivity = classify_sensitivity("normal text", false);
        assert_eq!(sensitivity, MemorySensitivity::Internal);
    }

    #[test]
    fn test_is_observation_safe_rejects_secret() {
        use crate::observation::{MemoryScope, Observation};
        let mut obs = Observation::new(
            MemoryScope::Project,
            "s1".into(),
            crate::observation::MemoryKind::Fact,
            "test".into(),
            crate::observation::MemoryConfidence::Low,
            MemorySensitivity::Internal,
        )
        .unwrap();
        // Manually set sensitivity to secret after constructor bypass
        obs.sensitivity = MemorySensitivity::Secret;
        assert!(!is_observation_safe(&obs));
    }
}
