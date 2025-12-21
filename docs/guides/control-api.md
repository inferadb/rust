# Control API

Manage organizations, vaults, schemas, members, and audit logs.

## API Hierarchy

```rust
let client = Client::from_env().await?;

// Organization context
let org = client.organization("org_8675309...");

// Vault operations (unified: authorization + management)
let vault = org.vault("vlt_01JFQGK...");

// Account (current user)
let account = client.account();
```

| Method             | Returns              | Use Case                   |
| ------------------ | -------------------- | -------------------------- |
| `organization(id)` | `OrganizationClient` | Org-scoped operations      |
| `org.vault(id)`    | `VaultClient`        | Authorization + management |
| `org.vaults()`     | `VaultsClient`       | List/create vaults         |
| `org.members()`    | `MembersClient`      | Member management          |
| `account()`        | `AccountClient`      | Current user operations    |

## Organization Management

```rust
let org = client.organization("org_8675309...");

// Get organization details
let info = org.get().await?;
println!("Name: {}, Plan: {:?}", info.name, info.plan);

// Update organization
org.update(UpdateOrganization {
    name: Some("Acme Corporation".into()),
    ..Default::default()
}).await?;

// Delete (owner only, requires confirmation)
org.delete()
    .confirm("DELETE ACME")
    .await?;
```

### Create Organization

```rust
let org = client
    .organizations()
    .create(CreateOrganization {
        name: "Acme Corp".into(),
        slug: Some("acme".into()),
        ..Default::default()
    })
    .await?;

println!("Created: {}", org.id);
```

### List Organizations

```rust
let orgs = client.organizations().list().await?;

for org in &orgs {
    println!("{}: {} (role: {})", org.id, org.name, org.role);
}
```

## Vault Management

`VaultClient` provides both authorization operations (check, relationships) and management operations:

```rust
let vault = client.organization("org_...").vault("vlt_...");

// Authorization
vault.check("user:alice", "view", "doc:1").await?;

// Management (same client)
let info = vault.get().await?;
vault.schemas().get_active().await?;
```

### Create Vault

```rust
let vault = client
    .organization("org_...")
    .vaults()
    .create(CreateVault {
        name: "production".into(),
        ..Default::default()
    })
    .await?;
```

### List Vaults

```rust
let vaults = client
    .organization("org_...")
    .vaults()
    .list()
    .await?;
```

### Delete Vault

```rust
vault.delete().await?;
```

## Schema Management

Schemas define entity types, relations, and permissions.

```rust
let schemas = vault.schemas();

// Get active schema
let active = schemas.get_active().await?;
println!("Version: {}", active.version);

// List all schema versions
let versions = schemas.list().await?;

// Push new schema
let result = schemas.push(r#"
    entity User {}
    entity Document {
        relations { owner: User, viewer: User }
        permissions { view: viewer | owner, edit: owner }
    }
"#).await?;

println!("New version: {}", result.version);

// Activate specific version
schemas.activate(&version_id).await?;
```

### Schema Validation

```rust
// Validate before pushing
let validation = schemas
    .validate(schema_content)
    .await?;

if !validation.is_valid() {
    for error in &validation.errors {
        eprintln!("Line {}: {}", error.line, error.message);
    }
}
```

## Member Management

```rust
let members = client.organization("org_...").members();

// List members
let list = members.list().await?;
for member in &list {
    println!("{}: {} ({})", member.user_id, member.email, member.role);
}

// Invite new member
members.invite(Invite {
    email: "alice@example.com".into(),
    role: OrgRole::Member,
    ..Default::default()
}).await?;

// Update role
members.update(&user_id, UpdateMember {
    role: Some(OrgRole::Admin),
    ..Default::default()
}).await?;

// Remove member
members.remove(&user_id).await?;
```

### Invitations

```rust
let invites = client.organization("org_...").invitations();

// List pending
let pending = invites.list().await?;

// Resend invitation
invites.resend(&invite_id).await?;

// Revoke invitation
invites.revoke(&invite_id).await?;
```

## Team Management

```rust
let teams = client.organization("org_...").teams();

// Create team
let team = teams.create(CreateTeam {
    name: "Engineering".into(),
    ..Default::default()
}).await?;

// List teams
let list = teams.list().await?;

// Add member to team
teams.add_member(&team_id, &user_id).await?;

// Remove member from team
teams.remove_member(&team_id, &user_id).await?;
```

## Audit Logs

Query audit logs for compliance and debugging.

```rust
let logs = client.organization("org_...").audit_logs();

// List recent events
let events = logs.list().await?;

for event in &events {
    println!("{}: {} {} on {}",
        event.timestamp,
        event.actor,
        event.action,
        event.resource
    );
}

// Filter by actor
let user_events = logs
    .actor(&user_id)
    .list()
    .await?;

// Filter by action type
let writes = logs
    .action(AuditAction::RelationshipWrite)
    .list()
    .await?;

// Filter by time range
let recent = logs
    .after(Utc::now() - Duration::hours(24))
    .before(Utc::now())
    .list()
    .await?;
```

## Account Management

Manage the current authenticated user's account.

```rust
let account = client.account();

// Get account details
let info = account.get().await?;
println!("Email: {}", info.primary_email);

// Update account
account.update(UpdateAccount {
    name: Some("New Name".into()),
    ..Default::default()
}).await?;
```

### Email Management

```rust
let emails = client.account().emails();

// List emails
let list = emails.list().await?;

// Add email
emails.add("new@example.com").await?;

// Set primary
emails.set_primary(&email_id).await?;
```

### Session Management

```rust
let sessions = client.account().sessions();

// List active sessions
let list = sessions.list().await?;

// Revoke specific session
sessions.revoke(&session_id).await?;

// Revoke all other sessions
sessions.revoke_others().await?;
```

## API Clients (Service Accounts)

Create service accounts for machine-to-machine authentication.

```rust
let clients = client.organization("org_...").clients();

// Create API client
let api_client = clients.create(CreateClient {
    name: "backend-service".into(),
    ..Default::default()
}).await?;

println!("Client ID: {}", api_client.id);

// Manage certificates
let certs = clients.client(&api_client.id).certificates();
let cert = certs.create(CreateCertificate {
    public_key: public_key_pem,
    ..Default::default()
}).await?;
```

## Vault Tokens

Create scoped tokens for specific vault operations.

```rust
let tokens = vault.tokens();

// Create read-only token
let token = tokens.create(CreateToken {
    name: "read-only".into(),
    permissions: vec![Permission::Check],
    expires_in: Some(Duration::days(30)),
    ..Default::default()
}).await?;

// List tokens
let list = tokens.list().await?;

// Revoke token
tokens.revoke(&token_id).await?;
```

## Best Practices

1. **Reuse clients** - Create `Client` once at startup, share via app state
2. **Store vault references** - `VaultClient` is cheap to clone (Arc internally)
3. **Use audit logs** - Query audit logs when debugging permission issues
4. **Scope tokens narrowly** - Create tokens with minimal required permissions
5. **Monitor schema versions** - Track which version is active in production
