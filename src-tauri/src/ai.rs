use crate::error::AppError;
use crate::store::{Storable, Store};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use rig::prelude::*;
use rig::providers::{anthropic, deepseek, gemini, groq, openai, openrouter, xai};
use rig::streaming::StreamingPrompt;
use rusqlite::{params, Result as RusqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};

const AI_KEYRING_SERVICE: &str = "dev.byteatatime.raycast.ai";
const AI_KEYRING_USERNAME: &str = "openrouter_api_key";
const AI_USAGE_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS ai_generations (
    id TEXT PRIMARY KEY,
    created INTEGER NOT NULL,
    model TEXT NOT NULL,
    tokens_prompt INTEGER NOT NULL,
    tokens_completion INTEGER NOT NULL,
    native_tokens_prompt INTEGER NOT NULL,
    native_tokens_completion INTEGER NOT NULL,
    total_cost REAL NOT NULL
)";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskOptions {
    pub model: Option<String>,
    pub creativity: Option<String>,
    pub provider: Option<String>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    request_id: String,
    text: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamEnd {
    request_id: String,
    full_text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerationData {
    pub id: String,
    pub created: i64,
    pub model: String,
    #[serde(default)]
    pub tokens_prompt: i64,
    #[serde(default)]
    pub tokens_completion: i64,
    #[serde(default)]
    pub native_tokens_prompt: i64,
    #[serde(default)]
    pub native_tokens_completion: i64,
    #[serde(default)]
    pub total_cost: f64,
}

impl Storable for GenerationData {
    fn from_row(row: &rusqlite::Row) -> RusqliteResult<Self> {
        Ok(GenerationData {
            id: row.get(0)?,
            created: row.get(1)?,
            model: row.get(2)?,
            tokens_prompt: row.get(3)?,
            tokens_completion: row.get(4)?,
            native_tokens_prompt: row.get(5)?,
            native_tokens_completion: row.get(6)?,
            total_cost: row.get(7)?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f64,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    pub enabled: bool,
    pub base_url: Option<String>, // For custom endpoints
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: false,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AIProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Perplexity,
    OpenRouter,
    XAI,
    DeepSeek,
    Groq,
}

impl AIProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AIProvider::OpenAI => "openai",
            AIProvider::Anthropic => "anthropic",
            AIProvider::Gemini => "gemini",
            AIProvider::Perplexity => "perplexity",
            AIProvider::OpenRouter => "openrouter",
            AIProvider::XAI => "xai",
            AIProvider::DeepSeek => "deepseek",
            AIProvider::Groq => "groq",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(AIProvider::OpenAI),
            "anthropic" => Some(AIProvider::Anthropic),
            "gemini" => Some(AIProvider::Gemini),
            "perplexity" => Some(AIProvider::Perplexity),
            "openrouter" => Some(AIProvider::OpenRouter),
            "xai" => Some(AIProvider::XAI),
            "deepseek" => Some(AIProvider::DeepSeek),
            "groq" => Some(AIProvider::Groq),
            _ => None,
        }
    }

    pub fn default_models(&self) -> Vec<&'static str> {
        match self {
            AIProvider::OpenAI => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-4",
                "o1",
                "o1-mini",
                "o3-mini",
            ],
            AIProvider::Anthropic => vec![
                "claude-3-5-sonnet-20241022",
                "claude-3-5-haiku-20241022",
                "claude-3-opus-20240229",
                "claude-3-sonnet-20240229",
                "claude-3-haiku-20240307",
            ],
            AIProvider::Gemini => vec![
                "gemini-2.0-flash-exp",
                "gemini-1.5-pro",
                "gemini-1.5-flash",
                "gemini-1.5-flash-8b",
            ],
            AIProvider::Perplexity => vec![
                "llama-3.1-sonar-small-128k-online",
                "llama-3.1-sonar-large-128k-online",
                "llama-3.1-sonar-huge-128k-online",
            ],
            AIProvider::OpenRouter => vec![
                "openai/gpt-4o",
                "anthropic/claude-3-5-sonnet",
                "meta-llama/llama-3.1-405b-instruct",
                "google/gemini-pro-1.5",
                "mistralai/mistral-large",
            ],
            AIProvider::XAI => vec!["grok-beta", "grok-2-1212", "grok-2-vision-1212"],
            AIProvider::DeepSeek => vec!["deepseek-chat", "deepseek-coder", "deepseek-reasoner"],
            AIProvider::Groq => vec![
                "llama-3.1-70b-versatile",
                "llama-3.1-8b-instant",
                "mixtral-8x7b-32768",
                "gemma2-9b-it",
            ],
        }
    }

    pub fn supports_streaming(&self) -> bool {
        match self {
            AIProvider::OpenAI => true,
            AIProvider::Anthropic => true,
            AIProvider::Gemini => true,      // Now implemented in rig
            AIProvider::Perplexity => false, // Not implemented in rig yet
            AIProvider::OpenRouter => true,
            AIProvider::XAI => true,
            AIProvider::DeepSeek => true,
            AIProvider::Groq => true,
        }
    }

    pub fn supports_function_calling(&self) -> bool {
        match self {
            AIProvider::OpenAI => true,
            AIProvider::Anthropic => true,
            AIProvider::Gemini => true,
            AIProvider::Perplexity => false,
            AIProvider::OpenRouter => true, // Depends on model
            AIProvider::XAI => false,
            AIProvider::DeepSeek => true,
            AIProvider::Groq => true,
        }
    }

    pub fn keyring_service(&self) -> String {
        format!("dev.byteatatime.raycast.ai.{}", self.as_str())
    }

    pub fn keyring_username(&self) -> String {
        format!("{}_api_key", self.as_str())
    }
}

