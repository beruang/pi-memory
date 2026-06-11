use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub semantic: f64,
    pub keyword: f64,
    pub confidence: f64,
    pub recency: f64,
    pub evidence: f64,
    pub file_or_entity_match: f64,
}

impl Default for ScoreWeights {
    fn default() -> Self {
        Self {
            semantic: 0.45,
            keyword: 0.30,
            confidence: 0.10,
            recency: 0.05,
            evidence: 0.05,
            file_or_entity_match: 0.05,
        }
    }
}

impl ScoreWeights {
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.semantic
            + self.keyword
            + self.confidence
            + self.recency
            + self.evidence
            + self.file_or_entity_match;
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!("weights sum to {}, expected 1.0", sum));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ScoredObservation {
    pub observation_id: uuid::Uuid,
    pub kind: String,
    pub summary: String,
    pub confidence: String,
    pub status: String,
    pub vector_score: f64,
    pub text_score: f64,
    pub confidence_score: f64,
    pub recency_score: f64,
    pub final_score: f64,
}

pub fn confidence_to_score(confidence: &str) -> f64 {
    match confidence {
        "high" => 1.0,
        "medium" => 0.7,
        "low" => 0.4,
        _ => 0.5,
    }
}

pub fn recency_decay(age_days: f64) -> f64 {
    (1.0 / (1.0 + age_days / 30.0)).min(1.0)
}

pub fn compute_final_score(
    vector_score: f64,
    text_score: f64,
    confidence_str: &str,
    age_days: f64,
    evidence_count: usize,
    weights: &ScoreWeights,
) -> f64 {
    let confidence_score = confidence_to_score(confidence_str);
    let recency_score = recency_decay(age_days);
    let evidence_score = (evidence_count as f64).min(5.0) / 5.0;

    vector_score * weights.semantic
        + text_score * weights.keyword
        + confidence_score * weights.confidence
        + recency_score * weights.recency
        + evidence_score * weights.evidence
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_weights_sum_to_one() {
        let weights = ScoreWeights::default();
        assert!(weights.validate().is_ok());
    }

    #[test]
    fn test_confidence_to_score() {
        assert_eq!(confidence_to_score("high"), 1.0);
        assert_eq!(confidence_to_score("medium"), 0.7);
        assert_eq!(confidence_to_score("low"), 0.4);
        assert_eq!(confidence_to_score("unknown"), 0.5);
    }

    #[test]
    fn test_recency_decay() {
        assert_eq!(recency_decay(0.0), 1.0);
        assert!(recency_decay(30.0) < 0.6);
        assert!(recency_decay(365.0) < 0.1);
    }

    #[test]
    fn test_compute_final_score_range() {
        let weights = ScoreWeights::default();
        let score = compute_final_score(0.8, 0.6, "high", 10.0, 3, &weights);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_invalid_weights_rejected() {
        let mut weights = ScoreWeights::default();
        weights.semantic = 0.9;
        assert!(weights.validate().is_err());
    }
}
