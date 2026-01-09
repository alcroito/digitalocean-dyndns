# Hetzner Cloud API Migration Design

**Date:** 2026-01-08  
**Status:** Approved for implementation

## Overview

This document describes the migration from the deprecated Hetzner DNS API (`dns.hetzner.com`) to the new Hetzner Cloud API (`api.hetzner.cloud`) for DNS zone and record management.

## Motivation

The old Hetzner DNS API is deprecated. The new Hetzner Cloud API provides DNS management functionality using a modern RRSet-based approach that groups records by name and type.

## Key Differences

### Old API (dns.hetzner.com)
- Individual record operations
- Records have zone_id fields
- Update individual records by ID
- API: `https://dns.hetzner.com/api/v1`
- Auth: `Auth-API-Token` header

### New API (Hetzner Cloud)
- **RRSet-based operations** (record sets grouped by name + type)
- Records are grouped into RRSets (e.g., `www/A` contains all A records for www)
- Operations work on entire RRSets, not individual records
- API: `https://api.hetzner.cloud/v1`
- Auth: Bearer token (standard HTTP Bearer authentication)

### Architectural Changes

1. **RRSet concept**: Instead of updating record ID `123`, you update the RRSet identified by `zone_name/record_name/record_type` (e.g., `example.com/www/A`)

2. **Zone lookup first**: Must find zone ID by name, then work with RRSets within that zone

3. **Set-based updates**: Use `/zones/{zone}/rrsets/{name}/{type}/actions/set_records` to replace all records in an RRSet with new values

4. **Integer IDs**: Zone IDs are integers (not strings), RRSet IDs are composite strings like `www/A`

## Data Structures

### Core Types

```rust
// Response types
#[derive(Deserialize, Debug)]
struct HetznerCloudZonesResponse {
    zones: Vec<HetznerZone>,
}

#[derive(Deserialize, Debug)]
struct HetznerZone {
    id: i64,           // Integer ID in new API
    name: String,      // e.g., "example.com"
    created: String,
    mode: String,      // "primary" or "secondary"
}

#[derive(Deserialize, Debug)]
struct HetznerRRSetsResponse {
    rrsets: Vec<HetznerRRSet>,
}

#[derive(Deserialize, Debug)]
struct HetznerRRSet {
    id: String,           // Composite: "www/A"
    name: String,         // e.g., "www" or "@" for apex
    #[serde(rename = "type")]
    record_type: String,  // "A", "AAAA", etc.
    ttl: Option<i64>,     // Nullable in API
    records: Vec<HetznerRecord>,
    zone: i64,            // Zone ID reference
}

#[derive(Deserialize, Debug)]
struct HetznerRecord {
    value: String,        // The IP address or record value
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
}

// Request types
#[derive(Serialize, Debug)]
struct SetRecordsRequest {
    records: Vec<HetznerRecordInput>,
}

#[derive(Serialize, Debug)]
struct HetznerRecordInput {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
}
```

### Conversion to Common Types

```rust
impl From<HetznerRRSet> for crate::types::DomainRecordCommon {
    fn from(rrset: HetznerRRSet) -> Self {
        // Use RRSet's composite ID
        // Extract first record value (typical for A/AAAA records)
        let ip_value = rrset.records
            .first()
            .map(|r| r.value.clone())
            .unwrap_or_default();
        
        Self {
            id: rrset.id,
            record_type: rrset.record_type,
            name: rrset.name,
            ip_value,
        }
    }
}
```

## Implementation

### API Structure

```rust
pub struct HetznerApi {
    request_client: Client,
    token: SecretHetznerToken,
    zone_cache: std::sync::RwLock<HashMap<String, i64>>, // Cache zone name -> ID
}
```

### Key Methods

**Zone lookup with caching:**
- Check cache first for zone name → ID mapping
- Query API with name filter: `GET /zones?name={zone_name}`
- Cache result for future lookups

**Get domain records:**
- Find zone ID by name
- Query: `GET /zones/{zone_id}/rrsets`
- Convert RRSets to common format

**Update domain IP:**
- Find zone ID by name
- Parse composite record ID (e.g., `www/A`)
- POST to: `/zones/{zone_id}/rrsets/{name}/{type}/actions/set_records`
- Payload: `{"records": [{"value": "new.ip.address"}]}`

### API Endpoints

- Base: `https://api.hetzner.cloud/v1`
- List zones: `GET /zones?name={zone_name}`
- List RRSets: `GET /zones/{zone_id}/rrsets`
- Update RRSet: `POST /zones/{zone_id}/rrsets/{name}/{type}/actions/set_records`

### Authentication

- Use Bearer token: `.bearer_auth(token)`
- Token must be Hetzner Cloud API token (not DNS API token)

## Error Handling

### API Error Response

```rust
#[derive(Deserialize, Debug)]
struct HetznerErrorResponse {
    error: HetznerError,
}

#[derive(Deserialize, Debug)]
struct HetznerError {
    code: String,
    message: String,
}
```

### Error Scenarios

1. **incorrect_zone_mode**: Zone is in secondary mode (not editable)
2. **not_found**: Zone or RRSet doesn't exist
3. **Rate limiting**: Handled by existing retry logic
4. **Authentication errors**: Invalid or expired token

## Edge Cases

### 1. Zone Not Found
- User misconfigured domain name
- Clear error directing user to create zone in Hetzner Cloud

### 2. RRSet Doesn't Exist
- On first update, RRSet might not exist
- Use `set_records` action which creates if needed

### 3. Multiple Records in RRSet
- RRSets can contain multiple records
- Our use case: single dynamic IP
- `set_records` replaces all records with our single IP

### 4. Zone in Secondary Mode
- Check for `incorrect_zone_mode` error
- Return clear error to user

### 5. TTL Handling
- RRSet TTL is nullable (uses zone default)
- Don't set TTL in requests, let API use existing/default

### 6. Hostname Normalization
- API requires lowercase names
- Apex domain uses `@` notation
- Normalize before building URLs

```rust
fn normalize_hostname(hostname: &str, domain: &str) -> String {
    let normalized = hostname.to_lowercase();
    if normalized.is_empty() || normalized == domain {
        "@".to_string()
    } else {
        normalized
    }
}
```

## Configuration Migration

Users upgrading will need to:

1. Generate a new **Hetzner Cloud API token** (not DNS API token)
2. Ensure their zones exist in Hetzner Cloud DNS
3. Update config with new token

The `hetzner_token` configuration field will now accept Cloud API tokens.

## Testing Strategy

1. Unit tests for type conversions
2. Integration tests with mock API responses
3. Manual testing with real Hetzner Cloud account
4. Verify both IPv4 (A) and IPv6 (AAAA) record updates

## Implementation Tasks

1. Rewrite Hetzner API types (response/request structs)
2. Implement zone caching with RwLock
3. Implement zone lookup by name
4. Implement get_domain_records with RRSet fetching
5. Implement update_domain_ip with set_records action
6. Add error handling for Hetzner-specific errors
7. Add hostname normalization
8. Update tests
9. Update documentation

## Rollout Plan

- This is a breaking change for Hetzner users
- Old DNS API credentials will not work
- Consider version bump or migration guide
