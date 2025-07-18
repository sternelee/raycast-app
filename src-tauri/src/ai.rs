use crate::error::AppError;
use crate::store::{Storable, Store};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use rig::prelude::*;
use rig::providers::openai;
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
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            top_p: None,
            max_tokens: None,
            enabled: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum AIProvider {
    OpenAI,
    Anthropic,
    Gemini,
    Perplexity,
    OpenRouter,
    XAI,
    DeepSeek,
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
            _ => None,
        }
    }

    pub fn default_models(&self) -> Vec<&'static str> {
        match self {
            AIProvider::OpenAI => vec!["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-4", "o1", "o3"],
            AIProvider::Anthropic => vec![
                "claude-3-opus-20240229",
                "claude-3-sonnet-20240229",
                "claude-3-haiku-20240307",
            ],
            AIProvider::Gemini => {
                vec!["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash-001"]
            }
            AIProvider::Perplexity => vec![
                "llama-3.1-sonar-small-128k-online",
                "llama-3.1-sonar-large-128k-online",
            ],
            AIProvider::OpenRouter => vec![
                "openai/gpt-4o",
                "anthropic/claude-3-sonnet",
                "meta-llama/llama-3.1-405b-instruct",
            ],
            AIProvider::XAI => vec!["grok-beta"],
            AIProvider::DeepSeek => vec!["deepseek-chat", "deepseek-coder"],
        }
    }
}
static DEFAULT_AI_MODELS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // OpenAI
    m.insert("OpenAI_GPT4.1", "openai/gpt-4.1");
    m.insert("OpenAI_GPT4.1-mini", "openai/gpt-4.1-mini");
    m.insert("OpenAI_GPT4.1-nano", "openai/gpt-4.1-nano");
    m.insert("OpenAI_GPT4", "openai/gpt-4");
    m.insert("OpenAI_GPT4-turbo", "openai/gpt-4-turbo");
    m.insert("OpenAI_GPT4o", "openai/gpt-4o");
    m.insert("OpenAI_GPT4o-mini", "openai/gpt-4o-mini");
    m.insert("OpenAI_o3", "openai/o3");
    m.insert("OpenAI_o4-mini", "openai/o4-mini");
    m.insert("OpenAI_o1", "openai/o1");
    m.insert("OpenAI_o3-mini", "openai/o3-mini");
    // Anthropic
    m.insert("Anthropic_Claude_Haiku", "anthropic/claude-3-haiku");
    m.insert("Anthropic_Claude_Sonnet", "anthropic/claude-3-sonnet");
    m.insert("Anthropic_Claude_Sonnet_3.7", "anthropic/claude-3.7-sonnet");
    m.insert("Anthropic_Claude_Opus", "anthropic/claude-3-opus");
    m.insert("Anthropic_Claude_4_Sonnet", "anthropic/claude-sonnet-4");
    m.insert("Anthropic_Claude_4_Opus", "anthropic/claude-opus-4");
    // Perplexity
    m.insert("Perplexity_Sonar", "perplexity/sonar");
    m.insert("Perplexity_Sonar_Pro", "perplexity/sonar-pro");
    m.insert("Perplexity_Sonar_Reasoning", "perplexity/sonar-reasoning");
    m.insert(
        "Perplexity_Sonar_Reasoning_Pro",
        "perplexity/sonar-reasoning-pro",
    );
    // Meta (via OpenRouter)
    m.insert("Llama4_Scout", "meta-llama/llama-4-scout");
    m.insert("Llama3.3_70B", "meta-llama/llama-3.3-70b-instruct");
    m.insert("Llama3.1_8B", "meta-llama/llama-3.1-8b-instruct");
    m.insert("Llama3.1_405B", "meta-llama/llama-3.1-405b-instruct");
    // Mistral (via OpenRouter)
    m.insert("Mistral_Nemo", "mistralai/mistral-nemo");
    m.insert("Mistral_Large", "mistralai/mistral-large");
    m.insert("Mistral_Medium", "mistralai/mistral-medium-3");
    m.insert("Mistral_Small", "mistralai/mistral-small");
    m.insert("Mistral_Codestral", "mistralai/codestral-2501");
    // DeepSeek (via OpenRouter)
    m.insert(
        "DeepSeek_R1_Distill_Llama_3.3_70B",
        "deepseek/deepseek-r1-distill-llama-70b",
    );
    m.insert("DeepSeek_R1", "deepseek/deepseek-r1");
    m.insert("DeepSeek_V3", "deepseek/deepseek-chat");
    // Google
    m.insert("Google_Gemini_2.5_Pro", "google/gemini-2.5-pro");
    m.insert("Google_Gemini_2.5_Flash", "google/gemini-2.5-flash");
    m.insert("Google_Gemini_2.0_Flash", "google/gemini-2.0-flash-001");
    // xAI (via OpenRouter)
    m.insert("xAI_Grok_3", "x-ai/grok-3");
    m.insert("xAI_Grok_3_Mini", "x-ai/grok-3-mini");
    m.insert("xAI_Grok_2", "x-ai/grok-2-1212");

    m
});

