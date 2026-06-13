use std::sync::Arc;

use memory_config::AppConfig;
use memory_core::MemoryError;
use memory_daemon::queue::JobQueue;
use memory_daemon::scheduler::Scheduler;
use memory_daemon::worker::{
    CleanupEventsWorker, ConsolidationWorker, DetectConflictsWorker, EmbeddingWorker,
};
use memory_db::{
    migrate, AuditRepository, ConflictsRepository, EmbeddingsRepository, EventsRepository,
    EvidenceRepository, ObservationsRepository, SearchParams, SearchRepository,
    SupersessionsRepository,
};
use memory_mcp::MemoryMcpServer;
use memory_providers::{create_consolidation_provider, create_embedding_provider, ProviderConfig};
use sqlx::PgPool;
use std::path::Path;
use tracing::{error, info};

async fn load_config(config_path: Option<&Path>) -> AppConfig {
    AppConfig::load(config_path).unwrap_or_else(|e| {
        eprintln!("Warning: failed to load config: {}", e);
        AppConfig::default()
    })
}

async fn create_pool(database_url: &str) -> Result<PgPool, MemoryError> {
    PgPool::connect(database_url)
        .await
        .map_err(MemoryError::Database)
}

pub async fn serve(port: u16, config_path: Option<String>) {
    let config = load_config(config_path.as_deref().map(Path::new)).await;
    info!("Starting agent-memory daemon on port {}...", port);
    println!("agent-memory daemon starting on port {}...", port);

    let pool = match create_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            println!("ERROR: Failed to connect to database: {}", e);
            println!("Make sure PostgreSQL is running and DATABASE_URL is correct.");
            return;
        }
    };

    // Run migrations
    if let Err(e) = migrate(&pool).await {
        error!("Failed to run migrations: {}", e);
        println!("ERROR: Failed to run migrations: {}", e);
        return;
    }

    // Create provider config
    let provider_config = ProviderConfig {
        api_key: config
            .embedding
            .api_key_env
            .as_ref()
            .and_then(|k| std::env::var(k.as_str()).ok()),
        model: config.embedding.model.clone(),
        base_url: config.embedding.base_url.clone(),
        dimensions: Some(config.embedding.dimensions),
    };

    println!("Database connected. Starting daemon workers...");

    // Start job queue and workers
    let queue = Arc::new(JobQueue::new());

    let mut handles = Vec::new();

    // Consolidation worker
    let cons_queue = queue.clone();
    let cons_pool = pool.clone();
    let cons_provider_type = config.embedding.kind.clone();
    let cons_provider_cfg = provider_config.clone();
    handles.push(tokio::spawn(async move {
        let worker =
            ConsolidationWorker::new(cons_queue, cons_pool, cons_provider_type, cons_provider_cfg);
        worker.run().await;
    }));

    // Embedding worker
    let emb_queue = queue.clone();
    let emb_pool = pool.clone();
    handles.push(tokio::spawn(async move {
        let worker = EmbeddingWorker::new(emb_queue, emb_pool);
        worker.run().await;
    }));

    // Cleanup events worker
    let cleanup_queue = queue.clone();
    let cleanup_pool = pool.clone();
    let retention_days = config.event_retention_days;
    handles.push(tokio::spawn(async move {
        let worker = CleanupEventsWorker::new(cleanup_queue, cleanup_pool, retention_days);
        worker.run().await;
    }));

    // Conflict detection worker
    let detect_queue = queue.clone();
    let detect_pool = pool.clone();
    handles.push(tokio::spawn(async move {
        let worker = DetectConflictsWorker::new(detect_queue, detect_pool);
        worker.run().await;
    }));

    // Scheduler (periodically enqueues cleanup + conflict detection)
    let scheduler_queue = queue.clone();
    handles.push(tokio::spawn(async move {
        let scheduler = Scheduler::new(scheduler_queue);
        scheduler.start().await;
    }));

    // Start HTTP API server (--port overrides config file)
    let api_host = config.api.host.clone();
    let api_port = port;
    let app = memory_api::routes::create_router(memory_api::handlers::AppState::new(pool.clone()));
    let addr = format!("{}:{}", api_host, api_port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("API server listening on {}", addr);
    println!("API server listening on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

pub async fn mcp() {
    let config = load_config(None).await;
    info!("Starting MCP server over stdio...");
    println!("MCP server starting on stdio...");

    let pool = match create_pool(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            println!("ERROR: Failed to connect to database: {}", e);
            println!("Make sure PostgreSQL is running and DATABASE_URL is correct.");
            return;
        }
    };

    // Run migrations
    if let Err(e) = migrate(&pool).await {
        error!("Failed to run migrations: {}", e);
        println!("ERROR: Failed to run migrations: {}", e);
        return;
    }

    // Create repositories
    let obs_repo = ObservationsRepository::new(pool.clone());
    let evidence_repo = EvidenceRepository::new(pool.clone());
    let embeddings_repo = EmbeddingsRepository::new(pool.clone());
    let search_repo = SearchRepository::new(pool.clone());
    let conflicts_repo = ConflictsRepository::new(pool.clone());
    let audit_repo = AuditRepository::new(pool.clone());
    let supers_repo = SupersessionsRepository::new(pool.clone());
    let events_repo = EventsRepository::new(pool.clone());

    let provider_config = ProviderConfig {
        api_key: config
            .embedding
            .api_key_env
            .as_ref()
            .and_then(|k| std::env::var(k.as_str()).ok()),
        model: config.embedding.model.clone(),
        base_url: config.embedding.base_url.clone(),
        dimensions: Some(config.embedding.dimensions),
    };
    let provider_kind = &config.embedding.kind;
    let consolidation_provider = create_consolidation_provider(provider_kind, &provider_config)
        .unwrap_or_else(|e| {
            error!("Failed to create consolidation provider ({}), using mock: {}", provider_kind, e);
            create_consolidation_provider("mock", &ProviderConfig {
                api_key: None,
                model: "mock".into(),
                base_url: None,
                dimensions: None,
            }).expect("mock provider always works")
        });

    // MCP server uses agent context with optional project restriction from config
    let access_ctx = memory_core::AccessContext::agent(config.project_id);

    let server = MemoryMcpServer::new(
        obs_repo,
        evidence_repo,
        embeddings_repo,
        search_repo,
        conflicts_repo,
        audit_repo,
        supers_repo,
        events_repo,
        consolidation_provider,
        access_ctx,
    );

    println!("MCP server ready. Running over stdio...");
    let _ = memory_mcp::transport::run_stdio_server(server).await;
}

pub async fn migrate_cmd(database_url: Option<String>) {
    let url = database_url.unwrap_or_else(|| {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost:5432/agent_memory".into())
    });
    info!("Running migrations against {}", url);
    println!("Running database migrations...");
    println!("Database URL: {}", url);

    let pool = match PgPool::connect(&url).await {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to connect: {}", e);
            return;
        }
    };

    match memory_db::migrate(&pool).await {
        Ok(_) => {
            println!("Migrations completed successfully.");
        }
        Err(e) => {
            println!("ERROR: Migration failed: {}", e);
        }
    }
}

