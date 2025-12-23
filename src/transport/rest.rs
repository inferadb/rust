//! REST transport implementation using reqwest.
//!
//! This module provides HTTP/REST transport for the InferaDB SDK,
//! handling both standard JSON responses and Server-Sent Events (SSE)
//! for streaming endpoints.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::{Stream, StreamExt};
use parking_lot::RwLock;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

use crate::config::{RetryConfig, TlsConfig};
use crate::error::ErrorKind;
use crate::transport::traits::{
    CheckRequest, CheckResponse, ListRelationshipsResponse, ListResourcesResponse,
    ListSubjectsResponse, PoolConfig, RestStats, SimulateRequest, SimulateResponse, Transport,
    TransportClient, TransportStats, WriteRequest, WriteResponse,
};
use crate::types::{ConsistencyToken, Decision, Relationship};
use crate::user_agent;
use crate::Error;

// ============================================================================
// REST Transport
// ============================================================================

/// REST transport using reqwest.
///
/// Handles HTTP/REST communication with the InferaDB Engine API,
/// including SSE streaming for list operations.
#[derive(Clone)]
pub struct RestTransport {
    client: reqwest::Client,
    base_url: Url,
    auth_token: Arc<RwLock<Option<String>>>,
    retry_config: RetryConfig,
    stats: Arc<RwLock<RestStats>>,
}

impl std::fmt::Debug for RestTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestTransport")
            .field("base_url", &self.base_url.as_str())
            .finish_non_exhaustive()
    }
}

impl RestTransport {
    /// Creates a new REST transport builder.
    pub fn builder() -> RestTransportBuilder {
        RestTransportBuilder::new()
    }

