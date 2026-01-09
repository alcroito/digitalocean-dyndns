//! Integration tests for provider configuration parsing and validation
//!
//! Tests the new `[[providers]]` configuration format, backwards compatibility
//! with legacy token configuration, and provider filtering logic.

use do_ddns::config::app_config::DomainRecord;
use do_ddns::config::provider_config::{ProviderType, ProvidersConfig};
use figment::providers::{Format, Toml};
use figment::Figment;
use secrecy::ExposeSecret;

// ============================================================================
// Configuration Loading Tests
// ============================================================================

#[test]
fn test_new_provider_config_parsing_single_provider() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test_token_123"
    "#;

    let figment = Figment::from(Toml::string(toml));
    let config: ProvidersConfig = figment.extract().unwrap();

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].provider, ProviderType::DigitalOcean);
    assert_eq!(
        config.providers[0].token.expose_secret().as_str(),
        "test_token_123"
    );
}

#[test]
fn test_new_provider_config_parsing_multiple_providers() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token_abc123"
        
        [[providers]]
        provider = "hetzner"
        token = "hz_token_xyz789"
    "#;

    let figment = Figment::from(Toml::string(toml));
    let config: ProvidersConfig = figment.extract().unwrap();

    assert_eq!(config.providers.len(), 2);

    // Verify first provider (DigitalOcean)
    assert_eq!(config.providers[0].provider, ProviderType::DigitalOcean);
    assert_eq!(
        config.providers[0].token.expose_secret().as_str(),
        "do_token_abc123"
    );

    // Verify second provider (Hetzner)
    assert_eq!(config.providers[1].provider, ProviderType::Hetzner);
    assert_eq!(
        config.providers[1].token.expose_secret().as_str(),
        "hz_token_xyz789"
    );
}

#[test]
fn test_new_provider_config_with_options() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test_token"
        
        [providers.options]
        timeout = 30
        rate_limit = 100
    "#;

    let figment = Figment::from(Toml::string(toml));
    let config: ProvidersConfig = figment.extract().unwrap();

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].options.len(), 2);
    assert_eq!(
        config.providers[0].options["timeout"],
        serde_json::json!(30)
    );
    assert_eq!(
        config.providers[0].options["rate_limit"],
        serde_json::json!(100)
    );
}

#[test]
fn test_provider_config_digitalocean_alias() {
    // Test that "digital_ocean" alias works
    let toml = r#"
        [[providers]]
        provider = "digital_ocean"
        token = "test_token"
    "#;

    let figment = Figment::from(Toml::string(toml));
    let config: ProvidersConfig = figment.extract().unwrap();

    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].provider, ProviderType::DigitalOcean);
}

#[test]
fn test_empty_providers_config() {
    let toml = r#"
        # Empty config
    "#;

    let figment = Figment::from(Toml::string(toml));
    let config: ProvidersConfig = figment.extract().unwrap();

    assert_eq!(config.providers.len(), 0);
}

// ============================================================================
// Provider Filtering Tests (Domain Records)
// ============================================================================

#[test]
fn test_domain_record_no_providers_field() {
    let toml = r#"
        type = "A"
        name = "home"
    "#;

    let record: DomainRecord = toml::from_str(toml).unwrap();

    // Should update on all providers when providers field is absent
    assert!(record.should_update_on(ProviderType::DigitalOcean));
    assert!(record.should_update_on(ProviderType::Hetzner));
    assert_eq!(record.record_type, "A");
    assert_eq!(record.name, "home");
}

#[test]
fn test_domain_record_empty_providers_array() {
    let toml = r#"
        type = "A"
        name = "home"
        providers = []
    "#;

    let record: DomainRecord = toml::from_str(toml).unwrap();

    // Should update on all providers when providers array is empty
    assert!(record.should_update_on(ProviderType::DigitalOcean));
    assert!(record.should_update_on(ProviderType::Hetzner));
}

