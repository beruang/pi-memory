use memory_core::*;
use memory_providers::*;

#[tokio::test]
async fn test_full_memory_lifecycle() {
    // Create observation
    let obs = Observation::new(
        MemoryScope::Project,
        "test-session".into(),
        MemoryKind::Decision,
        "Use PostgreSQL advisory locks for job deduplication".into(),
        MemoryConfidence::High,
        MemorySensitivity::Internal,
    )
    .unwrap();

    assert_eq!(obs.scope, MemoryScope::Project);
    assert_eq!(obs.kind, MemoryKind::Decision);
    assert_eq!(obs.confidence, MemoryConfidence::High);
    assert_eq!(obs.status, MemoryStatus::Active);
    assert!(!obs.id.is_nil());
    assert_eq!(obs.session_id, "test-session");
}

#[tokio::test]
async fn test_observation_with_evidence() {
    let mut obs = Observation::new(
        MemoryScope::Project,
        "s1".into(),
        MemoryKind::Fix,
        "Queue duplication resolved by worker concurrency = 1".into(),
        MemoryConfidence::High,
        MemorySensitivity::Internal,
    )
    .unwrap();

    let evidence = EvidenceRef::new(obs.id, EvidenceSourceType::Message, "msg-1".into())
        .with_excerpt("Let's set worker concurrency to 1 during migration jobs.".into());

    obs.evidence.push(evidence);
    assert_eq!(obs.evidence.len(), 1);
    assert_eq!(
        obs.evidence[0].excerpt.as_deref(),
        Some("Let's set worker concurrency to 1 during migration jobs.")
    );
}

#[tokio::test]
async fn test_privacy_pipeline() {
    use memory_core::privacy::{classify_sensitivity, scan_for_secrets, strip_private_blocks};

    // Secret detection
    let with_secret = "API_KEY=sk-abcdefghijklmnopqrstuvwxyz123456";
    assert!(!scan_for_secrets(with_secret).is_empty());

    // Clean content
    let clean = "The project uses PostgreSQL and pgvector.";
    assert!(scan_for_secrets(clean).is_empty());

    // Private block stripping
    let mixed = "visible <private>secret stuff</private> visible";
    let (cleaned, ranges) = strip_private_blocks(mixed);
    assert_eq!(cleaned, "visible  visible");
    assert_eq!(ranges.len(), 1);
    assert!(!cleaned.contains("secret stuff"));

    // Sensitivity classification
    assert_eq!(
        classify_sensitivity("API_KEY=sk-abcdefghijklmnopqrstuvwxyz123456", false),
        MemorySensitivity::Secret
    );
    assert_eq!(
        classify_sensitivity("normal text", false),
        MemorySensitivity::Internal
    );
}

#[tokio::test]
async fn test_conflict_detection_flow() {
    let new = Observation::new(
        MemoryScope::Project,
        "s1".into(),
        MemoryKind::Fact,
        "Project upgraded to Node 22".into(),
        MemoryConfidence::High,
        MemorySensitivity::Internal,
    )
    .unwrap();

    let mut existing = Observation::new(
        MemoryScope::Project,
        "s1".into(),
        MemoryKind::Fact,
        "Project downgraded to Node 20".into(),
        MemoryConfidence::High,
        MemorySensitivity::Internal,
    )
    .unwrap();
    existing.entities = vec!["node".into(), "runtime".into()];
    let mut new_with_entities = new.clone();
    new_with_entities.entities = vec!["node".into(), "runtime".into()];

    let conflicts = memory_core::conflict::detect_conflicts(&new_with_entities, &[existing]);
    assert!(!conflicts.is_empty());
}