    /// Creates a new REST transport with the given configuration.
    pub fn new(
        base_url: Url,
        tls_config: &TlsConfig,
        pool_config: &PoolConfig,
        retry_config: RetryConfig,
        timeout: Duration,
    ) -> Result<Self, Error> {
        let mut client_builder = reqwest::Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(pool_config.max_idle_per_host as usize)
            .pool_idle_timeout(pool_config.idle_timeout)
            .user_agent(user_agent::user_agent());

        // Configure TLS with skip_verification for local development
        if tls_config.skip_verification {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        // Add custom CA certificate if provided
        if let Some(ref ca_cert_file) = tls_config.ca_cert_file {
            let cert_pem = std::fs::read(ca_cert_file).map_err(|e| {
                Error::new(
                    ErrorKind::Configuration,
                    format!("Failed to read certificate {:?}: {}", ca_cert_file, e),
                )
            })?;
            let cert = reqwest::Certificate::from_pem(&cert_pem).map_err(|e| {
                Error::new(
                    ErrorKind::Configuration,
                    format!("Invalid certificate {:?}: {}", ca_cert_file, e),
                )
            })?;
            client_builder = client_builder.add_root_certificate(cert);
        }

        // Add CA certificate from PEM data if provided
        if let Some(ref ca_cert_pem) = tls_config.ca_cert_pem {
            let cert = reqwest::Certificate::from_pem(ca_cert_pem.as_bytes()).map_err(|e| {
                Error::new(
                    ErrorKind::Configuration,
                    format!("Invalid CA certificate PEM: {}", e),
                )
            })?;
            client_builder = client_builder.add_root_certificate(cert);
        }

        let client = client_builder.build().map_err(|e| {
            Error::new(
                ErrorKind::Configuration,
                format!("Failed to create HTTP client: {}", e),
            )
        })?;

        Ok(Self {
            client,
            base_url,
            auth_token: Arc::new(RwLock::new(None)),
            retry_config,
            stats: Arc::new(RwLock::new(RestStats::default())),
        })
    }

    /// Sets the authentication token.
    pub fn set_auth_token(&self, token: String) {
        *self.auth_token.write() = Some(token);
    }

    /// Clears the authentication token.
    pub fn clear_auth_token(&self) {
        *self.auth_token.write() = None;
    }

    /// Builds default headers for requests.
    fn build_headers(&self) -> Result<HeaderMap, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        // Note: User-Agent is set at the client level via .user_agent() in new()

        if let Some(ref token) = *self.auth_token.read() {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_value).map_err(|_| {
                    Error::new(ErrorKind::Unauthorized, "Invalid auth token format")
                })?,
            );
        }

        Ok(headers)
    }

    /// Makes a POST request with JSON body.
    async fn post<T, R>(&self, path: &str, body: &T) -> Result<R, Error>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = self.base_url.join(path).map_err(|e| {
            Error::new(ErrorKind::Configuration, format!("Invalid URL path: {}", e))
        })?;

        let headers = self.build_headers()?;

        let response = self
            .execute_with_retry(|| async {
                self.client
                    .post(url.clone())
                    .headers(headers.clone())
                    .json(body)
                    .send()
                    .await
            })
            .await?;

        self.handle_response(response).await
    }

    /// Makes a DELETE request.
    async fn delete_request(&self, path: &str) -> Result<(), Error> {
        let url = self.base_url.join(path).map_err(|e| {
            Error::new(ErrorKind::Configuration, format!("Invalid URL path: {}", e))
        })?;

        let headers = self.build_headers()?;

        let response = self
            .execute_with_retry(|| async {
                self.client
                    .delete(url.clone())
                    .headers(headers.clone())
                    .send()
                    .await
            })
            .await?;

        self.handle_error_response(response).await
    }

    /// Makes a POST request that returns SSE stream.
    async fn post_sse<T, R>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<R, Error>> + Send>>, Error>
    where
        T: Serialize,
        R: DeserializeOwned + Send + 'static,
    {
        let url = self.base_url.join(path).map_err(|e| {
            Error::new(ErrorKind::Configuration, format!("Invalid URL path: {}", e))
        })?;

        let mut headers = self.build_headers()?;
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(body)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.requests_sent += 1;
            stats.sse_connections += 1;
            stats.sse_active += 1;
        }

        let status = response.status();
        if !status.is_success() {
            {
                let mut stats = self.stats.write();
                stats.sse_active = stats.sse_active.saturating_sub(1);
                stats.requests_failed += 1;
            } // Guard dropped here before await

            let error_text = response.text().await.unwrap_or_default();
            return Err(map_status_error(status.as_u16(), &error_text));
        }

        let stats = Arc::clone(&self.stats);
        let byte_stream = response.bytes_stream();

        // Parse SSE stream and box it for Unpin
        let sse_stream = parse_sse_stream(byte_stream, stats);

        Ok(Box::pin(sse_stream))
    }

    /// Executes a request with retry logic.
    async fn execute_with_retry<F, Fut>(&self, make_request: F) -> Result<reqwest::Response, Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let mut attempt = 0;
        let max_attempts = self.retry_config.max_retries + 1; // +1 for initial attempt
        let mut delay = self.retry_config.initial_delay;

        loop {
            attempt += 1;

            match make_request().await {
                Ok(response) => {
                    let status = response.status();

                    // Handle rate limiting
                    if status.as_u16() == 429 {
                        if attempt >= max_attempts {
                            let mut stats = self.stats.write();
                            stats.requests_sent += 1;
                            stats.requests_failed += 1;
                            drop(stats);
                            return Err(Error::new(
                                ErrorKind::RateLimited,
                                "Rate limited after max retries",
                            ));
                        }

                        // Get Retry-After header if present
                        let retry_after = response
                            .headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(Duration::from_secs)
                            .unwrap_or(delay);

                        tokio::time::sleep(retry_after).await;
                        delay = std::cmp::min(delay * 2, self.retry_config.max_delay);
                        continue;
                    }

                    // Don't retry client errors (except 429)
                    if status.is_client_error() || status.is_success() || status.is_redirection() {
                        let mut stats = self.stats.write();
                        stats.requests_sent += 1;
                        if !status.is_success() {
                            stats.requests_failed += 1;
                        }
                        return Ok(response);
                    }

                    // Retry server errors
                    if attempt >= max_attempts {
                        let mut stats = self.stats.write();
                        stats.requests_sent += 1;
                        stats.requests_failed += 1;
                        return Ok(response);
                    }

                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, self.retry_config.max_delay);
                }
                Err(e) => {
                    if attempt >= max_attempts {
                        let mut stats = self.stats.write();
                        stats.requests_sent += 1;
                        stats.requests_failed += 1;
                        return Err(map_reqwest_error(e));
                    }

                    // Only retry on connection/timeout errors
                    if e.is_connect() || e.is_timeout() {
                        tokio::time::sleep(delay).await;
                        delay = std::cmp::min(delay * 2, self.retry_config.max_delay);
                        continue;
                    }

                    let mut stats = self.stats.write();
                    stats.requests_sent += 1;
                    stats.requests_failed += 1;
                    return Err(map_reqwest_error(e));
                }
            }
        }
    }

    /// Handles a response and parses JSON body.
    async fn handle_response<R>(&self, response: reqwest::Response) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(map_status_error(status.as_u16(), &error_text));
        }

        response.json::<R>().await.map_err(|e| {
            Error::new(
                ErrorKind::InvalidResponse,
                format!("Failed to parse response: {}", e),
            )
        })
    }

    /// Handles error response (for DELETE which returns no body).
    async fn handle_error_response(&self, response: reqwest::Response) -> Result<(), Error> {
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(map_status_error(status.as_u16(), &error_text));
        }

        Ok(())
    }
}

// ============================================================================
// REST Transport Builder
// ============================================================================

/// Builder for REST transport.
pub struct RestTransportBuilder {
    base_url: Option<Url>,
    tls_config: TlsConfig,
    pool_config: PoolConfig,
    retry_config: RetryConfig,
    timeout: Duration,
}