fn get_default_provider_configs() -> HashMap<String, ProviderConfig> {
    let mut configs = HashMap::new();

    // OpenAI default config
    configs.insert(
        "openai".to_string(),
        ProviderConfig {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: true,
        },
    );

    // Anthropic default config
    configs.insert(
        "anthropic".to_string(),
        ProviderConfig {
            api_key: None,
            model: "claude-3-5-sonnet-20241022".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: false,
        },
    );

    // Add other providers...
    configs.insert(
        "gemini".to_string(),
        ProviderConfig {
            api_key: None,
            model: "gemini-1.5-flash".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: false,
        },
    );

    configs.insert(
        "perplexity".to_string(),
        ProviderConfig {
            api_key: None,
            model: "llama-3.1-sonar-small-128k-online".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: false,
        },
    );

    configs.insert(
        "xai".to_string(),
        ProviderConfig {
            api_key: None,
            model: "grok-beta".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: false,
        },
    );

    configs.insert(
        "deepseek".to_string(),
        ProviderConfig {
            api_key: None,
            model: "deepseek-chat".to_string(),
            temperature: 0.7,
            top_p: Some(1.0),
            max_tokens: Some(2048),
            enabled: false,
        },
    );

    configs
}

// Keyring functions for each provider
fn get_openai_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new("dev.byteatatime.raycast.ai", "openai_api_key").map_err(AppError::from)
}

fn get_anthropic_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new("dev.byteatatime.raycast.ai", "anthropic_api_key").map_err(AppError::from)
}

fn get_gemini_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new("dev.byteatatime.raycast.ai", "gemini_api_key").map_err(AppError::from)
}

fn get_perplexity_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new("dev.byteatatime.raycast.ai", "perplexity_api_key").map_err(AppError::from)
}

fn get_xai_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new("dev.byteatatime.raycast.ai", "xai_api_key").map_err(AppError::from)
}

fn get_deepseek_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new("dev.byteatatime.raycast.ai", "deepseek_api_key").map_err(AppError::from)
}

// Legacy keyring function for backward compatibility
fn get_keyring_entry() -> Result<keyring::Entry, AppError> {
    keyring::Entry::new(AI_KEYRING_SERVICE, AI_KEYRING_USERNAME).map_err(AppError::from)
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    enabled: bool,
    model_associations: HashMap<String, String>,
    providers: HashMap<String, ProviderConfig>,
    default_provider: String,
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
        user_settings.default_provider = "openrouter".to_string();
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

    Ok(user_settings)
}