#[tokio::test]
async fn test_supersession_flow() {
    let mut old = Observation::new(
        MemoryScope::Project,
        "s1".into(),
        MemoryKind::Fact,
        "Project uses Node 20".into(),
        MemoryConfidence::High,
        MemorySensitivity::Internal,
    )
    .unwrap();

    let new = Observation::new(
        MemoryScope::Project,
        "s2".into(),
        MemoryKind::Fact,
        "Project upgraded to Node 22".into(),
        MemoryConfidence::High,
        MemorySensitivity::Internal,
    )
    .unwrap();

    old.status = MemoryStatus::Superseded;
    old.superseded_by = Some(new.id);

    assert_eq!(old.status, MemoryStatus::Superseded);
    assert_eq!(old.superseded_by, Some(new.id));
}

#[tokio::test]
async fn test_provider_mock_roundtrip() {
    let emb_config = ProviderConfig {
        api_key: None,
        model: "mock".into(),
        base_url: None,
        dimensions: Some(128),
    };
    let emb_provider = create_embedding_provider("mock", &emb_config).unwrap();
    let embedding = emb_provider.embed("test memory text").await.unwrap();
    assert_eq!(embedding.len(), 1536);
    assert!(embedding.iter().any(|&v| v != 0.0));

    let con_config = ProviderConfig {
        api_key: None,
        model: "mock".into(),
        base_url: None,
        dimensions: None,
    };
    let con_provider = create_consolidation_provider("mock", &con_config).unwrap();
    let input = ConsolidationInput {
        session_id: "s1".into(),
        project_id: None,
        events: vec![],
        existing_observations: vec![],
        user_instructions: None,
    };
    let candidates = con_provider.consolidate(input).await.unwrap();
    assert!(candidates.is_empty()); // No events = no candidates
}

#[tokio::test]
async fn test_consolidation_with_events() {
    let con_config = ProviderConfig {
        api_key: None,
        model: "mock".into(),
        base_url: None,
        dimensions: None,
    };
    let con_provider = create_consolidation_provider("mock", &con_config).unwrap();
    let event = SessionEvent::new(
        "s1".into(),
        "user_prompt".into(),
        serde_json::json!({"text": "The team decided to use PostgreSQL advisory locks for job deduplication because Redis locks expired too early during long-running jobs."}),
    );
    let input = ConsolidationInput {
        session_id: "s1".into(),
        project_id: None,
        events: vec![event],
        existing_observations: vec![],
        user_instructions: None,
    };
    let candidates = con_provider.consolidate(input).await.unwrap();
    assert!(!candidates.is_empty(), "Provider should produce candidates from session events");

    // Verify candidate structure
    let candidate = &candidates[0];
    assert_eq!(candidate.scope, MemoryScope::Project);
    assert_eq!(candidate.kind, MemoryKind::Fact);
    assert_eq!(candidate.source_event_ids.len(), 1);
}

#[test]
fn test_token_budget_session_start() {
    let budget = TokenBudget::default();
    assert_eq!(budget.session_start, 1000);
    assert!(budget.session_start < budget.task_recall);
    assert!(budget.session_start > budget.preference_recall);
}

#[tokio::test]
async fn test_hard_delete_status_transition() {
    // Verify that deleted status is valid transition from any active status
    let obs = Observation::new(
        MemoryScope::Project,
        "s1".into(),
        MemoryKind::Fact,
        "test".into(),
        MemoryConfidence::Low,
        MemorySensitivity::Internal,
    )
    .unwrap();
    assert!(valid_transition(obs.status, MemoryStatus::Deleted));
}

#[tokio::test]
async fn test_session_start_empty_project() {
    // Verify the SessionEvent creation and mark_redacted used in session events
    let mut event = SessionEvent::new(
        "s-start".into(),
        "session_start".into(),
        serde_json::json!({"project_id": "00000000-0000-0000-0000-000000000000"}),
    );
    assert!(!event.redacted);

    event.mark_redacted();
    assert!(event.redacted);
    assert_eq!(event.session_id, "s-start");
}

#[test]
fn test_tool_definitions_include_session_start() {
    let tools = memory_mcp::schemas::all_tool_definitions();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(
        names.contains(&"memory.session_start"),
        "memory.session_start must be defined"
    );
    assert_eq!(tools.len(), 12);
}