pub async fn search(query: String, scope: Option<String>, project_id: Option<String>, limit: i64) {
    let scope = scope.unwrap_or_else(|| "project".into());
    info!(
        "Searching: query='{}' scope='{}' limit={}",
        query, scope, limit
    );

    let pid = project_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let pool = match create_pool("postgres://localhost:5432/agent_memory").await {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to connect to database: {}", e);
            return;
        }
    };

    let provider = match create_embedding_provider(
        "mock",
        &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(1536),
        },
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to create embedding provider: {}", e);
            return;
        }
    };
    let query_embedding = match provider.embed(&query).await {
        Ok(e) => e,
        Err(err) => {
            println!("ERROR: Failed to generate embedding: {}", err);
            return;
        }
    };

    let search_repo = SearchRepository::new(pool);
    let results = match search_repo
        .hybrid_search(SearchParams {
            query_embedding,
            text_query: query.clone(),
            scope,
            project_id: pid,
            kinds: None,
            files: None,
            entities: None,
            limit,
            min_confidence: None,
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("ERROR: Search failed: {}", e);
            return;
        }
    };

    println!("Search results for '{}':", query);
    for (i, r) in results.iter().enumerate() {
        println!(
            "  {}. [{}] {} (score: {:.3}, conf: {})",
            i + 1,
            r.observation.kind,
            r.observation.summary,
            r.final_score,
            r.observation.confidence,
        );
    }
    println!("Total: {} results", results.len());
}