// Optimized model mappings organized by provider
static DEFAULT_AI_MODELS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // OpenAI Models
    m.insert("OpenAI_GPT4o", "openai/gpt-4o");
    m.insert("OpenAI_GPT4o_Mini", "openai/gpt-4o-mini");
    m.insert("OpenAI_GPT4_Turbo", "openai/gpt-4-turbo");
    m.insert("OpenAI_GPT4", "openai/gpt-4");
    m.insert("OpenAI_O1", "openai/o1");
    m.insert("OpenAI_O1_Mini", "openai/o1-mini");
    m.insert("OpenAI_O3_Mini", "openai/o3-mini");

    // Anthropic Models
    m.insert(
        "Anthropic_Claude_3_5_Sonnet",
        "anthropic/claude-3-5-sonnet-20241022",
    );
    m.insert(
        "Anthropic_Claude_3_5_Haiku",
        "anthropic/claude-3-5-haiku-20241022",
    );
    m.insert(
        "Anthropic_Claude_3_Opus",
        "anthropic/claude-3-opus-20240229",
    );
    m.insert(
        "Anthropic_Claude_3_Sonnet",
        "anthropic/claude-3-sonnet-20240229",
    );
    m.insert(
        "Anthropic_Claude_3_Haiku",
        "anthropic/claude-3-haiku-20240307",
    );

    // Google Models
    m.insert("Google_Gemini_2_0_Flash", "google/gemini-2.0-flash-exp");
    m.insert("Google_Gemini_1_5_Pro", "google/gemini-1.5-pro");
    m.insert("Google_Gemini_1_5_Flash", "google/gemini-1.5-flash");
    m.insert("Google_Gemini_1_5_Flash_8B", "google/gemini-1.5-flash-8b");

    // Perplexity Models
    m.insert(
        "Perplexity_Sonar_Small",
        "perplexity/llama-3.1-sonar-small-128k-online",
    );
    m.insert(
        "Perplexity_Sonar_Large",
        "perplexity/llama-3.1-sonar-large-128k-online",
    );
    m.insert(
        "Perplexity_Sonar_Huge",
        "perplexity/llama-3.1-sonar-huge-128k-online",
    );

    // Meta (via OpenRouter)
    m.insert("Meta_Llama_3_1_405B", "meta-llama/llama-3.1-405b-instruct");
    m.insert("Meta_Llama_3_1_70B", "meta-llama/llama-3.1-70b-instruct");
    m.insert("Meta_Llama_3_1_8B", "meta-llama/llama-3.1-8b-instruct");

    // Mistral (via OpenRouter)
    m.insert("Mistral_Large", "mistralai/mistral-large");
    m.insert("Mistral_Nemo", "mistralai/mistral-nemo");
    m.insert("Mistral_Codestral", "mistralai/codestral-latest");

    // DeepSeek Models
    m.insert("DeepSeek_Chat", "deepseek/deepseek-chat");
    m.insert("DeepSeek_Coder", "deepseek/deepseek-coder");
    m.insert("DeepSeek_Reasoner", "deepseek/deepseek-reasoner");

    // xAI Models
    m.insert("xAI_Grok_Beta", "x-ai/grok-beta");
    m.insert("xAI_Grok_2", "x-ai/grok-2-1212");
    m.insert("xAI_Grok_2_Vision", "x-ai/grok-2-vision-1212");

    // Groq Models
    m.insert("Groq_Llama_3_1_70B", "groq/llama-3.1-70b-versatile");
    m.insert("Groq_Llama_3_1_8B", "groq/llama-3.1-8b-instant");
    m.insert("Groq_Mixtral_8x7B", "groq/mixtral-8x7b-32768");
    m.insert("Groq_Gemma2_9B", "groq/gemma2-9b-it");

    m
});

