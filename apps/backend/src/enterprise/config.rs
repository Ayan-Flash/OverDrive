use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub app_name: String,
    pub logo_url: String,
    pub primary_color: String,
    pub secondary_color: String,
    pub custom_css: String,
    pub show_branding: bool,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            app_name: "OverDrive".into(),
            logo_url: String::new(),
            primary_color: "#f04747".into(),
            secondary_color: "#1a1a2e".into(),
            custom_css: String::new(),
            show_branding: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CentralizedServerConfig {
    pub enabled: bool,
    pub server_url: String,
    pub api_key: String,
    pub sync_interval_secs: u64,
    pub auto_register: bool,
    pub machine_name: String,
    pub sync_policies: bool,
    pub sync_reports: bool,
    pub sync_profiles: bool,
}

impl Default for CentralizedServerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: String::new(),
            api_key: String::new(),
            sync_interval_secs: 300,
            auto_register: true,
            machine_name: hostname(),
            sync_policies: true,
            sync_reports: false,
            sync_profiles: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseConfig {
    pub branding: BrandingConfig,
    pub centralized: CentralizedServerConfig,
    pub policies_enabled: bool,
    pub enforcement_level: EnforcementLevel,
}

impl Default for EnterpriseConfig {
    fn default() -> Self {
        Self {
            branding: BrandingConfig::default(),
            centralized: CentralizedServerConfig::default(),
            policies_enabled: true,
            enforcement_level: EnforcementLevel::Recommended,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnforcementLevel {
    Recommended,
    Enforced,
    Strict,
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".into())
}