pub async fn recall(task: String, scope: Option<String>, project_id: Option<String>) {
    let scope = scope.unwrap_or_else(|| "project".into());
    let token_budget: usize = 1200;
    info!("Recalling for task: '{}' scope='{}'", task, scope);

    let pid = project_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let pool = match create_pool("postgres://localhost:5432/agent_memory").await {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to connect to database: {}", e);
            return;
        }
    };

    let provider = match create_embedding_provider(
        "mock",
        &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(1536),
        },
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to create embedding provider: {}", e);
            return;
        }
    };
    let query_embedding = match provider.embed(&task).await {
        Ok(e) => e,
        Err(err) => {
            println!("ERROR: Failed to generate embedding: {}", err);
            return;
        }
    };

    let search_repo = SearchRepository::new(pool);
    let results = match search_repo
        .hybrid_search(SearchParams {
            query_embedding,
            text_query: task.clone(),
            scope,
            project_id: pid,
            kinds: None,
            files: None,
            entities: None,
            limit: 10,
            min_confidence: None,
        })
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("ERROR: Recall failed: {}", e);
            return;
        }
    };

    let chars_per_token = 4;
    let mut token_estimate = 0;
    println!("Recall results for task '{}':", task);
    for (i, r) in results.iter().enumerate() {
        let item_tokens = r.observation.summary.len() / chars_per_token;
        if token_estimate + item_tokens > token_budget {
            println!("  ... (token budget of {} exceeded)", token_budget);
            break;
        }
        token_estimate += item_tokens;
        println!(
            "  {}. [{}] {} (score: {:.3}, conf: {})",
            i + 1,
            r.observation.kind,
            r.observation.summary,
            r.final_score,
            r.observation.confidence,
        );
    }
    println!("Total: {} results ({} tokens)", results.len(), token_estimate);
}

pub async fn consolidate(session: String, project_id: Option<String>) {
    info!("Consolidating session: {}", session);
    println!("Consolidating session '{}'...", session);

    let pid = project_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let pool = match create_pool("postgres://localhost:5432/agent_memory").await {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to connect to database: {}", e);
            return;
        }
    };

    let events_repo = EventsRepository::new(pool.clone());
    let events = match events_repo.list_by_session(&session, 200).await {
        Ok(e) => e,
        Err(err) => {
            println!("ERROR: Failed to load events: {}", err);
            return;
        }
    };
    println!("  Loaded {} events", events.len());

    let obs_repo = ObservationsRepository::new(pool.clone());
    let existing = match obs_repo.list_active_with_entities(pid).await {
        Ok(o) => o,
        Err(err) => {
            println!("ERROR: Failed to load existing observations: {}", err);
            return;
        }
    };
    println!("  Loaded {} existing observations", existing.len());

    let provider = match create_consolidation_provider(
        "mock",
        &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: None,
        },
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to create consolidation provider: {}", e);
            return;
        }
    };

    let input = memory_core::ConsolidationInput {
        session_id: session.clone(),
        project_id: pid,
        events,
        existing_observations: existing,
        user_instructions: None,
    };

    let candidates = match provider.consolidate(input).await {
        Ok(c) => c,
        Err(e) => {
            println!("ERROR: Consolidation failed: {}", e);
            return;
        }
    };
    println!("  Generated {} candidate observations", candidates.len());

    let embedding_provider = match create_embedding_provider(
        "mock",
        &ProviderConfig {
            api_key: None,
            model: "mock".into(),
            base_url: None,
            dimensions: Some(1536),
        },
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to create embedding provider: {}", e);
            return;
        }
    };

    let evidence_repo = EvidenceRepository::new(pool.clone());
    let embeddings_repo = EmbeddingsRepository::new(pool.clone());
    let conflicts_repo = ConflictsRepository::new(pool.clone());
    let audit_repo = AuditRepository::new(pool.clone());

    let mut written_count = 0;
    let mut conflict_count = 0;

    for candidate in &candidates {
        let mut obs = match memory_core::Observation::new(
            candidate.scope,
            session.clone(),
            candidate.kind,
            candidate.summary.clone(),
            candidate.confidence,
            candidate.sensitivity,
        ) {
            Ok(o) => o,
            Err(e) => {
                println!("  WARN: Skipping candidate: {}", e);
                continue;
            }
        };
        obs.project_id = pid;
        obs.entities = candidate.entities.clone();
        obs.files = candidate.files.clone();
        obs.commands = candidate.commands.clone();

        let conflicts = memory_core::detect_conflicts(&obs, &[]);
        obs.status = if conflicts.is_empty() {
            memory_core::MemoryStatus::Active
        } else {
            memory_core::MemoryStatus::Conflicted
        };

        if let Ok(saved) = obs_repo
            .insert_with_links(&obs, &candidate.files, &candidate.entities, &candidate.commands, &[])
            .await
        {
            // Insert evidence from source events
            for event_id in &candidate.source_event_ids {
                let mut ev = memory_core::EvidenceRef::new(
                    obs.id,
                    memory_core::EvidenceSourceType::Message,
                    event_id.to_string(),
                );
                if let Some(ref rationale) = candidate.rationale {
                    ev = ev.with_excerpt(rationale.clone());
                }
                let _ = evidence_repo.insert(&ev).await;
            }

            // Generate and store embedding
            if let Ok(embedding) = embedding_provider.embed(&obs.summary).await {
                let _ = embeddings_repo
                    .upsert_embedding(obs.id, "mock", 1536, &embedding)
                    .await;
            }

            for conflict in &conflicts {
                let _ = conflicts_repo
                    .insert_conflict(
                        conflict.left_observation_id,
                        conflict.right_observation_id,
                        &conflict.conflict_type.to_string(),
                        &conflict.description,
                    )
                    .await;
            }
            conflict_count += conflicts.len();
            written_count += 1;

            println!(
                "  Wrote observation: [{}] {} (status: {})",
                saved.kind, saved.summary, saved.status
            );
        }
    }

    let _ = audit_repo
        .insert(
            "cli",
            None,
            "consolidate",
            None,
            None,
            Some(&serde_json::json!({
                "session_id": session,
                "candidates_generated": candidates.len(),
                "observations_written": written_count,
                "conflicts_detected": conflict_count,
            })),
        )
        .await;

    println!(
        "Consolidation complete: {} observations written, {} conflicts detected",
        written_count, conflict_count
    );
}

