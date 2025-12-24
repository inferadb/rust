//! gRPC transport implementation using tonic.
//!
//! This module provides gRPC transport for high-performance communication
//! with InferaDB services. It uses HTTP/2 and provides native streaming support.
//!
//! ## Usage
//!
//! The gRPC transport is the preferred transport for production use due to
//! its performance benefits and native support for bidirectional streaming.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use parking_lot::RwLock;
use tonic::transport::{Channel, Endpoint};
use url::Url;

use tonic::service::interceptor::InterceptedService;

use super::proto::inferadb_service_client::InferadbServiceClient;
use super::proto::{self as pb};
use crate::config::{RetryConfig, TlsConfig};
use crate::transport::traits::{
    CheckRequest, CheckResponse, GrpcStats, ListRelationshipsResponse, ListResourcesResponse,
    ListSubjectsResponse, PoolConfig, SimulateRequest, SimulateResponse, Transport,
    TransportClient, TransportStats, WriteRequest, WriteResponse,
};
use crate::types::{ConsistencyToken, Decision, Relationship};
use crate::user_agent;
use crate::Error;

/// Interceptor that adds user-agent metadata to all gRPC requests.
#[allow(clippy::result_large_err)] // tonic::Status is the required error type for interceptors
fn user_agent_interceptor(
    mut req: tonic::Request<()>,
) -> Result<tonic::Request<()>, tonic::Status> {
    req.metadata_mut().insert(
        "user-agent",
        user_agent::user_agent()
            .parse()
            .unwrap_or_else(|_| tonic::metadata::MetadataValue::from_static("inferadb-rust")),
    );
    Ok(req)
}

