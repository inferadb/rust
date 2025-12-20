# Time-Based Permissions

This guide covers implementing permissions with temporal constraints, including expiring access, scheduled permissions, and time-windowed access.

## Overview

Time-based permissions enable:
- **Temporary access** - Contractor or guest access that auto-expires
- **Scheduled access** - Permissions that activate at a future time
- **Time-windowed access** - Access only during specific hours (business hours, on-call rotations)
- **Audit queries** - Check what permissions existed at a point in time

## Expiring Permissions

Grant access that automatically expires:

```rust
use inferadb::{Relationship, Expiration};
use std::time::Duration;
use chrono::{Utc, Duration as ChronoDuration};

// Grant temporary access for 24 hours
client.write(
    Relationship::new("doc:sensitive", "viewer", "user:contractor")
        .expires_in(Duration::from_secs(86400))  // 24 hours
).await?;

// Grant access until specific time
client.write(
    Relationship::new("doc:project", "editor", "user:temp")
        .expires_at(Utc::now() + ChronoDuration::days(30))
).await?;

// Access automatically denied after expiration
// No cleanup needed - expired relationships filtered at query time
```

## Scheduled Permissions

Grant permissions that become active in the future:

```rust
use inferadb::{Relationship, Schedule};

// Permission that starts in the future (embargo period)
client.write(
    Relationship::new("doc:announcement", "viewer", "group:employees")
        .starts_at(Utc::now() + ChronoDuration::hours(2))
).await?;

// Before start time: permission denied
// After start time: permission granted
```

## Time-Windowed Access

Grant access only during specific time windows:

```rust
// Access only during business hours
client.write(
    Relationship::new("system:prod", "operator", "user:oncall")
        .schedule(Schedule::recurring()
            .weekdays()
            .hours(9, 17)  // 9 AM - 5 PM
            .timezone("America/New_York"))
).await?;

// Weekend maintenance window
client.write(
    Relationship::new("system:prod", "admin", "user:sre")
        .schedule(Schedule::recurring()
            .days([Saturday, Sunday])
            .hours(2, 6)  // 2 AM - 6 AM
            .timezone("UTC"))
).await?;
```

## Time-Aware Queries

Check permissions at specific points in time:

```rust
// Check permission at specific point in time
let was_allowed = client
    .check("user:alice", "view", "doc:1")
    .at_time(Utc::now() - ChronoDuration::hours(1))  // 1 hour ago
    .await?;

// Check if permission will be valid at future time
let will_be_allowed = client
    .check("user:alice", "view", "doc:1")
    .at_time(Utc::now() + ChronoDuration::days(7))  // 1 week from now
    .await?;
```

### Listing Relationships with Expiration

```rust
// List relationships with expiration info
let relationships = client
    .list_relationships()
    .resource("doc:sensitive")
    .include_expiration()
    .collect()
    .await?;

for rel in relationships {
    if let Some(expires) = rel.expires_at() {
        println!("{} -> {} expires at {}", rel.subject(), rel.relation(), expires);
    }
}

// Find expiring relationships (for notification/renewal workflows)
let expiring_soon = client
    .list_relationships()
    .expires_within(Duration::from_days(7))
    .collect()
    .await?;
```

## Schema-Level Temporal Rules

Define time-based constraints in your IPL schema:

```ipl
entity Document {
    attributes {
        embargo_until: Timestamp
        archive_after: Timestamp
    }

    relations {
        viewer: User | Group#member
        editor: User
    }

    permissions {
        // Can only view if not embargoed
        view: viewer & @now >= @embargo_until

        // Cannot edit after archived
        edit: editor & @now < @archive_after
    }
}

entity Subscription {
    attributes {
        expires_at: Timestamp
    }

    relations {
        subscriber: User
    }

    permissions {
        // Access only while subscription is valid
        access: subscriber & @now < @expires_at
    }
}
```

## Common Patterns

### Temporary Collaborator Access