impl RestTransportBuilder {
    fn new() -> Self {
        Self {
            base_url: None,
            tls_config: TlsConfig::default(),
            pool_config: PoolConfig::default(),
            retry_config: RetryConfig::default(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Sets the base URL.
    pub fn base_url(mut self, url: impl AsRef<str>) -> Result<Self, Error> {
        self.base_url = Some(Url::parse(url.as_ref()).map_err(|e| {
            Error::new(ErrorKind::Configuration, format!("Invalid base URL: {}", e))
        })?);
        Ok(self)
    }

    /// Sets the TLS configuration.
    pub fn tls_config(mut self, config: TlsConfig) -> Self {
        self.tls_config = config;
        self
    }

    /// Sets the connection pool configuration.
    pub fn pool_config(mut self, config: PoolConfig) -> Self {
        self.pool_config = config;
        self
    }

    /// Sets the retry configuration.
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builds the REST transport.
    pub fn build(self) -> Result<RestTransport, Error> {
        let base_url = self
            .base_url
            .ok_or_else(|| Error::new(ErrorKind::Configuration, "Base URL is required"))?;

        RestTransport::new(
            base_url,
            &self.tls_config,
            &self.pool_config,
            self.retry_config,
            self.timeout,
        )
    }
}

// ============================================================================
// API Request/Response Types
// ============================================================================

#[derive(Debug, Serialize)]
struct EvaluateRequest {
    evaluations: Vec<EvaluateItem>,
}

#[derive(Debug, Serialize)]
struct EvaluateItem {
    subject: String,
    resource: String,
    permission: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct EvaluateResponse {
    decision: String,
    index: usize,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    trace: Option<TraceResponse>,
}

/// Trace response from the engine API.
#[derive(Debug, Deserialize)]
struct TraceResponse {
    #[serde(default)]
    duration_micros: u64,
    #[serde(default)]
    relationships_read: u64,
    #[serde(default)]
    relations_evaluated: u64,
    #[serde(default)]
    root: Option<EvaluationNodeResponse>,
}

/// Evaluation node from the engine API.
#[derive(Debug, Deserialize)]
struct EvaluationNodeResponse {
    #[serde(default)]
    node_type: Option<NodeTypeResponse>,
    #[serde(default)]
    result: bool,
    #[serde(default)]
    children: Vec<EvaluationNodeResponse>,
}

/// Node type from the engine API.
#[derive(Debug, Deserialize)]
struct NodeTypeResponse {
    #[serde(default)]
    direct_check: Option<DirectCheckResponse>,
    #[serde(default)]
    computed_userset: Option<ComputedUsersetResponse>,
    #[serde(default)]
    related_object_userset: Option<RelatedObjectUsersetResponse>,
    #[serde(default)]
    union: Option<serde_json::Value>,
    #[serde(default)]
    intersection: Option<serde_json::Value>,
    #[serde(default)]
    exclusion: Option<serde_json::Value>,
    #[serde(default)]
    wasm_module: Option<WasmModuleResponse>,
}

#[derive(Debug, Deserialize)]
struct DirectCheckResponse {
    resource: String,
    relation: String,
    subject: String,
}

#[derive(Debug, Deserialize)]
struct ComputedUsersetResponse {
    relation: String,
}

#[derive(Debug, Deserialize)]
struct RelatedObjectUsersetResponse {
    relationship: String,
    computed: String,
}

#[derive(Debug, Deserialize)]
struct WasmModuleResponse {
    module_name: String,
}

#[derive(Debug, Serialize)]
struct WriteRelationshipsRequest {
    relationships: Vec<RelationshipDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_revision: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RelationshipDto {
    resource: String,
    relation: String,
    subject: String,
}

#[derive(Debug, Deserialize)]
struct WriteRelationshipsResponse {
    revision: String,
    #[allow(dead_code)]
    relationships_written: usize,
}

#[derive(Debug, Serialize)]
struct ListRelationshipsApiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListResourcesApiRequest {
    subject: String,
    resource_type: String,
    permission: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_id_pattern: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListSubjectsApiRequest {
    resource: String,
    relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
}

// ============================================================================
// Trace Conversion Helpers
// ============================================================================

fn convert_trace_response(trace: TraceResponse) -> super::traits::DecisionTrace {
    super::traits::DecisionTrace {
        duration_micros: trace.duration_micros,
        relationships_read: trace.relationships_read,
        relations_evaluated: trace.relations_evaluated,
        root: trace.root.map(convert_evaluation_node),
    }
}

fn convert_evaluation_node(node: EvaluationNodeResponse) -> super::traits::EvaluationNode {
    let node_type = if let Some(nt) = node.node_type {
        if let Some(dc) = nt.direct_check {
            super::traits::EvaluationNodeType::DirectCheck {
                resource: dc.resource,
                relation: dc.relation,
                subject: dc.subject,
            }
        } else if let Some(cu) = nt.computed_userset {
            super::traits::EvaluationNodeType::ComputedUserset {
                relation: cu.relation,
            }
        } else if let Some(rou) = nt.related_object_userset {
            super::traits::EvaluationNodeType::RelatedObjectUserset {
                relationship: rou.relationship,
                computed: rou.computed,
            }
        } else if nt.union.is_some() {
            super::traits::EvaluationNodeType::Union
        } else if nt.intersection.is_some() {
            super::traits::EvaluationNodeType::Intersection
        } else if nt.exclusion.is_some() {
            super::traits::EvaluationNodeType::Exclusion
        } else if let Some(wm) = nt.wasm_module {
            super::traits::EvaluationNodeType::WasmModule {
                module_name: wm.module_name,
            }
        } else {
            // Default to union if unknown
            super::traits::EvaluationNodeType::Union
        }
    } else {
        super::traits::EvaluationNodeType::Union
    };

    super::traits::EvaluationNode {
        node_type,
        result: node.result,
        children: node
            .children
            .into_iter()
            .map(convert_evaluation_node)
            .collect(),
    }
}

// ============================================================================
// TransportClient Implementation
// ============================================================================

#[async_trait::async_trait]
impl TransportClient for RestTransport {
    async fn check(&self, request: CheckRequest) -> Result<CheckResponse, Error> {
        let api_request = EvaluateRequest {
            evaluations: vec![EvaluateItem {
                subject: request.subject.clone(),
                resource: request.resource.clone(),
                permission: request.permission.clone(),
                context: request.context.map(|c| c.into_value()),
                trace: if request.trace { Some(true) } else { None },
            }],
        };

        // Use SSE endpoint for streaming
        let mut stream = self
            .post_sse::<_, EvaluateResponse>("/access/v1/evaluate", &api_request)
            .await?;

        // Get the first result
        if let Some(result) = stream.next().await {
            let response = result?;
            let allowed = response.decision == "allow";
            return Ok(CheckResponse {
                allowed,
                decision: if allowed {
                    Decision::allowed()
                } else {
                    Decision::denied()
                },
                trace: response.trace.map(convert_trace_response),
            });
        }

        Err(Error::new(
            ErrorKind::InvalidResponse,
            "No response from evaluate endpoint",
        ))
    }

    async fn check_batch(&self, requests: Vec<CheckRequest>) -> Result<Vec<CheckResponse>, Error> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let api_request = EvaluateRequest {
            evaluations: requests
                .iter()
                .map(|r| EvaluateItem {
                    subject: r.subject.clone(),
                    resource: r.resource.clone(),
                    permission: r.permission.clone(),
                    context: r.context.clone().map(|c| c.into_value()),
                    trace: if r.trace { Some(true) } else { None },
                })
                .collect(),
        };

        let mut stream = self
            .post_sse::<_, EvaluateResponse>("/access/v1/evaluate", &api_request)
            .await?;

        let mut results = vec![None; requests.len()];

        while let Some(result) = stream.next().await {
            let response = result?;
            if response.index < results.len() {
                let allowed = response.decision == "allow";
                results[response.index] = Some(CheckResponse {
                    allowed,
                    decision: if allowed {
                        Decision::allowed()
                    } else {
                        Decision::denied()
                    },
                    trace: response.trace.map(convert_trace_response),
                });
            }
        }

        // Convert Option<CheckResponse> to CheckResponse, error if any missing
        results
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                r.ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidResponse,
                        format!("Missing result for check at index {}", i),
                    )
                })
            })
            .collect()
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteResponse, Error> {
        let api_request = WriteRelationshipsRequest {
            relationships: vec![RelationshipDto {
                resource: request.relationship.resource().to_string(),
                relation: request.relationship.relation().to_string(),
                subject: request.relationship.subject().to_string(),
            }],
            expected_revision: None,
        };

        let response: WriteRelationshipsResponse = self
            .post("/access/v1/relationships/write", &api_request)
            .await?;

        Ok(WriteResponse {
            consistency_token: ConsistencyToken::new(response.revision),
        })
    }

    async fn write_batch(&self, requests: Vec<WriteRequest>) -> Result<WriteResponse, Error> {
        if requests.is_empty() {
            return Ok(WriteResponse {
                consistency_token: ConsistencyToken::new(""),
            });
        }

        let api_request = WriteRelationshipsRequest {
            relationships: requests
                .iter()
                .map(|r| RelationshipDto {
                    resource: r.relationship.resource().to_string(),
                    relation: r.relationship.relation().to_string(),
                    subject: r.relationship.subject().to_string(),
                })
                .collect(),
            expected_revision: None,
        };

        let response: WriteRelationshipsResponse = self
            .post("/access/v1/relationships/write", &api_request)
            .await?;

        Ok(WriteResponse {
            consistency_token: ConsistencyToken::new(response.revision),
        })
    }

    async fn delete(&self, relationship: Relationship<'static>) -> Result<(), Error> {
        let path = format!(
            "/access/v1/relationships/{}/{}/{}",
            urlencoding::encode(relationship.resource()),
            urlencoding::encode(relationship.relation()),
            urlencoding::encode(relationship.subject())
        );
        self.delete_request(&path).await
    }

    async fn list_relationships(
        &self,
        resource: Option<&str>,
        relation: Option<&str>,
        subject: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRelationshipsResponse, Error> {
        let api_request = ListRelationshipsApiRequest {
            resource: resource.map(String::from),
            relation: relation.map(String::from),
            subject: subject.map(String::from),
            limit,
            cursor: cursor.map(String::from),
        };

        let mut stream = self
            .post_sse::<_, RelationshipDto>("/access/v1/relationships/list", &api_request)
            .await?;

        let mut relationships = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(dto) => {
                    relationships.push(Relationship::new(dto.resource, dto.relation, dto.subject));
                }
                Err(e) => {
                    // Check if this is the summary event (we'd need special handling)
                    // For now, just skip errors that might be from summary parsing
                    if !e.to_string().contains("summary") {
                        return Err(e);
                    }
                }
            }
        }

        Ok(ListRelationshipsResponse {
            relationships,
            next_cursor: None,
        })
    }