/// Type alias for the intercepted gRPC client.
type InterceptedClient = InferadbServiceClient<
    InterceptedService<
        Channel,
        fn(tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status>,
    >,
>;

/// gRPC transport client using tonic.
///
/// This transport provides high-performance communication with InferaDB
/// services over HTTP/2 with native streaming support.
#[derive(Clone)]
pub struct GrpcTransport {
    client: InterceptedClient,
    stats: Arc<RwLock<GrpcStats>>,
}

impl GrpcTransport {
    /// Creates a new gRPC transport.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The gRPC endpoint URL
    /// * `tls_config` - TLS configuration
    /// * `pool_config` - Connection pool configuration
    /// * `retry_config` - Retry behavior configuration
    /// * `timeout` - Request timeout
    pub async fn new(
        base_url: Url,
        tls_config: &TlsConfig,
        pool_config: &PoolConfig,
        _retry_config: RetryConfig,
        timeout: Duration,
    ) -> Result<Self, Error> {
        let endpoint = Endpoint::from_shared(base_url.to_string())
            .map_err(|e| Error::configuration(format!("Invalid gRPC URL: {}", e)))?
            .timeout(timeout)
            .connect_timeout(pool_config.pool_timeout)
            .concurrency_limit(pool_config.max_connections as usize);

        // Configure TLS
        let endpoint = if base_url.scheme() == "https" {
            let mut tls = tonic::transport::ClientTlsConfig::new();

            // Add custom CA if configured
            if let Some(ref ca_pem) = tls_config.ca_cert_pem {
                let cert = tonic::transport::Certificate::from_pem(ca_pem);
                tls = tls.ca_certificate(cert);
            }

            // Add client certificate for mTLS if configured
            if tls_config.is_mtls_configured() {
                if let (Some(ref cert_path), Some(ref key_path)) =
                    (&tls_config.client_cert_file, &tls_config.client_key_file)
                {
                    let cert_pem = std::fs::read_to_string(cert_path).map_err(|e| {
                        Error::configuration(format!("Failed to read client cert: {}", e))
                    })?;
                    let key_pem = std::fs::read_to_string(key_path).map_err(|e| {
                        Error::configuration(format!("Failed to read client key: {}", e))
                    })?;
                    let identity = tonic::transport::Identity::from_pem(&cert_pem, &key_pem);
                    tls = tls.identity(identity);
                }
            }

            endpoint
                .tls_config(tls)
                .map_err(|e| Error::configuration(format!("Failed to configure TLS: {}", e)))?
        } else {
            endpoint
        };

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| Error::connection(format!("Failed to connect to gRPC server: {}", e)))?;

        // Create client with user-agent interceptor
        let interceptor: fn(tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> =
            user_agent_interceptor;
        let client = InferadbServiceClient::with_interceptor(channel, interceptor);

        Ok(Self {
            client,
            stats: Arc::new(RwLock::new(GrpcStats::default())),
        })
    }

    /// Returns a builder for configuring the gRPC transport.
    pub fn builder(base_url: Url) -> GrpcTransportBuilder {
        GrpcTransportBuilder {
            base_url,
            tls_config: TlsConfig::default(),
            pool_config: PoolConfig::default(),
            retry_config: RetryConfig::default(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Converts a tonic status to our Error type.
    fn convert_error(status: tonic::Status) -> Error {
        let message = status.message().to_string();
        match status.code() {
            tonic::Code::InvalidArgument => Error::invalid_argument(message),
            tonic::Code::NotFound => Error::not_found(message),
            tonic::Code::PermissionDenied => Error::forbidden(message),
            tonic::Code::Unauthenticated => Error::unauthorized(message),
            tonic::Code::ResourceExhausted => Error::rate_limited(None),
            tonic::Code::Unavailable => Error::unavailable(message),
            tonic::Code::DeadlineExceeded => Error::timeout(message),
            _ => Error::connection(format!("gRPC error: {}", message)),
        }
    }

    /// Increments the requests_sent counter.
    fn increment_requests(&self) {
        self.stats.write().requests_sent += 1;
    }

    /// Increments the requests_failed counter.
    fn increment_failures(&self) {
        self.stats.write().requests_failed += 1;
    }

    /// Converts a proto Decision to our Decision type.
    fn convert_decision(decision: i32) -> bool {
        decision == pb::Decision::Allow as i32
    }

    /// Converts a proto DecisionTrace to our DecisionTrace type.
    fn convert_trace(trace: pb::DecisionTrace) -> super::traits::DecisionTrace {
        super::traits::DecisionTrace {
            duration_micros: trace.duration_micros,
            relationships_read: trace.relationships_read,
            relations_evaluated: trace.relations_evaluated,
            root: trace.root.map(Self::convert_evaluation_node),
        }
    }

    /// Converts a proto EvaluationNode to our EvaluationNode type.
    fn convert_evaluation_node(node: pb::EvaluationNode) -> super::traits::EvaluationNode {
        let node_type = node
            .node_type
            .and_then(|nt| nt.r#type)
            .map(|t| match t {
                pb::node_type::Type::DirectCheck(dc) => {
                    super::traits::EvaluationNodeType::DirectCheck {
                        resource: dc.resource,
                        relation: dc.relation,
                        subject: dc.subject,
                    }
                }
                pb::node_type::Type::ComputedUserset(cu) => {
                    super::traits::EvaluationNodeType::ComputedUserset {
                        relation: cu.relation,
                    }
                }
                pb::node_type::Type::RelatedObjectUserset(rou) => {
                    super::traits::EvaluationNodeType::RelatedObjectUserset {
                        relationship: rou.relationship,
                        computed: rou.computed,
                    }
                }
                pb::node_type::Type::Union(_) => super::traits::EvaluationNodeType::Union,
                pb::node_type::Type::Intersection(_) => {
                    super::traits::EvaluationNodeType::Intersection
                }
                pb::node_type::Type::Exclusion(_) => super::traits::EvaluationNodeType::Exclusion,
                pb::node_type::Type::WasmModule(wm) => {
                    super::traits::EvaluationNodeType::WasmModule {
                        module_name: wm.module_name,
                    }
                }
            })
            .unwrap_or(super::traits::EvaluationNodeType::Union);

        super::traits::EvaluationNode {
            node_type,
            result: node.result,
            children: node
                .children
                .into_iter()
                .map(Self::convert_evaluation_node)
                .collect(),
        }
    }

    /// Converts a proto Relationship to our Relationship type.
    fn convert_relationship(rel: pb::Relationship) -> Relationship<'static> {
        Relationship::new(rel.resource, rel.relation, rel.subject).into_owned()
    }
}

/// Builder for configuring a gRPC transport.
pub struct GrpcTransportBuilder {
    base_url: Url,
    tls_config: TlsConfig,
    pool_config: PoolConfig,
    retry_config: RetryConfig,
    timeout: Duration,
}

impl GrpcTransportBuilder {
    /// Sets the TLS configuration.
    #[must_use]
    pub fn tls_config(mut self, config: TlsConfig) -> Self {
        self.tls_config = config;
        self
    }

    /// Sets the connection pool configuration.
    #[must_use]
    pub fn pool_config(mut self, config: PoolConfig) -> Self {
        self.pool_config = config;
        self
    }

    /// Sets the retry configuration.
    #[must_use]
    pub fn retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builds the gRPC transport.
    pub async fn build(self) -> Result<GrpcTransport, Error> {
        GrpcTransport::new(
            self.base_url,
            &self.tls_config,
            &self.pool_config,
            self.retry_config,
            self.timeout,
        )
        .await
    }
}

#[async_trait::async_trait]
impl TransportClient for GrpcTransport {
    async fn check(&self, request: CheckRequest) -> Result<CheckResponse, Error> {
        self.increment_requests();

        let pb_request = pb::EvaluateRequest {
            subject: request.subject,
            permission: request.permission,
            resource: request.resource,
            context: request
                .context
                .map(|c| serde_json::to_string(&c).unwrap_or_default()),
            trace: Some(request.trace),
        };

        // Use streaming API with a single request
        let stream = futures::stream::once(async { pb_request });
        let mut client = self.client.clone();

        let response = client.evaluate(stream).await.map_err(Self::convert_error)?;

        let mut stream = response.into_inner();

        if let Some(result) = stream.next().await {
            let eval_response = result.map_err(Self::convert_error)?;

            if let Some(error) = eval_response.error {
                self.increment_failures();
                return Err(Error::internal(error));
            }

            let allowed = Self::convert_decision(eval_response.decision);

            Ok(CheckResponse {
                allowed,
                decision: Decision::new(allowed),
                trace: eval_response.trace.map(Self::convert_trace),
            })
        } else {
            self.increment_failures();
            Err(Error::internal("No response received from server"))
        }
    }

    async fn check_batch(&self, requests: Vec<CheckRequest>) -> Result<Vec<CheckResponse>, Error> {
        self.increment_requests();

        let pb_requests: Vec<pb::EvaluateRequest> = requests
            .into_iter()
            .map(|r| pb::EvaluateRequest {
                subject: r.subject,
                permission: r.permission,
                resource: r.resource,
                context: r
                    .context
                    .map(|c| serde_json::to_string(&c).unwrap_or_default()),
                trace: Some(r.trace),
            })
            .collect();

        let stream = futures::stream::iter(pb_requests);
        let mut client = self.client.clone();

        let response = client.evaluate(stream).await.map_err(Self::convert_error)?;

        let mut stream = response.into_inner();
        let mut results = Vec::new();

        while let Some(result) = stream.next().await {
            let eval_response = result.map_err(Self::convert_error)?;

            if let Some(error) = eval_response.error {
                self.increment_failures();
                return Err(Error::internal(error));
            }

            let allowed = Self::convert_decision(eval_response.decision);
            results.push(CheckResponse {
                allowed,
                decision: Decision::new(allowed),
                trace: eval_response.trace.map(Self::convert_trace),
            });
        }

        Ok(results)
    }

    async fn write(&self, request: WriteRequest) -> Result<WriteResponse, Error> {
        self.increment_requests();

        let pb_request = pb::WriteRequest {
            relationships: vec![pb::Relationship {
                resource: request.relationship.resource().to_string(),
                relation: request.relationship.relation().to_string(),
                subject: request.relationship.subject().to_string(),
            }],
        };

        let stream = futures::stream::once(async { pb_request });
        let mut client = self.client.clone();

        let response = client
            .write_relationships(stream)
            .await
            .map_err(Self::convert_error)?;

        let write_response = response.into_inner();

        Ok(WriteResponse {
            consistency_token: ConsistencyToken::new(&write_response.revision),
        })
    }

    async fn write_batch(&self, requests: Vec<WriteRequest>) -> Result<WriteResponse, Error> {
        self.increment_requests();

        let relationships: Vec<pb::Relationship> = requests
            .into_iter()
            .map(|r| pb::Relationship {
                resource: r.relationship.resource().to_string(),
                relation: r.relationship.relation().to_string(),
                subject: r.relationship.subject().to_string(),
            })
            .collect();

        let pb_request = pb::WriteRequest { relationships };

        let stream = futures::stream::once(async { pb_request });
        let mut client = self.client.clone();

        let response = client
            .write_relationships(stream)
            .await
            .map_err(Self::convert_error)?;

        let write_response = response.into_inner();

        Ok(WriteResponse {
            consistency_token: ConsistencyToken::new(&write_response.revision),
        })
    }

    async fn delete(&self, relationship: Relationship<'static>) -> Result<(), Error> {
        self.increment_requests();

        let pb_request = pb::DeleteRequest {
            filter: None,
            relationships: vec![pb::Relationship {
                resource: relationship.resource().to_string(),
                relation: relationship.relation().to_string(),
                subject: relationship.subject().to_string(),
            }],
            limit: None,
        };

        let stream = futures::stream::once(async { pb_request });
        let mut client = self.client.clone();

        client
            .delete_relationships(stream)
            .await
            .map_err(Self::convert_error)?;

        Ok(())
    }

    async fn list_relationships(
        &self,
        resource: Option<&str>,
        relation: Option<&str>,
        subject: Option<&str>,
        limit: Option<u32>,
        cursor: Option<&str>,
    ) -> Result<ListRelationshipsResponse, Error> {
        self.increment_requests();

        let pb_request = pb::ListRelationshipsRequest {
            resource: resource.map(String::from),
            relation: relation.map(String::from),
            subject: subject.map(String::from),
            limit,
            cursor: cursor.map(String::from),
        };

        let mut client = self.client.clone();
        let response = client
            .list_relationships(pb_request)
            .await
            .map_err(Self::convert_error)?;

        let mut stream = response.into_inner();
        let mut relationships = Vec::new();
        let mut next_cursor = None;

        while let Some(result) = stream.next().await {
            let list_response = result.map_err(Self::convert_error)?;
            if let Some(rel) = list_response.relationship {
                relationships.push(Self::convert_relationship(rel));
            }

            if let Some(c) = list_response.cursor {
                next_cursor = Some(c);
            }
        }

        Ok(ListRelationshipsResponse {
            relationships,
            next_cursor,
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
        self.increment_requests();

        let pb_request = pb::ListResourcesRequest {
            subject: subject.to_string(),
            permission: permission.to_string(),
            resource_type: resource_type.unwrap_or_default().to_string(),
            limit,
            cursor: cursor.map(String::from),
            resource_id_pattern: None,
        };

        let mut client = self.client.clone();
        let response = client
            .list_resources(pb_request)
            .await
            .map_err(Self::convert_error)?;

        let mut stream = response.into_inner();
        let mut resources = Vec::new();
        let mut next_cursor = None;

        while let Some(result) = stream.next().await {
            let list_response = result.map_err(Self::convert_error)?;
            if !list_response.resource.is_empty() {
                resources.push(list_response.resource);
            }

            if let Some(c) = list_response.cursor {
                next_cursor = Some(c);
            }
        }

        Ok(ListResourcesResponse {
            resources,
            next_cursor,
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
        self.increment_requests();

        let pb_request = pb::ListSubjectsRequest {
            resource: resource.to_string(),
            relation: permission.to_string(),
            subject_type: subject_type.map(String::from),
            limit,
            cursor: cursor.map(String::from),
        };

        let mut client = self.client.clone();
        let response = client
            .list_subjects(pb_request)
            .await
            .map_err(Self::convert_error)?;

        let mut stream = response.into_inner();
        let mut subjects = Vec::new();
        let mut next_cursor = None;

        while let Some(result) = stream.next().await {
            let list_response = result.map_err(Self::convert_error)?;
            if !list_response.subject.is_empty() {
                subjects.push(list_response.subject);
            }

            if let Some(c) = list_response.cursor {
                next_cursor = Some(c);
            }
        }

        Ok(ListSubjectsResponse {
            subjects,
            next_cursor,
        })
    }

    fn transport_type(&self) -> Transport {
        Transport::Grpc
    }

    fn stats(&self) -> TransportStats {
        let grpc = self.stats.read().clone();
        TransportStats {
            active_transport: Transport::Grpc,
            fallback_count: 0,
            last_fallback_reason: None,
            last_fallback_at: None,
            grpc: Some(grpc),
            rest: None,
        }
    }

    async fn health_check(&self) -> Result<(), Error> {
        let mut client = self.client.clone();

        client
            .health(pb::HealthRequest {})
            .await
            .map_err(Self::convert_error)?;

        Ok(())
    }

    async fn simulate(&self, request: SimulateRequest) -> Result<SimulateResponse, Error> {
        self.increment_requests();

        let context_relationships: Vec<pb::Relationship> = request
            .additions
            .iter()
            .map(|r| pb::Relationship {
                resource: r.resource().to_string(),
                relation: r.relation().to_string(),
                subject: r.subject().to_string(),
            })
            .collect();

        let pb_request = pb::SimulateRequest {
            context_relationships,
            check: Some(pb::SimulateCheck {
                subject: request.subject,
                resource: request.resource,
                permission: request.permission,
                context: request
                    .context
                    .map(|c| serde_json::to_string(&c).unwrap_or_default()),
            }),
        };

        let mut client = self.client.clone();
        let response = client
            .simulate(pb_request)
            .await
            .map_err(Self::convert_error)?;

        let sim_response = response.into_inner();
        let allowed = Self::convert_decision(sim_response.decision);

        Ok(SimulateResponse {
            allowed,
            decision: Decision::new(allowed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TlsConfig;
    use crate::transport::traits::PoolConfig;

    #[test]
    fn test_grpc_transport_builder() {
        let url = Url::parse("https://api.example.com").unwrap();
        let builder = GrpcTransport::builder(url)
            .timeout(Duration::from_secs(60))
            .retry_config(RetryConfig::disabled());

        assert_eq!(builder.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_grpc_transport_builder_all_methods() {
        let url = Url::parse("https://api.example.com").unwrap();
        let tls_config = TlsConfig::default();
        let pool_config = PoolConfig {
            max_connections: 100,
            idle_timeout: Duration::from_secs(60),
            max_idle_per_host: 20,
            pool_timeout: Duration::from_secs(5),
            http2_only: true,
            http2_keepalive: Duration::from_secs(30),
        };
        let retry_config = RetryConfig::default();

        let builder = GrpcTransport::builder(url)
            .tls_config(tls_config)
            .pool_config(pool_config)
            .retry_config(retry_config)
            .timeout(Duration::from_secs(120));

        assert_eq!(builder.timeout, Duration::from_secs(120));
        assert_eq!(builder.pool_config.max_connections, 100);
        assert_eq!(builder.pool_config.max_idle_per_host, 20);
    }

    #[test]
    fn test_convert_decision() {
        assert!(GrpcTransport::convert_decision(pb::Decision::Allow as i32));
        assert!(!GrpcTransport::convert_decision(pb::Decision::Deny as i32));
        assert!(!GrpcTransport::convert_decision(
            pb::Decision::Unspecified as i32
        ));
        // Test unknown decision values
        assert!(!GrpcTransport::convert_decision(999));
        assert!(!GrpcTransport::convert_decision(-1));
    }

    #[test]
    fn test_convert_error_invalid_argument() {
        let status = tonic::Status::invalid_argument("bad request");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("bad request"));
    }

    #[test]
    fn test_convert_error_not_found() {
        let status = tonic::Status::not_found("not found");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn test_convert_error_unauthenticated() {
        let status = tonic::Status::unauthenticated("auth failed");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("auth failed"));
    }

    #[test]
    fn test_convert_error_permission_denied() {
        let status = tonic::Status::permission_denied("forbidden");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("forbidden"));
    }

    #[test]
    fn test_convert_error_resource_exhausted() {
        let status = tonic::Status::resource_exhausted("rate limited");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().to_lowercase().contains("rate"));
    }

    #[test]
    fn test_convert_error_unavailable() {
        let status = tonic::Status::unavailable("service down");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("service down"));
    }

    #[test]
    fn test_convert_error_deadline_exceeded() {
        let status = tonic::Status::deadline_exceeded("timeout");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("timeout"));
    }

    #[test]
    fn test_convert_error_unknown() {
        let status = tonic::Status::unknown("something went wrong");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("something went wrong"));
    }

    #[test]
    fn test_convert_error_internal() {
        let status = tonic::Status::internal("internal error");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("internal error"));
    }

    #[test]
    fn test_convert_error_cancelled() {
        let status = tonic::Status::cancelled("cancelled");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("cancelled"));
    }

    #[test]
    fn test_convert_error_already_exists() {
        let status = tonic::Status::already_exists("already exists");
        let error = GrpcTransport::convert_error(status);
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn test_convert_relationship() {
        let pb_rel = pb::Relationship {
            resource: "document:123".to_string(),
            relation: "viewer".to_string(),
            subject: "user:alice".to_string(),
        };

        let rel = GrpcTransport::convert_relationship(pb_rel);
        assert_eq!(rel.resource(), "document:123");
        assert_eq!(rel.relation(), "viewer");
        assert_eq!(rel.subject(), "user:alice");
    }

    #[test]
    fn test_convert_trace_empty() {
        let trace = pb::DecisionTrace {
            decision: pb::Decision::Allow as i32,
            duration_micros: 100,
            relationships_read: 5,
            relations_evaluated: 3,
            root: None,
        };

        let converted = GrpcTransport::convert_trace(trace);
        assert_eq!(converted.duration_micros, 100);
        assert_eq!(converted.relationships_read, 5);
        assert_eq!(converted.relations_evaluated, 3);
        assert!(converted.root.is_none());
    }

    #[test]
    fn test_convert_trace_with_root() {
        let trace = pb::DecisionTrace {
            decision: pb::Decision::Deny as i32,
            duration_micros: 200,
            relationships_read: 10,
            relations_evaluated: 8,
            root: Some(pb::EvaluationNode {
                node_type: None,
                result: true,
                children: vec![],
            }),
        };

        let converted = GrpcTransport::convert_trace(trace);
        assert_eq!(converted.duration_micros, 200);
        assert!(converted.root.is_some());
        assert!(converted.root.unwrap().result);
    }

    #[test]
    fn test_convert_evaluation_node_direct_check() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::DirectCheck(pb::DirectCheck {
                    resource: "doc:1".to_string(),
                    relation: "owner".to_string(),
                    subject: "user:bob".to_string(),
                })),
            }),
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        assert!(converted.result);
        match converted.node_type {
            super::super::traits::EvaluationNodeType::DirectCheck {
                resource,
                relation,
                subject,
            } => {
                assert_eq!(resource, "doc:1");
                assert_eq!(relation, "owner");
                assert_eq!(subject, "user:bob");
            }
            _ => panic!("Expected DirectCheck"),
        }
    }

    #[test]
    fn test_convert_evaluation_node_computed_userset() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::ComputedUserset(pb::ComputedUserset {
                    relation: "editor".to_string(),
                    relationship: "parent".to_string(),
                })),
            }),
            result: false,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        assert!(!converted.result);
        match converted.node_type {
            super::super::traits::EvaluationNodeType::ComputedUserset { relation } => {
                assert_eq!(relation, "editor");
            }
            _ => panic!("Expected ComputedUserset"),
        }
    }

    #[test]
    fn test_convert_evaluation_node_related_object_userset() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::RelatedObjectUserset(
                    pb::RelatedObjectUserset {
                        relationship: "parent".to_string(),
                        computed: "owner".to_string(),
                    },
                )),
            }),
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        match converted.node_type {
            super::super::traits::EvaluationNodeType::RelatedObjectUserset {
                relationship,
                computed,
            } => {
                assert_eq!(relationship, "parent");
                assert_eq!(computed, "owner");
            }
            _ => panic!("Expected RelatedObjectUserset"),
        }
    }

    #[test]
    fn test_convert_evaluation_node_union() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::Union(pb::Union {})),
            }),
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        assert!(matches!(
            converted.node_type,
            super::super::traits::EvaluationNodeType::Union
        ));
    }

    #[test]
    fn test_convert_evaluation_node_intersection() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::Intersection(pb::Intersection {})),
            }),
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        assert!(matches!(
            converted.node_type,
            super::super::traits::EvaluationNodeType::Intersection
        ));
    }

    #[test]
    fn test_convert_evaluation_node_exclusion() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::Exclusion(pb::Exclusion {})),
            }),
            result: false,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        assert!(matches!(
            converted.node_type,
            super::super::traits::EvaluationNodeType::Exclusion
        ));
    }

    #[test]
    fn test_convert_evaluation_node_wasm_module() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::WasmModule(pb::WasmModule {
                    module_name: "my_module".to_string(),
                })),
            }),
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        match converted.node_type {
            super::super::traits::EvaluationNodeType::WasmModule { module_name } => {
                assert_eq!(module_name, "my_module");
            }
            _ => panic!("Expected WasmModule"),
        }
    }

    #[test]
    fn test_convert_evaluation_node_with_children() {
        let child1 = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::Union(pb::Union {})),
            }),
            result: true,
            children: vec![],
        };

        let child2 = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::Intersection(pb::Intersection {})),
            }),
            result: false,
            children: vec![],
        };

        let parent = pb::EvaluationNode {
            node_type: Some(pb::NodeType {
                r#type: Some(pb::node_type::Type::Union(pb::Union {})),
            }),
            result: true,
            children: vec![child1, child2],
        };

        let converted = GrpcTransport::convert_evaluation_node(parent);
        assert_eq!(converted.children.len(), 2);
        assert!(converted.children[0].result);
        assert!(!converted.children[1].result);
    }

    #[test]
    fn test_convert_evaluation_node_no_type() {
        let node = pb::EvaluationNode {
            node_type: None,
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        // Should default to Union
        assert!(matches!(
            converted.node_type,
            super::super::traits::EvaluationNodeType::Union
        ));
    }

    #[test]
    fn test_convert_evaluation_node_empty_type() {
        let node = pb::EvaluationNode {
            node_type: Some(pb::NodeType { r#type: None }),
            result: true,
            children: vec![],
        };

        let converted = GrpcTransport::convert_evaluation_node(node);
        // Should default to Union
        assert!(matches!(
            converted.node_type,
            super::super::traits::EvaluationNodeType::Union
        ));
    }

    #[test]
    fn test_user_agent_interceptor() {
        let request = tonic::Request::new(());
        let result = user_agent_interceptor(request);

        assert!(result.is_ok());
        let req = result.unwrap();
        let user_agent = req.metadata().get("user-agent");
        assert!(user_agent.is_some());
        let ua_value = user_agent.unwrap().to_str().unwrap();
        assert!(ua_value.contains("inferadb-rust"));
    }
}