pub async fn inspect(observation_id: String) {
    info!("Inspecting observation: {}", observation_id);

    let pool = match create_pool("postgres://localhost:5432/agent_memory").await {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: Failed to connect to database: {}", e);
            return;
        }
    };

    let id = match uuid::Uuid::parse_str(&observation_id) {
        Ok(id) => id,
        Err(e) => {
            println!("ERROR: Invalid observation ID: {}", e);
            return;
        }
    };

    let obs_repo = ObservationsRepository::new(pool.clone());
    let evidence_repo = EvidenceRepository::new(pool.clone());

    let obs = match obs_repo.get_by_id_with_links(id).await {
        Ok(Some(o)) => o,
        Ok(None) => {
            println!("Observation not found: {}", observation_id);
            return;
        }
        Err(e) => {
            println!("ERROR: Failed to load observation: {}", e);
            return;
        }
    };

    let evidence = match evidence_repo.get_by_observation_id(id).await {
        Ok(e) => e,
        Err(err) => {
            println!("  (Failed to load evidence: {})", err);
            vec![]
        }
    };

    println!("Observation: {}", obs.id);
    println!("  Kind:       {}", obs.kind);
    println!("  Summary:    {}", obs.summary);
    println!("  Scope:      {}", obs.scope);
    println!("  Confidence: {}", obs.confidence);
    println!("  Sensitivity: {}", obs.sensitivity);
    println!("  Status:     {}", obs.status);
    println!("  Session:    {}", obs.session_id);
    if let Some(pid) = obs.project_id {
        println!("  Project:    {}", pid);
    }
    println!("  Created:    {}", obs.created_at);
    println!("  Updated:    {}", obs.updated_at);
    if !obs.entities.is_empty() {
        println!("  Entities:   {:?}", obs.entities);
    }
    if !obs.files.is_empty() {
        println!("  Files:      {:?}", obs.files);
    }
    println!("  Evidence:");
    for ev in &evidence {
        println!(
            "    - [{}] {}: {}",
            ev.source_type, ev.source_id, ev.excerpt.as_deref().unwrap_or("")
        );
    }
}

pub async fn ui(port: u16, config_path: Option<String>) {
    // The review UI is served at the root of the API server (/)
    // Start the full daemon so all API endpoints are available
    info!("Starting review UI daemon on port {}...", port);
    println!("Memory Review UI starting at http://localhost:{}", port);
    println!("(Full API is also available at http://localhost:{}/health)", port);
    serve(port, config_path).await;
}