    async fn list_resources(
        &self,
        subject: &str,
        permission: &str,
        resource_type: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListResourcesResponse, Error> {
        let api_request = ListResourcesApiRequest {
            subject: subject.to_string(),
            resource_type: resource_type.unwrap_or("").to_string(),
            permission: permission.to_string(),
            limit,
            cursor: cursor.map(String::from),
            resource_id_pattern: None,
        };

        let mut stream = self
            .post_sse::<_, String>("/access/v1/resources/list", &api_request)
            .await?;

        let mut resources = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(resource) => {
                    resources.push(resource);
                }
                Err(_) => {
                    // Skip summary/non-data events
                }
            }
        }

        Ok(ListResourcesResponse {
            resources,
            next_cursor: None,
        })
    }

    async fn list_subjects(
        &self,
        permission: &str,
        resource: &str,
        subject_type: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListSubjectsResponse, Error> {
        let api_request = ListSubjectsApiRequest {
            resource: resource.to_string(),
            relation: permission.to_string(),
            subject_type: subject_type.map(String::from),
            limit,
            cursor: cursor.map(String::from),
        };

        let mut stream = self
            .post_sse::<_, String>("/access/v1/subjects/list", &api_request)
            .await?;

        let mut subjects = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(subject) => {
                    subjects.push(subject);
                }
                Err(_) => {
                    // Skip summary/non-data events
                }
            }
        }

        Ok(ListSubjectsResponse {
            subjects,
            next_cursor: None,
        })
    }

    fn transport_type(&self) -> Transport {
        Transport::Http
    }

    fn stats(&self) -> TransportStats {
        TransportStats {
            active_transport: Transport::Http,
            fallback_count: 0,
            last_fallback_reason: None,
            last_fallback_at: None,
            grpc: None,
            rest: Some(self.stats.read().clone()),
        }
    }

    async fn health_check(&self) -> Result<(), Error> {
        let url = self
            .base_url
            .join("/healthz")
            .map_err(|e| Error::new(ErrorKind::Configuration, format!("Invalid URL: {}", e)))?;

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::Unavailable,
                format!("Health check failed with status {}", response.status()),
            ))
        }
    }

    async fn simulate(&self, request: SimulateRequest) -> Result<SimulateResponse, Error> {
        #[derive(Serialize)]
        struct SimulateApiRequest {
            subject: String,
            permission: String,
            resource: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            context: Option<serde_json::Value>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            additions: Vec<RelationshipDto>,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            removals: Vec<RelationshipDto>,
        }

        #[derive(Deserialize)]
        struct SimulateApiResponse {
            allowed: bool,
            #[serde(default)]
            decision_id: Option<String>,
        }

        let context_value = request.context.map(|ctx| {
            serde_json::to_value(ctx)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()))
        });

        let api_request = SimulateApiRequest {
            subject: request.subject,
            permission: request.permission,
            resource: request.resource,
            context: context_value,
            additions: request
                .additions
                .iter()
                .map(|r| RelationshipDto {
                    resource: r.resource().to_string(),
                    relation: r.relation().to_string(),
                    subject: r.subject().to_string(),
                })
                .collect(),
            removals: request
                .removals
                .iter()
                .map(|r| RelationshipDto {
                    resource: r.resource().to_string(),
                    relation: r.relation().to_string(),
                    subject: r.subject().to_string(),
                })
                .collect(),
        };

        let api_response: SimulateApiResponse =
            self.post("/access/v1/simulate", &api_request).await?;

        Ok(SimulateResponse {
            allowed: api_response.allowed,
            decision: Decision::new(api_response.allowed),
        })
    }
}