#[test]
fn test_domain_record_single_provider() {
    let toml = r#"
        type = "A"
        name = "backup"
        providers = ["hetzner"]
    "#;

    let record: DomainRecord = toml::from_str(toml).unwrap();

    // Should only update on Hetzner
    assert!(!record.should_update_on(ProviderType::DigitalOcean));
    assert!(record.should_update_on(ProviderType::Hetzner));
}

#[test]
fn test_domain_record_multiple_providers() {
    let toml = r#"
        type = "A"
        name = "cdn"
        providers = ["digitalocean", "hetzner"]
    "#;

    let record: DomainRecord = toml::from_str(toml).unwrap();

    // Should update on both providers
    assert!(record.should_update_on(ProviderType::DigitalOcean));
    assert!(record.should_update_on(ProviderType::Hetzner));
}

#[test]
fn test_domain_record_digitalocean_only() {
    let toml = r#"
        type = "A"
        name = "test"
        providers = ["digitalocean"]
    "#;

    let record: DomainRecord = toml::from_str(toml).unwrap();

    // Should only update on DigitalOcean
    assert!(record.should_update_on(ProviderType::DigitalOcean));
    assert!(!record.should_update_on(ProviderType::Hetzner));
}

#[test]
fn test_domain_record_invalid_provider_name() {
    let toml = r#"
        type = "A"
        name = "test"
        providers = ["invalid_provider"]
    "#;

    // Should fail parsing due to invalid provider name
    let result: Result<DomainRecord, _> = toml::from_str(toml);
    assert!(result.is_err());
}

#[test]
fn test_domain_record_mixed_valid_and_invalid_providers() {
    let toml = r#"
        type = "A"
        name = "test"
        providers = ["digitalocean", "cloudflare"]
    "#;

    // Should fail parsing because cloudflare is not a valid provider (yet)
    let result: Result<DomainRecord, _> = toml::from_str(toml);
    assert!(result.is_err());
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn test_providers_config_validate_empty_fails() {
    let config = ProvidersConfig::default();
    let result = config.validate();

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("At least one DNS provider must be configured"));
}

#[test]
fn test_providers_config_validate_with_provider_succeeds() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test_token"
    "#;

    let config: ProvidersConfig = toml::from_str(toml).unwrap();
    let result = config.validate();

    assert!(result.is_ok());
}

#[test]
fn test_provider_config_missing_token_field() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
    "#;

    let result: Result<ProvidersConfig, _> = toml::from_str(toml);
    assert!(result.is_err());
}

#[test]
fn test_provider_config_missing_provider_field() {
    let toml = r#"
        [[providers]]
        token = "test_token"
    "#;

    let result: Result<ProvidersConfig, _> = toml::from_str(toml);
    assert!(result.is_err());
}

#[test]
fn test_provider_config_invalid_provider_type() {
    let toml = r#"
        [[providers]]
        provider = "cloudflare"
        token = "test_token"
    "#;

    // Should fail because cloudflare is not a valid ProviderType (yet)
    let result: Result<ProvidersConfig, _> = toml::from_str(toml);
    assert!(result.is_err());
}

#[test]
fn test_provider_config_empty_token() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = ""
    "#;

    // Should succeed - empty token validation is handled elsewhere
    let result: Result<ProvidersConfig, _> = toml::from_str(toml);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().providers[0].token.expose_secret().as_str(),
        ""
    );
}

// ============================================================================
// Backwards Compatibility Tests
// ============================================================================

#[test]
fn test_legacy_digital_ocean_token_config() {
    // Test that legacy digital_ocean_token still works
    let toml = r#"
        update_interval = "10m"
        digital_ocean_token = "legacy_do_token_123"
        log_level = 0
        dry_run = false
        ipv4 = true
        ipv6 = false
        collect_stats = false
        enable_web = false
        listen_hostname = "localhost"
        listen_port = 8095
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "www"
    "#;

    #[derive(serde::Deserialize)]
    struct LegacyConfig {
        digital_ocean_token: Option<do_ddns::digital_ocean_token::SecretDigitalOceanToken>,
        #[serde(default)]
        providers_config: ProvidersConfig,
    }

    let config: LegacyConfig = toml::from_str(toml).unwrap();

    // Legacy token should be present
    assert!(config.digital_ocean_token.is_some());

    // New providers config should be empty
    assert_eq!(config.providers_config.providers.len(), 0);
}