/// Get default provider configurations with optimized settings
fn get_default_provider_configs() -> HashMap<String, ProviderConfig> {
    let mut configs = HashMap::new();

    // OpenAI configuration
    configs.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: true,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: true,
        },
    );

    // Anthropic configuration
    configs.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: true,
        },
    );

    // Gemini configuration
    configs.insert(
        "gemini".to_string(),
        ProviderConfig {
            api_key: None,
            model: "gemini-1.5-flash".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: false,
            supports_function_calling: true,
        },
    );

    // Perplexity configuration
    configs.insert(
        "perplexity".to_string(),
        ProviderConfig {
            api_key: None,
            model: "llama-3.1-sonar-small-128k-online".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: false,
            supports_function_calling: false,
        },
    );

    // OpenRouter configuration
    configs.insert(
        "openrouter".to_string(),
        ProviderConfig {
            api_key: None,
            model: "openai/gpt-4o-mini".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: true,
        },
    );

    // xAI configuration
    configs.insert(
        "xai".to_string(),
        ProviderConfig {
            api_key: None,
            model: "grok-beta".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: false,
        },
    );

    // DeepSeek configuration
    configs.insert(
        "deepseek".to_string(),
        ProviderConfig {
            api_key: None,
            model: "deepseek-chat".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: true,
        },
    );

    // Groq configuration
    configs.insert(
        "groq".to_string(),
        ProviderConfig {
            api_key: None,
            model: "llama-3.1-70b-versatile".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(4096),
            enabled: false,
            base_url: None,
            supports_streaming: true,
            supports_function_calling: true,
        },
    );

    configs
}

/// Unified keyring management
struct KeyringManager;

impl KeyringManager {
    fn get_entry_for_provider(provider: &AIProvider) -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(&provider.keyring_service(), &provider.keyring_username())
            .map_err(AppError::from)
    }

    fn get_legacy_entry() -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(AI_KEYRING_SERVICE, AI_KEYRING_USERNAME).map_err(AppError::from)
    }

    pub fn set_api_key(provider: &AIProvider, key: &str) -> Result<(), AppError> {
        let entry = Self::get_entry_for_provider(provider)?;
        entry.set_password(key).map_err(AppError::from)?;
        Ok(())
    }

    pub fn get_api_key(provider: &AIProvider) -> Result<String, AppError> {
        // For OpenRouter, try legacy key first for backward compatibility
        if provider == &AIProvider::OpenRouter {
            if let Ok(entry) = Self::get_legacy_entry() {
                if let Ok(key) = entry.get_password() {
                    return Ok(key);
                }
            }
        }

        let entry = Self::get_entry_for_provider(provider)?;
        entry.get_password().map_err(AppError::from)
    }

    pub fn is_api_key_set(provider: &AIProvider) -> bool {
        Self::get_api_key(provider).is_ok()
    }

    pub fn clear_api_key(provider: &AIProvider) -> Result<(), AppError> {
        let entry = Self::get_entry_for_provider(provider)?;
        entry.delete_credential().map_err(AppError::from)?;
        Ok(())
    }
}