// ============================================================================
// SSE Parsing
// ============================================================================

/// Parses an SSE stream into typed items.
fn parse_sse_stream<T: DeserializeOwned + 'static>(
    byte_stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin + Send + 'static,
    stats: Arc<RwLock<RestStats>>,
) -> impl Stream<Item = Result<T, Error>> {
    let buffer = Arc::new(parking_lot::Mutex::new(String::new()));

    futures::stream::unfold(
        (byte_stream, buffer, stats, false),
        |(mut stream, buffer, stats, mut done)| async move {
            if done {
                return None;
            }

            loop {
                // Check if we have a complete event in the buffer
                {
                    let mut buf = buffer.lock();
                    if let Some(pos) = buf.find("\n\n") {
                        let event = buf[..pos].to_string();
                        *buf = buf[pos + 2..].to_string();
                        drop(buf);

                        // Parse the SSE event
                        if let Some(data) = parse_sse_event(&event) {
                            // Check if this is the summary event
                            if event.contains("event: summary") {
                                // Mark as done after summary
                                done = true;
                                let mut s = stats.write();
                                s.sse_active = s.sse_active.saturating_sub(1);
                            }

                            match serde_json::from_str::<T>(&data) {
                                Ok(item) => {
                                    return Some((Ok(item), (stream, buffer, stats, done)));
                                }
                                Err(_) => {
                                    // If we can't parse, might be a different event type
                                    // Continue to next event
                                    continue;
                                }
                            }
                        }
                    }
                }

                // Need more data
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        let mut buf = buffer.lock();
                        if let Ok(s) = std::str::from_utf8(&bytes) {
                            buf.push_str(s);
                        }
                    }
                    Some(Err(e)) => {
                        {
                            let mut s = stats.write();
                            s.sse_active = s.sse_active.saturating_sub(1);
                            s.requests_failed += 1;
                        }
                        return Some((Err(map_reqwest_error(e)), (stream, buffer, stats, true)));
                    }
                    None => {
                        let mut s = stats.write();
                        s.sse_active = s.sse_active.saturating_sub(1);
                        return None;
                    }
                }
            }
        },
    )
}

/// Parses a single SSE event and returns the data field.
fn parse_sse_event(event: &str) -> Option<String> {
    for line in event.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            return Some(data.to_string());
        }
    }
    None
}

// ============================================================================
// Error Mapping
// ============================================================================

/// Maps reqwest errors to SDK errors.
fn map_reqwest_error(e: reqwest::Error) -> Error {
    if e.is_timeout() {
        Error::new(ErrorKind::Timeout, format!("Request timed out: {}", e))
    } else if e.is_connect() {
        Error::new(ErrorKind::Connection, format!("Connection failed: {}", e))
    } else if e.is_request() {
        Error::new(
            ErrorKind::InvalidArgument,
            format!("Invalid request: {}", e),
        )
    } else {
        Error::new(ErrorKind::Transport, format!("HTTP error: {}", e))
    }
}