#[test]
fn test_legacy_digitalocean_token_config() {
    // Test that legacy digital_ocean_token still works
    let toml = r#"
        update_interval = "10m"
        digital_ocean_token = "legacy_do_token_123"
        log_level = 0
        dry_run = false
        ipv4 = true
        ipv6 = false
        collect_stats = false
        enable_web = false
        listen_hostname = "localhost"
        listen_port = 8095
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "www"
    "#;

    #[derive(serde::Deserialize)]
    struct LegacyConfig {
        digital_ocean_token: Option<do_ddns::digital_ocean_token::SecretDigitalOceanToken>,
        #[serde(default)]
        providers_config: ProvidersConfig,
    }

    let config: LegacyConfig = toml::from_str(toml).unwrap();

    // Legacy token should be present
    assert!(config.digital_ocean_token.is_some());

    // New providers config should be empty
    assert_eq!(config.providers_config.providers.len(), 0);
}

#[test]
fn test_new_config_coexists_with_legacy() {
    // Test that new providers config can coexist with legacy (new takes precedence)
    let toml = r#"
        update_interval = "10m"
        digital_ocean_token = "legacy_do_token"
        log_level = 0
        dry_run = false
        ipv4 = true
        ipv6 = false
        collect_stats = false
        enable_web = false
        listen_hostname = "localhost"
        listen_port = 8095
        
        [[providers]]
        provider = "digitalocean"
        token = "new_do_token"
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "www"
    "#;

    #[derive(serde::Deserialize)]
    struct MixedConfig {
        digital_ocean_token: Option<do_ddns::digital_ocean_token::SecretDigitalOceanToken>,
        #[serde(default)]
        providers: Vec<do_ddns::config::provider_config::ProviderConfig>,
    }

    let config: MixedConfig = toml::from_str(toml).unwrap();

    // Both should be present
    assert!(config.digital_ocean_token.is_some());
    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].provider, ProviderType::DigitalOcean);
}

#[test]
fn test_migration_from_legacy_to_new_format() {
    // Demonstrate migration path: same domain config works with both formats

    let legacy_toml = r#"
        digital_ocean_token = "token123"
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "www"
    "#;

    let new_toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "token123"
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "www"
    "#;

    // Both should parse successfully
    #[derive(serde::Deserialize)]
    struct LegacyConfig {
        digital_ocean_token: Option<do_ddns::digital_ocean_token::SecretDigitalOceanToken>,
    }

    #[derive(serde::Deserialize)]
    struct NewConfig {
        providers: Vec<do_ddns::config::provider_config::ProviderConfig>,
    }

    let legacy_config: LegacyConfig = toml::from_str(legacy_toml).unwrap();
    assert!(legacy_config.digital_ocean_token.is_some());

    let new_config: NewConfig = toml::from_str(new_toml).unwrap();
    assert_eq!(new_config.providers.len(), 1);
    assert_eq!(new_config.providers[0].provider, ProviderType::DigitalOcean);
}

// ============================================================================
// Helper Methods Tests
// ============================================================================

#[test]
fn test_providers_config_get_method() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token"
        
        [[providers]]
        provider = "hetzner"
        token = "hz_token"
    "#;

    let config: ProvidersConfig = toml::from_str(toml).unwrap();

    // Test get method
    let do_provider = config.get(ProviderType::DigitalOcean);
    assert!(do_provider.is_some());
    assert_eq!(do_provider.unwrap().provider, ProviderType::DigitalOcean);

    let hz_provider = config.get(ProviderType::Hetzner);
    assert!(hz_provider.is_some());
    assert_eq!(hz_provider.unwrap().provider, ProviderType::Hetzner);
}

#[test]
fn test_providers_config_has_provider_method() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token"
    "#;

    let config: ProvidersConfig = toml::from_str(toml).unwrap();

    assert!(config.has_provider(ProviderType::DigitalOcean));
    assert!(!config.has_provider(ProviderType::Hetzner));
}