#[tauri::command]
pub fn set_ai_settings(app: tauri::AppHandle, settings: AiSettings) -> Result<(), String> {
    let path = get_settings_path(&app)?;

    let mut settings_to_save = AiSettings {
        enabled: settings.enabled,
        model_associations: HashMap::new(),
        providers: settings.providers.clone(),
        default_provider: settings.default_provider.clone(),
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

fn get_keyring_entry_for_provider(provider: &str) -> Result<keyring::Entry, AppError> {
    let service = format!("dev.byteatatime.raycast.ai.{}", provider);
    let username = format!("{}_api_key", provider);
    keyring::Entry::new(&service, &username).map_err(AppError::from)
}

#[tauri::command]
pub fn set_provider_api_key(provider: String, key: String) -> Result<(), String> {
    get_keyring_entry_for_provider(&provider)
        .and_then(|entry| entry.set_password(&key).map_err(AppError::from))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_provider_api_key_set(provider: String) -> Result<bool, String> {
    match get_keyring_entry_for_provider(&provider)
        .and_then(|entry| entry.get_password().map_err(AppError::from))
    {
        Ok(_) => Ok(true),
        Err(AppError::Keyring(keyring::Error::NoEntry)) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn clear_provider_api_key(provider: String) -> Result<(), String> {
    get_keyring_entry_for_provider(&provider)
        .and_then(|entry| entry.delete_credential().map_err(AppError::from))
        .map_err(|e| e.to_string())
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
pub fn ai_can_access(app: tauri::AppHandle) -> Result<bool, String> {
    let settings = get_ai_settings(app)?;
    if !settings.enabled {
        return Ok(false);
    }
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
        AIProvider::OpenAI // Default provider
    };

    // Get provider configuration
    let config = settings
        .providers
        .get(&provider.as_str().to_string())
        .ok_or_else(|| format!("No configuration found for provider: {:?}", provider))?;

    // Get API key for the provider
    let api_key = match provider {
        AIProvider::OpenAI => get_openai_keyring_entry(),
        AIProvider::Anthropic => get_anthropic_keyring_entry(),
        AIProvider::Gemini => get_gemini_keyring_entry(),
        AIProvider::Perplexity => get_perplexity_keyring_entry(),
        AIProvider::OpenRouter => get_keyring_entry(), // Use legacy function for OpenRouter
        AIProvider::XAI => get_xai_keyring_entry(),
        AIProvider::DeepSeek => get_deepseek_keyring_entry(),
    }
    .and_then(|entry| entry.get_password().map_err(AppError::from))
    .map_err(|e| format!("Failed to get API key for {:?}: {}", provider, e))?;

    // Create agent based on provider
    let mut full_text = String::new();

    match provider {
        AIProvider::OpenAI => {
            let client = openai::Client::new(&api_key);
            let mut agent_builder = client.agent(&config.model);

            agent_builder = agent_builder.temperature(config.temperature);
            if let Some(max_tokens) = config.max_tokens {
                agent_builder = agent_builder.max_tokens(max_tokens as u64);
            }

            let agent = agent_builder.build();
            let mut stream = agent
                .stream_prompt(&prompt)
                .await
                .map_err(|e| format!("Stream error: {}", e))?;

            // Process the stream
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        match chunk {
                            rig::completion::AssistantContent::Text(content) => {
                                let content_str = content.text.clone();
                                full_text.push_str(&content_str);

                                // Emit chunk to frontend
                                app_handle
                                    .emit(
                                        "ai-stream-chunk",
                                        StreamChunk {
                                            request_id: request_id.clone(),
                                            text: content_str,
                                        },
                                    )
                                    .map_err(|e| e.to_string())?;
                            }
                            _ => {
                                // Handle other types of content if needed
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Stream error: {}", e));
                    }
                }
            }
        }
        _ => {
            return Err(format!(
                "Provider {:?} not yet implemented in streaming",
                provider
            ));
        }
    }

    // Emit end signal
    app_handle
        .emit(
            "ai-stream-end",
            StreamEnd {
                request_id: request_id.clone(),
                full_text: full_text.clone(),
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// Legacy API functions for backward compatibility
#[tauri::command]
pub fn set_ai_api_key(api_key: String) -> Result<(), String> {
    let entry = get_keyring_entry().map_err(|e| e.to_string())?;
    entry.set_password(&api_key).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn is_ai_api_key_set() -> bool {
    get_keyring_entry()
        .and_then(|entry| entry.get_password().map_err(AppError::from))
        .is_ok()
}

#[tauri::command]
pub fn clear_ai_api_key() -> Result<(), String> {
    let entry = get_keyring_entry().map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())?;
    Ok(())
}