/// Provider agent factory for creating streaming agents
struct ProviderAgentFactory;

impl ProviderAgentFactory {
    pub fn create_agent(
        provider: &AIProvider,
        config: &ProviderConfig,
        api_key: &str,
        options: &AskOptions,
    ) -> Result<Box<dyn rig::Agent>, String> {
        let temperature = options.temperature.unwrap_or(config.temperature);
        let max_tokens = options.max_tokens.or(config.max_tokens);

        match provider {
            AIProvider::OpenAI => {
                let client = openai::Client::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            AIProvider::DeepSeek => {
                let client = deepseek::Client::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            AIProvider::XAI => {
                let client = xai::Client::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            AIProvider::OpenRouter => {
                let client = openrouter::Client::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            AIProvider::Anthropic => {
                let client = anthropic::Anthropic::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            AIProvider::Gemini => {
                let client = gemini::Client::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            AIProvider::Groq => {
                let client = groq::Client::new(api_key);
                let model = options.model.as_ref().unwrap_or(&config.model);
                let mut agent_builder = client.agent(model);

                agent_builder = agent_builder.temperature(temperature);
                if let Some(top_p) = options.top_p.or(config.top_p) {
                    agent_builder = agent_builder.top_p(top_p);
                }
                if let Some(max_tokens) = max_tokens {
                    agent_builder = agent_builder.max_tokens(max_tokens as u64);
                }

                Ok(Box::new(agent_builder.build()))
            }
            _ => Err(format!(
                "Provider {:?} streaming not yet implemented",
                provider
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    enabled: bool,
    model_associations: HashMap<String, String>,
    providers: HashMap<String, ProviderConfig>,
    default_provider: String,
    global_temperature: Option<f64>,
    global_max_tokens: Option<u32>,
}

impl AiSettings {
    /// Validate settings configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate default provider exists
        if !self.providers.contains_key(&self.default_provider) {
            return Err(format!(
                "Default provider '{}' not found in providers",
                self.default_provider
            ));
        }

        // Validate provider configurations
        for (name, config) in &self.providers {
            if let Some(provider) = AIProvider::from_str(name) {
                if !provider.default_models().contains(&config.model.as_str()) {
                    eprintln!(
                        "Warning: Model '{}' might not be valid for provider '{}'",
                        config.model, name
                    );
                }
            }
        }

        // Validate temperature range
        if let Some(temp) = self.global_temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err("Global temperature must be between 0.0 and 2.0".to_string());
            }
        }

        Ok(())
    }

    /// Get enabled providers
    pub fn get_enabled_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

fn get_settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Failed to get app local data dir".to_string())?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    }
    Ok(data_dir.join("ai_settings.json"))
}

fn read_settings(path: &Path) -> Result<AiSettings, String> {
    if !path.exists() {
        return Ok(AiSettings::default());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if content.trim().is_empty() {
        return Ok(AiSettings::default());
    }
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn write_settings(path: &Path, settings: &AiSettings) -> Result<(), String> {
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ai_settings(app: tauri::AppHandle) -> Result<AiSettings, String> {
    let path = get_settings_path(&app)?;
    let mut user_settings = read_settings(&path)?;

    // Initialize default providers if not present
    if user_settings.providers.is_empty() {
        user_settings.providers = get_default_provider_configs();
    }

    // Set default provider if not set
    if user_settings.default_provider.is_empty() {
        user_settings.default_provider = "openai".to_string();
    }

    // Ensure all providers have default configs
    let default_configs = get_default_provider_configs();
    for (provider, default_config) in default_configs {
        user_settings
            .providers
            .entry(provider)
            .or_insert(default_config);
    }

    // Maintain backward compatibility with model associations
    for (key, &default_value) in DEFAULT_AI_MODELS.iter() {
        let entry = user_settings
            .model_associations
            .entry(key.to_string())
            .or_insert_with(|| default_value.to_string());

        if entry.is_empty() {
            *entry = default_value.to_string();
        }
    }

    // Validate settings
    user_settings.validate()?;

    Ok(user_settings)
}

#[tauri::command]
pub fn set_ai_settings(app: tauri::AppHandle, mut settings: AiSettings) -> Result<(), String> {
    let path = get_settings_path(&app)?;

    // Validate before saving
    settings.validate()?;

    let mut settings_to_save = AiSettings {
        enabled: settings.enabled,
        model_associations: HashMap::new(),
        providers: settings.providers.clone(),
        default_provider: settings.default_provider.clone(),
        global_temperature: settings.global_temperature,
        global_max_tokens: settings.global_max_tokens,
    };

    // Only save model associations that differ from defaults (backward compatibility)
    for (key, value) in settings.model_associations {
        let is_different_from_default = DEFAULT_AI_MODELS
            .get(key.as_str())
            .map_or(true, |&default_val| default_val != value);

        if is_different_from_default {
            settings_to_save.model_associations.insert(key, value);
        }
    }

    write_settings(&path, &settings_to_save)
}

#[tauri::command]
pub fn set_provider_api_key(provider: String, key: String) -> Result<(), String> {
    let ai_provider =
        AIProvider::from_str(&provider).ok_or_else(|| format!("Invalid provider: {}", provider))?;

    KeyringManager::set_api_key(&ai_provider, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_provider_api_key_set(provider: String) -> Result<bool, String> {
    let ai_provider =
        AIProvider::from_str(&provider).ok_or_else(|| format!("Invalid provider: {}", provider))?;

    Ok(KeyringManager::is_api_key_set(&ai_provider))
}

#[tauri::command]
pub fn clear_provider_api_key(provider: String) -> Result<(), String> {
    let ai_provider =
        AIProvider::from_str(&provider).ok_or_else(|| format!("Invalid provider: {}", provider))?;

    KeyringManager::clear_api_key(&ai_provider).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_available_providers() -> Result<Vec<String>, String> {
    Ok(vec![
        "openai".to_string(),
        "anthropic".to_string(),
        "gemini".to_string(),
        "perplexity".to_string(),
        "openrouter".to_string(),
        "xai".to_string(),
        "deepseek".to_string(),
        "groq".to_string(),
    ])
}

#[tauri::command]
pub fn get_provider_models(provider: String) -> Result<Vec<String>, String> {
    if let Some(ai_provider) = AIProvider::from_str(&provider) {
        Ok(ai_provider
            .default_models()
            .into_iter()
            .map(|s| s.to_string())
            .collect())
    } else {
        Err(format!("Unknown provider: {}", provider))
    }
}

#[tauri::command]
pub fn get_provider_capabilities(provider: String) -> Result<HashMap<String, bool>, String> {
    let ai_provider =
        AIProvider::from_str(&provider).ok_or_else(|| format!("Unknown provider: {}", provider))?;

    let mut capabilities = HashMap::new();
    capabilities.insert("streaming".to_string(), ai_provider.supports_streaming());
    capabilities.insert(
        "function_calling".to_string(),
        ai_provider.supports_function_calling(),
    );

    Ok(capabilities)
}

#[tauri::command]
pub fn ai_can_access(app: tauri::AppHandle) -> Result<bool, String> {
    let settings = get_ai_settings(app)?;
    if !settings.enabled {
        return Ok(false);
    }

    // Check if any provider has an API key set
    for provider_name in settings.get_enabled_providers() {
        if let Some(ai_provider) = AIProvider::from_str(&provider_name) {
            if KeyringManager::is_api_key_set(&ai_provider) {
                return Ok(true);
            }
        }
    }

    // Fallback to legacy check for OpenRouter
    Ok(is_ai_api_key_set())
}

pub struct AiUsageManager {
    store: Store,
}

impl AiUsageManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self, AppError> {
        let store = Store::new(app_handle, "ai_usage.sqlite")?;
        store.init_table(AI_USAGE_SCHEMA)?;
        Ok(Self { store })
    }

    pub fn log_generation(&self, data: &GenerationData) -> Result<(), AppError> {
        self.store.execute(
            "INSERT OR REPLACE INTO ai_generations (id, created, model, tokens_prompt, tokens_completion, native_tokens_prompt, native_tokens_completion, total_cost)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                data.id,
                data.created,
                data.model,
                data.tokens_prompt,
                data.tokens_completion,
                data.native_tokens_prompt,
                data.native_tokens_completion,
                data.total_cost
            ],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: u32, offset: u32) -> Result<Vec<GenerationData>, AppError> {
        self.store.query(
            "SELECT id, created, model, tokens_prompt, tokens_completion, native_tokens_prompt, native_tokens_completion, total_cost FROM ai_generations ORDER BY created DESC LIMIT ?1 OFFSET ?2",
            params![limit, offset],
        )
    }

    pub fn get_usage_stats(&self, days: u32) -> Result<HashMap<String, i64>, AppError> {
        let since_timestamp = chrono::Utc::now().timestamp() - (days as i64 * 24 * 60 * 60);

        let rows: Vec<(String, i64, i64)> = self.store.query(
            "SELECT model, SUM(tokens_prompt), SUM(tokens_completion) FROM ai_generations WHERE created > ?1 GROUP BY model",
            params![since_timestamp],
        )?;

        let mut stats = HashMap::new();
        for (model, prompt_tokens, completion_tokens) in rows {
            stats.insert(format!("{}_prompt_tokens", model), prompt_tokens);
            stats.insert(format!("{}_completion_tokens", model), completion_tokens);
            stats.insert(
                format!("{}_total_tokens", model),
                prompt_tokens + completion_tokens,
            );
        }

        Ok(stats)
    }
}

#[tauri::command]
pub fn get_ai_usage_history(
    manager: State<AiUsageManager>,
    limit: u32,
    offset: u32,
) -> Result<Vec<GenerationData>, String> {
    manager
        .get_history(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ai_usage_stats(
    manager: State<AiUsageManager>,
    days: u32,
) -> Result<HashMap<String, i64>, String> {
    manager.get_usage_stats(days).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ai_ask_stream(
    app_handle: AppHandle,
    request_id: String,
    prompt: String,
    options: AskOptions,
) -> Result<(), String> {
    let settings = get_ai_settings(app_handle.clone())?;
    if !settings.enabled {
        return Err("AI features are not enabled.".to_string());
    }

    // Determine which provider to use based on options or default
    let provider = if let Some(provider_str) = &options.provider {
        AIProvider::from_str(provider_str)
            .ok_or_else(|| format!("Invalid provider: {}", provider_str))?
    } else {
        AIProvider::from_str(&settings.default_provider)
            .ok_or_else(|| format!("Invalid default provider: {}", settings.default_provider))?
    };

    // Check if provider supports streaming
    if !provider.supports_streaming() {
        return Err(format!(
            "Provider {:?} does not support streaming",
            provider
        ));
    }

    // Get provider configuration
    let config = settings
        .providers
        .get(&provider.as_str().to_string())
        .ok_or_else(|| format!("No configuration found for provider: {:?}", provider))?;

    if !config.enabled {
        return Err(format!("Provider {:?} is not enabled", provider));
    }

    // Get API key for the provider
    let api_key = KeyringManager::get_api_key(&provider)
        .map_err(|e| format!("Failed to get API key for {:?}: {}", provider, e))?;

    // Create agent using factory
    let agent = ProviderAgentFactory::create_agent(&provider, config, &api_key, &options)?;

    let mut full_text = String::new();

    // Create streaming prompt
    let mut stream = agent
        .stream_prompt(&prompt)
        .await
        .map_err(|e| format!("Stream error: {}", e))?;

    // Process the stream with improved error handling
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                match chunk {
                    rig::completion::AssistantContent::Text(content) => {
                        let content_str = content.text.clone();
                        full_text.push_str(&content_str);

                        // Emit chunk to frontend
                        if let Err(e) = app_handle.emit(
                            "ai-stream-chunk",
                            StreamChunk {
                                request_id: request_id.clone(),
                                text: content_str,
                            },
                        ) {
                            eprintln!("Failed to emit stream chunk: {}", e);
                        }
                    }
                    _ => {
                        // Handle other types of content if needed
                    }
                }
            }
            Err(e) => {
                eprintln!("Stream error: {}", e);
                return Err(format!("Stream error: {}", e));
            }
        }
    }

    // Emit end signal
    if let Err(e) = app_handle.emit(
        "ai-stream-end",
        StreamEnd {
            request_id: request_id.clone(),
            full_text: full_text.clone(),
        },
    ) {
        eprintln!("Failed to emit stream end: {}", e);
        return Err(format!("Failed to emit stream end: {}", e));
    }

    // Log usage (you may want to extract token count from the stream response)
    if let Ok(usage_manager) = app_handle.try_state::<AiUsageManager>() {
        let generation_data = GenerationData {
            id: request_id,
            created: chrono::Utc::now().timestamp(),
            model: options.model.unwrap_or(config.model.clone()),
            tokens_prompt: 0,     // TODO: Extract from response
            tokens_completion: 0, // TODO: Extract from response
            native_tokens_prompt: 0,
            native_tokens_completion: 0,
            total_cost: 0.0,
        };

        if let Err(e) = usage_manager.log_generation(&generation_data) {
            eprintln!("Failed to log generation: {}", e);
        }
    }

    Ok(())
}

// Legacy API functions for backward compatibility
#[tauri::command]
pub fn set_ai_api_key(api_key: String) -> Result<(), String> {
    KeyringManager::get_legacy_entry()
        .and_then(|entry| entry.set_password(&api_key).map_err(AppError::from))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_ai_api_key_set() -> bool {
    KeyringManager::get_legacy_entry()
        .and_then(|entry| entry.get_password().map_err(AppError::from))
        .is_ok()
}

#[tauri::command]
pub fn clear_ai_api_key() -> Result<(), String> {
    KeyringManager::get_legacy_entry()
        .and_then(|entry| entry.delete_credential().map_err(AppError::from))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_provider_from_str() {
        assert_eq!(AIProvider::from_str("openai"), Some(AIProvider::OpenAI));
        assert_eq!(
            AIProvider::from_str("anthropic"),
            Some(AIProvider::Anthropic)
        );
        assert_eq!(AIProvider::from_str("invalid"), None);
    }

    #[test]
    fn test_ai_settings_validation() {
        let mut settings = AiSettings::default();
        settings.default_provider = "nonexistent".to_string();

        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_provider_capabilities() {
        assert!(AIProvider::OpenAI.supports_streaming());
        assert!(AIProvider::OpenAI.supports_function_calling());
        assert!(!AIProvider::Perplexity.supports_function_calling());

        // Test new providers
        assert!(AIProvider::Anthropic.supports_streaming());
        assert!(AIProvider::Anthropic.supports_function_calling());
        assert!(AIProvider::Gemini.supports_streaming());
        assert!(AIProvider::Gemini.supports_function_calling());
        assert!(AIProvider::Groq.supports_streaming());
        assert!(AIProvider::Groq.supports_function_calling());
    }
}