#[test]
fn test_providers_config_provider_types_method() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token"
        
        [[providers]]
        provider = "hetzner"
        token = "hz_token"
    "#;

    let config: ProvidersConfig = toml::from_str(toml).unwrap();
    let types = config.provider_types();

    assert_eq!(types.len(), 2);
    assert!(types.contains(&ProviderType::DigitalOcean));
    assert!(types.contains(&ProviderType::Hetzner));
}

// ============================================================================
// Complex Configuration Tests
// ============================================================================

#[test]
fn test_full_config_with_domains_and_providers() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token_123"
        
        [[providers]]
        provider = "hetzner"
        token = "hz_token_456"
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "home"
        # No providers - updates on all
        
        [[domains.records]]
        type = "A"
        name = "backup"
        providers = ["hetzner"]
        
        [[domains.records]]
        type = "A"
        name = "cdn"
        providers = ["digitalocean", "hetzner"]
    "#;

    // Parse providers config - we need to wrap it in a struct
    #[derive(serde::Deserialize)]
    struct TestConfigWithProviders {
        #[serde(default)]
        providers: Vec<do_ddns::config::provider_config::ProviderConfig>,
    }

    let figment = Figment::from(Toml::string(toml));
    let config_with_providers: TestConfigWithProviders = figment.extract().unwrap();
    let providers_config = ProvidersConfig {
        providers: config_with_providers.providers,
    };

    assert_eq!(providers_config.providers.len(), 2);
    assert!(providers_config.has_provider(ProviderType::DigitalOcean));
    assert!(providers_config.has_provider(ProviderType::Hetzner));

    // Parse domain records manually to test filtering
    #[derive(serde::Deserialize)]
    struct TestDomain {
        #[allow(dead_code)]
        name: String,
        records: Vec<DomainRecord>,
    }

    #[derive(serde::Deserialize)]
    struct TestConfig {
        domains: Vec<TestDomain>,
    }

    let config: TestConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.domains.len(), 1);
    assert_eq!(config.domains[0].records.len(), 3);

    // Test first record (home) - should update on all providers
    let home_record = &config.domains[0].records[0];
    assert_eq!(home_record.name, "home");
    assert!(home_record.should_update_on(ProviderType::DigitalOcean));
    assert!(home_record.should_update_on(ProviderType::Hetzner));

    // Test second record (backup) - should only update on Hetzner
    let backup_record = &config.domains[0].records[1];
    assert_eq!(backup_record.name, "backup");
    assert!(!backup_record.should_update_on(ProviderType::DigitalOcean));
    assert!(backup_record.should_update_on(ProviderType::Hetzner));

    // Test third record (cdn) - should update on both providers
    let cdn_record = &config.domains[0].records[2];
    assert_eq!(cdn_record.name, "cdn");
    assert!(cdn_record.should_update_on(ProviderType::DigitalOcean));
    assert!(cdn_record.should_update_on(ProviderType::Hetzner));
}

