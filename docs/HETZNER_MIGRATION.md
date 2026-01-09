# Hetzner Cloud API Migration Guide

## Breaking Changes for Hetzner Users

The Hetzner DNS integration has been migrated from the deprecated `dns.hetzner.com` API to the new Hetzner Cloud API.

### What You Need to Do

1. **Generate a Hetzner Cloud API Token:**
   - Log into [Hetzner Cloud Console](https://console.hetzner.cloud/)
   - Navigate to your project → Security → API Tokens
   - Create a new API token with Read & Write permissions
   - **Important:** This is different from the old DNS API token

2. **Ensure Your Zones Exist in Hetzner Cloud:**
   - Your DNS zones must be created in Hetzner Cloud DNS
   - Zones must be in "primary" mode (not "secondary")

3. **Update Your Configuration:**
   - Replace the old DNS API token with your new Cloud API token
   - The `hetzner_token` field now expects a Hetzner Cloud API token

### Technical Changes

- **API Endpoint:** `https://dns.hetzner.com/api/v1` → `https://api.hetzner.cloud/v1`
- **Authentication:** `Auth-API-Token` header → Bearer token
- **Record Model:** Individual records → RRSets (grouped by name + type)
- **Provider Name:** "Hetzner" → "Hetzner Cloud" in logs

### Troubleshooting

**"Zone not found" error:**
- Ensure the zone exists in Hetzner Cloud Console
- Check that the domain name matches exactly

**"Zone is in secondary mode" error:**
- Primary zones can be managed via API
- Secondary zones cannot be updated programmatically

**"Invalid token" error:**
- Ensure you're using a Hetzner Cloud API token
- Old DNS API tokens will not work