```rust
async fn grant_collaborator_access(
    client: &Client,
    document_id: &str,
    collaborator: &str,
    duration: Duration,
) -> Result<(), Error> {
    client.write(
        Relationship::new(
            &format!("document:{}", document_id),
            "collaborator",
            collaborator
        )
        .expires_in(duration)
    ).await?;

    // Optionally notify when nearing expiration
    schedule_expiration_reminder(document_id, collaborator, duration).await;

    Ok(())
}
```

### Trial Period Access

```rust
async fn grant_trial_access(
    client: &Client,
    user: &str,
    trial_days: u64,
) -> Result<(), Error> {
    let expires = Utc::now() + ChronoDuration::days(trial_days as i64);

    client.write(
        Relationship::new("subscription:trial", "subscriber", user)
            .expires_at(expires)
    ).await?;

    Ok(())
}

async fn check_trial_active(client: &Client, user: &str) -> Result<bool, Error> {
    client.check(user, "access", "subscription:trial").await
}
```

### On-Call Rotation

```rust
async fn setup_oncall_rotation(
    client: &Client,
    user: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<(), Error> {
    client.write(
        Relationship::new("system:production", "oncall", user)
            .starts_at(start)
            .expires_at(end)
    ).await?;

    Ok(())
}
```

### Access Request Workflow

```rust
async fn approve_access_request(
    client: &Client,
    request: AccessRequest,
    approved_duration: Duration,
) -> Result<(), Error> {
    client.write(
        Relationship::new(&request.resource, &request.relation, &request.subject)
            .expires_in(approved_duration)
            .with_audit(AuditContext::new()
                .action_reason(format!("Access request #{} approved", request.id))
                .custom("approver", &request.approved_by))
    ).await?;

    Ok(())
}
```

## Type Reference

```rust
/// Expiration configuration for relationships
pub enum Expiration {
    /// Never expires
    Never,
    /// Expires at specific timestamp
    At(DateTime<Utc>),
    /// Expires after duration from creation
    After(Duration),
}

/// Recurring schedule for time-windowed access
pub struct Schedule {
    pub days: DaySet,
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: Tz,
}

impl Schedule {
    pub fn recurring() -> ScheduleBuilder;
    pub fn is_active(&self, at: DateTime<Utc>) -> bool;
    pub fn next_active_window(&self, from: DateTime<Utc>) -> Option<(DateTime<Utc>, DateTime<Utc>)>;
}

/// Extended relationship with temporal metadata
pub struct TemporalRelationship {
    pub subject: String,
    pub relation: String,
    pub resource: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub starts_at: Option<DateTime<Utc>>,
    pub schedule: Option<Schedule>,
}
```

## Best Practices

### 1. Set Reasonable Default Expirations

```rust
// Don't grant indefinite access when temporary is appropriate
// Bad
client.write(Relationship::new("doc:sensitive", "viewer", "user:contractor")).await?;

// Good
client.write(
    Relationship::new("doc:sensitive", "viewer", "user:contractor")
        .expires_in(Duration::from_days(90))
).await?;
```

### 2. Use Schema Attributes for Business Logic

```ipl
// Embed time constraints in schema, not just relationships
entity Document {
    attributes {
        review_deadline: Timestamp
    }

    permissions {
        submit_review: reviewer & @now < @review_deadline
    }
}
```

### 3. Monitor Expiring Permissions

```rust
// Set up alerts for critical expiring permissions
let expiring = client
    .list_relationships()
    .resource_type("system")
    .relation("admin")
    .expires_within(Duration::from_days(7))
    .collect()
    .await?;

for rel in expiring {
    alert_admin_expiring(&rel);
}
```

### 4. Audit Historical Access

```rust
// For compliance: check what access existed at incident time
let incident_time = parse_timestamp("2024-01-15T14:30:00Z")?;

let had_access = client
    .check("user:suspect", "edit", "doc:evidence")
    .at_time(incident_time)
    .await?;
```

### 5. Handle Timezone Correctly

```rust
// Always specify timezone for scheduled access
// Bad: ambiguous
.hours(9, 17)

// Good: explicit timezone
.hours(9, 17)
.timezone("America/New_York")
```