#[test]
fn test_multiple_domains_with_provider_filtering() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token"
        
        [[providers]]
        provider = "hetzner"
        token = "hz_token"
        
        [[domains]]
        name = "example.com"
        
        [[domains.records]]
        type = "A"
        name = "www"
        providers = ["digitalocean"]
        
        [[domains]]
        name = "example.org"
        
        [[domains.records]]
        type = "A"
        name = "www"
        providers = ["hetzner"]
    "#;

    #[derive(serde::Deserialize)]
    struct TestDomain {
        name: String,
        records: Vec<DomainRecord>,
    }

    #[derive(serde::Deserialize)]
    struct TestConfig {
        providers: Vec<do_ddns::config::provider_config::ProviderConfig>,
        domains: Vec<TestDomain>,
    }

    let config: TestConfig = toml::from_str(toml).unwrap();

    assert_eq!(config.providers.len(), 2);
    assert_eq!(config.domains.len(), 2);

    // First domain (example.com) should only update on DigitalOcean
    let example_com_record = &config.domains[0].records[0];
    assert_eq!(config.domains[0].name, "example.com");
    assert!(example_com_record.should_update_on(ProviderType::DigitalOcean));
    assert!(!example_com_record.should_update_on(ProviderType::Hetzner));

    // Second domain (example.org) should only update on Hetzner
    let example_org_record = &config.domains[1].records[0];
    assert_eq!(config.domains[1].name, "example.org");
    assert!(!example_org_record.should_update_on(ProviderType::DigitalOcean));
    assert!(example_org_record.should_update_on(ProviderType::Hetzner));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_duplicate_provider_types() {
    // Test that we can have duplicate provider types (though not recommended)
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "token1"
        
        [[providers]]
        provider = "digitalocean"
        token = "token2"
    "#;

    let config: ProvidersConfig = toml::from_str(toml).unwrap();

    // Should parse successfully, but get() will return the first one
    assert_eq!(config.providers.len(), 2);
    let provider = config.get(ProviderType::DigitalOcean).unwrap();
    assert_eq!(provider.token.expose_secret().as_str(), "token1");
}

#[test]
fn test_provider_config_case_sensitivity() {
    // Test that provider names are case-insensitive (due to lowercase serde)
    let toml_lower = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test"
    "#;

    let toml_upper = r#"
        [[providers]]
        provider = "DIGITALOCEAN"
    "#;

    let config_lower: Result<ProvidersConfig, _> = toml::from_str(toml_lower);
    assert!(config_lower.is_ok());

    // Uppercase should fail due to serde lowercase rename
    let config_upper: Result<ProvidersConfig, _> = toml::from_str(toml_upper);
    assert!(config_upper.is_err());
}

#[test]
fn test_domain_record_with_ipv6_type() {
    let toml = r#"
        type = "AAAA"
        name = "ipv6"
        providers = ["digitalocean"]
    "#;

    let record: DomainRecord = toml::from_str(toml).unwrap();

    assert_eq!(record.record_type, "AAAA");
    assert_eq!(record.name, "ipv6");
    assert!(record.should_update_on(ProviderType::DigitalOcean));
    assert!(!record.should_update_on(ProviderType::Hetzner));
}

#[test]
fn test_provider_options_with_nested_structure() {
    let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test_token"
        
        [providers.options]
        api = { timeout = 30, retries = 3 }
        cache = { enabled = true, ttl = 300 }
    "#;

    let config: ProvidersConfig = toml::from_str(toml).unwrap();

    assert_eq!(config.providers[0].options.len(), 2);
    assert!(config.providers[0].options.contains_key("api"));
    assert!(config.providers[0].options.contains_key("cache"));
}

#[test]
fn test_figment_extraction_from_nested_config() {
    let toml = r#"
        [general]
        dry_run = true
        
        [[providers]]
        provider = "digitalocean"
        token = "test_token"
    "#;

    let figment = Figment::from(Toml::string(toml));

    // Extract the full config and check providers array
    #[derive(serde::Deserialize)]
    struct TestConfig {
        #[serde(default)]
        providers: Vec<do_ddns::config::provider_config::ProviderConfig>,
    }

    let config: TestConfig = figment.extract().unwrap();
    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].provider, ProviderType::DigitalOcean);
}

// ============================================================================
// Provider Type Utility Tests
// ============================================================================

#[test]
fn test_provider_type_as_str() {
    assert_eq!(ProviderType::DigitalOcean.as_str(), "digitalocean");
    assert_eq!(ProviderType::Hetzner.as_str(), "hetzner");
}

#[test]
fn test_provider_type_equality() {
    assert_eq!(ProviderType::DigitalOcean, ProviderType::DigitalOcean);
    assert_ne!(ProviderType::DigitalOcean, ProviderType::Hetzner);
}

#[test]
fn test_provider_type_hash() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(ProviderType::DigitalOcean);
    set.insert(ProviderType::Hetzner);
    set.insert(ProviderType::DigitalOcean); // Duplicate

    assert_eq!(set.len(), 2);
}
