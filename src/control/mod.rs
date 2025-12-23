//! Control plane API for managing InferaDB resources.
//!
//! The control plane provides administrative operations:
//!
//! - Account management (current user)
//! - Organization management
//! - Vault management
//! - Team management
//! - Member management
//! - API client management
//! - JWKS operations
//! - Schema management
//! - Audit logs
//!
//! ## API Hierarchy
//!
//! ```rust,ignore
//! let client = Client::from_env().await?;
//!
//! // Account operations (current user)
//! let account = client.account();
//! let emails = account.emails().list().await?;
//!
//! // JWKS operations
//! let jwks = client.jwks().get().await?;
//!
//! // Organization context
//! let org = client.organization("org_8675309...");
//!
//! // Organization sub-clients
//! let vaults = org.vaults();
//! let members = org.members();
//! let teams = org.teams();
//! let invitations = org.invitations();
//! let audit_logs = org.audit_logs();
//! let api_clients = org.clients();
//!
//! // Vault operations
//! let vault = org.vault("vlt_01JFQGK...");
//! let schemas = vault.schemas();
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! // List vaults in an organization
//! let vaults = client
//!     .organization("org_123")
//!     .vaults()
//!     .list()
//!     .await?;
//!
//! // Create a new vault
//! let vault = client
//!     .organization("org_123")
//!     .vaults()
//!     .create(CreateVaultRequest::new("my-vault"))
//!     .await?;
//!
//! // Push a schema to the vault
//! let result = vault.schemas().push(r#"
//!     entity User {}
//!     entity Document {
//!         relations { owner: User }
//!         permissions { view: owner, edit: owner }
//!     }
//! "#).await?;
//! ```

// Allow dead code for control types not yet integrated
#![allow(dead_code)]

mod account;
mod audit;
mod clients;
mod jwks;
mod members;
mod organizations;
mod schemas;
mod teams;
mod types;
mod vaults;

// Re-export account types
pub use account::{
    Account, AccountClient, AccountStatus, ChangePasswordRequest, Email, EmailsClient, Session,
    SessionsClient, UpdateAccountRequest,
};

// Re-export API client types
pub use clients::{
    AddCertificateRequest, ApiClient, ApiClientsClient, CertificatesClient, ClientCertificate,
    ClientStatus, CreateApiClientRequest, RotateCertificateRequest, UpdateApiClientRequest,
};

// Re-export JWKS types
pub use jwks::{Jwk, Jwks, JwksClient};

// Re-export organization types
pub use organizations::{
    CreateOrganizationRequest, OrganizationControlClient, OrganizationInfo, OrganizationsClient,
    UpdateOrganizationRequest,
};

// Re-export vault types
pub use vaults::{CreateVaultRequest, UpdateVaultRequest, VaultInfo, VaultStatus, VaultsClient};

// Re-export team types
pub use teams::{
    CreateTeamRequest, TeamInfo, TeamMemberInfo, TeamRole, TeamsClient, UpdateTeamRequest,
};

// Re-export member types
pub use members::{
    InvitationInfo, InvitationStatus, InvitationsClient, InviteMemberRequest, MemberInfo,
    MemberStatus, MembersClient, OrgRole, UpdateMemberRequest,
};

// Re-export audit types
pub use audit::{
    ActorInfo, ActorType, AuditAction, AuditEvent, AuditLogsClient, AuditOutcome, ExportFormat,
};

// Re-export schema types
pub use schemas::{
    PushSchemaResult, SchemaChange, SchemaChangeType, SchemaDiff, SchemaInfo, SchemaStatus,
    SchemasClient, ValidationIssue, ValidationResult,
};

// Re-export common types
pub use types::{Page, PageInfo, SortOrder};