/// Maps HTTP status codes to SDK errors.
fn map_status_error(status: u16, body: &str) -> Error {
    let message = if body.is_empty() {
        format!("HTTP {}", status)
    } else {
        // Try to parse as JSON error
        if let Ok(error) = serde_json::from_str::<serde_json::Value>(body) {
            error
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or(body)
                .to_string()
        } else {
            body.to_string()
        }
    };

    match status {
        400 => Error::new(ErrorKind::InvalidArgument, message),
        401 => Error::new(ErrorKind::Unauthorized, message),
        403 => Error::new(ErrorKind::Forbidden, message),
        404 => Error::new(ErrorKind::NotFound, message),
        409 => Error::new(ErrorKind::Conflict, message),
        429 => Error::new(ErrorKind::RateLimited, message),
        500..=599 => Error::new(ErrorKind::Unavailable, message),
        _ => Error::new(ErrorKind::Transport, message),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_event() {
        let event = "data: {\"decision\":\"allow\",\"index\":0}";
        let data = parse_sse_event(event);
        assert_eq!(
            data,
            Some("{\"decision\":\"allow\",\"index\":0}".to_string())
        );
    }

    #[test]
    fn test_parse_sse_event_with_event_type() {
        let event = "event: summary\ndata: {\"total\":5}";
        let data = parse_sse_event(event);
        assert_eq!(data, Some("{\"total\":5}".to_string()));
    }

    #[test]
    fn test_parse_sse_event_empty() {
        let event = "";
        let data = parse_sse_event(event);
        assert_eq!(data, None);
    }

    #[test]
    fn test_map_status_error() {
        let err = map_status_error(401, "");
        assert!(matches!(err.kind(), ErrorKind::Unauthorized));

        let err = map_status_error(404, "{\"error\":\"Not found\"}");
        assert!(matches!(err.kind(), ErrorKind::NotFound));
        assert!(err.to_string().contains("Not found"));

        let err = map_status_error(429, "Rate limited");
        assert!(matches!(err.kind(), ErrorKind::RateLimited));

        let err = map_status_error(503, "Service unavailable");
        assert!(matches!(err.kind(), ErrorKind::Unavailable));
    }

    #[test]
    fn test_rest_transport_builder() {
        let result = RestTransportBuilder::new()
            .base_url("https://api.example.com")
            .unwrap()
            .timeout(Duration::from_secs(60))
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_rest_transport_builder_invalid_url() {
        let result = RestTransportBuilder::new().base_url("not a url");
        assert!(result.is_err());
    }

    #[test]
    fn test_rest_transport_builder_missing_url() {
        let result = RestTransportBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_map_status_error_400() {
        let err = map_status_error(400, "Bad request");
        assert!(matches!(err.kind(), ErrorKind::InvalidArgument));
    }

    #[test]
    fn test_map_status_error_403() {
        let err = map_status_error(403, "Forbidden");
        assert!(matches!(err.kind(), ErrorKind::Forbidden));
    }

    #[test]
    fn test_map_status_error_409() {
        let err = map_status_error(409, "Conflict");
        assert!(matches!(err.kind(), ErrorKind::Conflict));
    }

    #[test]
    fn test_map_status_error_other() {
        let err = map_status_error(418, "I'm a teapot");
        assert!(matches!(err.kind(), ErrorKind::Transport));
    }

    #[test]
    fn test_map_status_error_500_range() {
        for status in [500u16, 502, 503, 504] {
            let err = map_status_error(status, "Server error");
            assert!(matches!(err.kind(), ErrorKind::Unavailable));
        }
    }

    #[test]
    fn test_parse_sse_event_multiline_data() {
        let event = "data: line1\ndata: line2";
        let data = parse_sse_event(event);
        // Should only get first data line
        assert_eq!(data, Some("line1".to_string()));
    }

    #[test]
    fn test_parse_sse_event_no_data() {
        let event = "event: ping";
        let data = parse_sse_event(event);
        assert_eq!(data, None);
    }

    #[test]
    fn test_rest_transport_builder_with_pool_config() {
        let pool_config = PoolConfig {
            max_connections: 50,
            idle_timeout: Duration::from_secs(60),
            max_idle_per_host: 5,
            pool_timeout: Duration::from_secs(15),
            http2_only: false,
            http2_keepalive: Duration::from_secs(10),
        };

        let result = RestTransportBuilder::new()
            .base_url("https://api.example.com")
            .unwrap()
            .pool_config(pool_config)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_rest_transport_stats() {
        let transport = RestTransportBuilder::new()
            .base_url("https://api.example.com")
            .unwrap()
            .build()
            .unwrap();

        let stats = transport.stats();
        assert_eq!(stats.active_transport, Transport::Http);
        assert!(stats.rest.is_some());
        assert!(stats.grpc.is_none());
    }

    #[test]
    fn test_rest_transport_type() {
        let transport = RestTransportBuilder::new()
            .base_url("https://api.example.com")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(transport.transport_type(), Transport::Http);
    }
}

// Wiremock-based async tests
#[cfg(test)]
mod wiremock_tests {
    use super::*;
    use crate::Context;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn create_test_transport(server: &MockServer) -> RestTransport {
        RestTransportBuilder::new()
            .base_url(server.uri())
            .unwrap()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/healthz"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport.health_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_failure() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/healthz"))
            .respond_with(ResponseTemplate::new(503).set_body_string("Service unavailable"))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport.health_check().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::Unavailable));
    }

    #[tokio::test]
    async fn test_check_success() {
        let server = MockServer::start().await;

        // check uses SSE endpoint /access/v1/evaluate with text/event-stream format
        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: {\"decision\": \"allow\", \"index\": 0}\n\n"),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.allowed);
    }

    #[tokio::test]
    async fn test_check_denied() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: {\"decision\": \"deny\", \"index\": 0}\n\n"),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:bob".to_string(),
            permission: "edit".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.allowed);
    }

    #[tokio::test]
    async fn test_check_unauthorized() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Invalid token"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::Unauthorized
        ));
    }

    #[tokio::test]
    async fn test_write_success() {
        let server = MockServer::start().await;

        // write uses /access/v1/relationships/write and expects {revision: ...}
        Mock::given(method("POST"))
            .and(path("/access/v1/relationships/write"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "revision": "rev_abc123",
                "relationships_written": 1
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let relationship = Relationship::new("document:readme", "viewer", "user:alice");
        let request = WriteRequest {
            relationship: relationship.into_owned(),
            idempotency_key: None,
        };

        let result = transport.write(request).await;
        assert!(result.is_ok());
        let _response = result.unwrap();
    }

    #[tokio::test]
    async fn test_write_batch_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/relationships/write"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "revision": "rev_batch123",
                "relationships_written": 2
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let requests = vec![
            WriteRequest {
                relationship: Relationship::new("doc:1", "viewer", "user:alice").into_owned(),
                idempotency_key: None,
            },
            WriteRequest {
                relationship: Relationship::new("doc:2", "viewer", "user:bob").into_owned(),
                idempotency_key: None,
            },
        ];

        let result = transport.write_batch(requests).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_success() {
        let server = MockServer::start().await;

        // delete uses path with URL-encoded components
        Mock::given(method("DELETE"))
            .and(path(
                "/access/v1/relationships/document%3Areadme/viewer/user%3Aalice",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let relationship =
            Relationship::new("document:readme", "viewer", "user:alice").into_owned();

        let result = transport.delete(relationship).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path(
                "/access/v1/relationships/document%3Amissing/viewer/user%3Aalice",
            ))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": "Relationship not found"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let relationship =
            Relationship::new("document:missing", "viewer", "user:alice").into_owned();

        let result = transport.delete(relationship).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::NotFound));
    }

    #[tokio::test]
    async fn test_simulate_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/simulate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "allowed": true,
                "decision_id": "sim_123"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = SimulateRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            additions: vec![
                Relationship::new("document:readme", "viewer", "user:alice").into_owned()
            ],
            removals: vec![],
        };

        let result = transport.simulate(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.allowed);
    }

    #[tokio::test]
    async fn test_check_with_context() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: {\"decision\": \"allow\", \"index\": 0}\n\n"),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let mut context = Context::new();
        context = context.with("ip_address", "192.168.1.1");

        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: Some(context),
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limited() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "error": "Rate limit exceeded"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::RateLimited));
    }

    #[tokio::test]
    async fn test_internal_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": "Internal server error"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_err());
        // 500-599 errors are mapped to ServiceUnavailable in rest.rs
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::Unavailable));
    }

    #[tokio::test]
    async fn test_forbidden_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "error": "Forbidden"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_err());
        // 403 is mapped to PermissionDenied in rest.rs
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::Forbidden));
    }

    #[tokio::test]
    async fn test_write_conflict() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/relationships/write"))
            .respond_with(ResponseTemplate::new(409).set_body_json(serde_json::json!({
                "error": "Relationship already exists"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let relationship = Relationship::new("document:readme", "viewer", "user:alice");
        let request = WriteRequest {
            relationship: relationship.into_owned(),
            idempotency_key: None,
        };

        let result = transport.write(request).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::Conflict));
    }

    #[tokio::test]
    async fn test_service_unavailable() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/healthz"))
            .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
                "error": "Service unavailable"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport.health_check().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err().kind(), ErrorKind::Unavailable));
    }

    #[tokio::test]
    async fn test_bad_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "Invalid request format"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "".to_string(), // Invalid
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_err());
        // 400 errors are mapped to InvalidRequest in rest.rs
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::InvalidArgument
        ));
    }

    #[tokio::test]
    async fn test_simulate_denied() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/simulate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "allowed": false,
                "decision_id": "sim_456"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = SimulateRequest {
            subject: "user:bob".to_string(),
            permission: "edit".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            additions: vec![],
            removals: vec![],
        };

        let result = transport.simulate(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.allowed);
    }

    #[tokio::test]
    async fn test_check_batch_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: {\"decision\": \"allow\", \"index\": 0}\n\ndata: {\"decision\": \"deny\", \"index\": 1}\n\n"),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let requests = vec![
            CheckRequest {
                subject: "user:alice".to_string(),
                permission: "view".to_string(),
                resource: "doc:1".to_string(),
                context: None,
                consistency: None,
                trace: false,
            },
            CheckRequest {
                subject: "user:bob".to_string(),
                permission: "edit".to_string(),
                resource: "doc:2".to_string(),
                context: None,
                consistency: None,
                trace: false,
            },
        ];

        let result = transport.check_batch(requests).await;
        assert!(result.is_ok());
        let responses = result.unwrap();
        assert_eq!(responses.len(), 2);
        assert!(responses[0].allowed);
        assert!(!responses[1].allowed);
    }

    #[tokio::test]
    async fn test_list_relationships_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/relationships/list"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "data: {\"resource\": \"doc:1\", \"relation\": \"viewer\", \"subject\": \"user:alice\"}\n\n\
                         data: {\"resource\": \"doc:2\", \"relation\": \"editor\", \"subject\": \"user:bob\"}\n\n"
                    ),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport
            .list_relationships(None, None, None, Some(10), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.relationships.len(), 2);
    }

    #[tokio::test]
    async fn test_list_resources_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/resources/list"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "data: \"doc:1\"\n\n\
                         data: \"doc:2\"\n\n",
                    ),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport
            .list_resources("user:alice", "view", Some("doc"), Some(10), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.resources.len(), 2);
    }

    #[tokio::test]
    async fn test_list_subjects_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/subjects/list"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "data: \"user:alice\"\n\n\
                         data: \"user:bob\"\n\n",
                    ),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport
            .list_subjects("view", "doc:readme", Some("user"), Some(10), None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.subjects.len(), 2);
    }

    #[tokio::test]
    async fn test_check_with_trace() {
        let server = MockServer::start().await;

        // SSE response with trace data
        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "data: {\"decision\": \"allow\", \"index\": 0, \"trace\": {\"duration_micros\": 123, \"relationships_read\": 5, \"relations_evaluated\": 3, \"root\": {\"result\": true, \"children\": []}}}\n\n"
                    ),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: true,
        };

        let result = transport.check(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.allowed);
        assert!(response.trace.is_some());
        let trace = response.trace.unwrap();
        assert_eq!(trace.duration_micros, 123);
        assert_eq!(trace.relationships_read, 5);
    }

    #[tokio::test]
    async fn test_check_with_trace_node_types() {
        let server = MockServer::start().await;

        // SSE response with complex trace including different node types
        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(
                        "data: {\"decision\": \"allow\", \"index\": 0, \"trace\": {\"duration_micros\": 100, \"relationships_read\": 2, \"relations_evaluated\": 1, \"root\": {\"result\": true, \"node_type\": {\"direct_check\": {\"resource\": \"doc:1\", \"relation\": \"viewer\", \"subject\": \"user:alice\"}}, \"children\": []}}}\n\n"
                    ),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "doc:1".to_string(),
            context: None,
            consistency: None,
            trace: true,
        };

        let result = transport.check(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.allowed);
        assert!(response.trace.is_some());
    }

    #[tokio::test]
    async fn test_authentication_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": "Invalid or expired token"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: None,
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().kind(),
            ErrorKind::Unauthorized
        ));
    }

    #[tokio::test]
    async fn test_rest_transport_debug() {
        let server = MockServer::start().await;
        let transport = create_test_transport(&server).await;
        let debug = format!("{:?}", transport);
        assert!(debug.contains("RestTransport"));
    }

    #[tokio::test]
    async fn test_simulate_with_additions_and_removals() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/simulate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "allowed": true,
                "decision_id": "sim_789"
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = SimulateRequest {
            subject: "user:alice".to_string(),
            permission: "edit".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            additions: vec![
                Relationship::new("document:readme", "editor", "user:alice").into_owned()
            ],
            removals: vec![
                Relationship::new("document:readme", "viewer", "user:alice").into_owned()
            ],
        };

        let result = transport.simulate(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.allowed);
    }

    #[tokio::test]
    async fn test_list_subjects_with_no_limit() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/subjects/list"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: \"user:alice\"\n\n"),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport
            .list_subjects("view", "doc:readme", None, None, None)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.subjects.len(), 1);
    }

    #[tokio::test]
    async fn test_write_with_idempotency_key() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/relationships/write"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "revision": "rev_idem123",
                "relationships_written": 1
            })))
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let relationship = Relationship::new("document:readme", "viewer", "user:alice");
        let request = WriteRequest {
            relationship: relationship.into_owned(),
            idempotency_key: Some("idem_key_123".to_string()),
        };

        let result = transport.write(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_with_consistency() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/access/v1/evaluate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string("data: {\"decision\": \"allow\", \"index\": 0}\n\n"),
            )
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let request = CheckRequest {
            subject: "user:alice".to_string(),
            permission: "view".to_string(),
            resource: "document:readme".to_string(),
            context: None,
            consistency: Some(crate::ConsistencyToken::new("token_abc")),
            trace: false,
        };

        let result = transport.check(request).await;
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);
    }

    #[tokio::test]
    async fn test_user_agent_header_is_sent() {
        use wiremock::matchers::header_exists;

        let server = MockServer::start().await;

        // Verify requests include the User-Agent header
        Mock::given(method("GET"))
            .and(path("/healthz"))
            .and(header_exists("user-agent"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport.health_check().await;
        assert!(result.is_ok());

        // wiremock will fail if the header wasn't present
    }

    #[tokio::test]
    async fn test_user_agent_header_format() {
        use wiremock::matchers::header_regex;

        let server = MockServer::start().await;

        // Verify User-Agent matches expected format: inferadb-rust/X.Y.Z (...)
        Mock::given(method("GET"))
            .and(path("/healthz"))
            .and(header_regex(
                "user-agent",
                r"^inferadb-rust/\d+\.\d+\.\d+ \(.+\)$",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .expect(1)
            .mount(&server)
            .await;

        let transport = create_test_transport(&server).await;
        let result = transport.health_check().await;
        assert!(result.is_ok());
    }
}
