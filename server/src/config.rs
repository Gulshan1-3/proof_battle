use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidValue {
        var: String,
        val: String,
        expected: &'static str,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue { var, val, expected } => {
                write!(
                    f,
                    "invalid value for {var}: \"{val}\" (expected {expected})"
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

// Fields are consumed across implementation stages:
// - S1/S2: bind_addr, lean_project_dir, proof_tmp_dir, lean_timeout
// - S2: proof_max_bytes, proof_max_lines, lean_max_heartbeats, lean_max_memory_mb, lean_pool_size
// - S3: sandbox_enabled, sandbox_image
// - S4: protocol_version
// - S6: database_url, session_ttl, rate_limit_submissions, rate_limit_window, job_queue_capacity, submit_lane_reserve, check_lane_capacity
// - S9: log_filter
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub lean_project_dir: PathBuf,    // the lake project root (lean/)
    pub proof_tmp_dir: PathBuf,       // where submissions are materialised
    pub proof_max_bytes: usize,       // default 8192
    pub proof_max_lines: usize,       // default 120
    pub lean_timeout: Duration,       // default 30s
    pub lean_max_heartbeats: u32,     // default 200_000
    pub lean_max_memory_mb: u32,      // default 2048
    pub lean_pool_size: usize,        // default 4
    pub job_queue_capacity: usize,    // default 256
    pub submit_lane_reserve: usize,   // default 2   (ADR-008)
    pub check_lane_capacity: usize,   // default 8
    pub session_ttl: Duration,        // default 60s
    pub rate_limit_submissions: u32,  // default 5 per window
    pub rate_limit_window: Duration,  // default 60s
    pub database_url: Option<String>, // None => in-memory problem source
    pub sandbox_enabled: bool,        // default false in dev, MUST be true in prod
    pub sandbox_image: String,        // default "proofbattle/lean-sandbox:lean-4.21.0-rc3"
    pub reconnect_grace_window: Duration, // default 10s
    pub protocol_version: u32,        // default 1
    pub log_filter: String,           // default "proof_battle_server=info"
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let env_map: HashMap<String, String> = std::env::vars().collect();
        Self::from_map(&env_map)
    }

    pub fn from_map(map: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let bind_addr = map
            .get("BIND_ADDR")
            .cloned()
            .unwrap_or_else(|| "0.0.0.0:3000".to_string());

        let lean_project_dir = PathBuf::from(
            map.get("LEAN_PROJECT_DIR")
                .or_else(|| map.get("LEAN_PATH"))
                .cloned()
                .unwrap_or_else(|| "lean".to_string()),
        );

        let proof_tmp_dir = PathBuf::from(
            map.get("PROOF_TMP_DIR")
                .cloned()
                .unwrap_or_else(|| "lean/proofs".to_string()),
        );

        let proof_max_bytes = parse_usize(map, "PROOF_MAX_BYTES", 8192, "positive integer bytes")?;
        let proof_max_lines = parse_usize(map, "PROOF_MAX_LINES", 120, "positive integer lines")?;

        let lean_timeout_secs = parse_u64(map, "LEAN_TIMEOUT", 30, "integer seconds")?;
        let lean_timeout = Duration::from_secs(lean_timeout_secs);

        let lean_max_heartbeats =
            parse_u32(map, "LEAN_MAX_HEARTBEATS", 200_000, "positive integer")?;
        let lean_max_memory_mb = parse_u32(
            map,
            "LEAN_MAX_MEMORY_MB",
            2048,
            "positive integer megabytes",
        )?;
        let lean_pool_size = parse_usize(map, "LEAN_POOL_SIZE", 4, "positive integer")?;
        let job_queue_capacity = parse_usize(map, "JOB_QUEUE_CAPACITY", 256, "positive integer")?;
        let submit_lane_reserve = parse_usize(map, "SUBMIT_LANE_RESERVE", 2, "positive integer")?;
        let check_lane_capacity = parse_usize(map, "CHECK_LANE_CAPACITY", 8, "positive integer")?;

        let session_ttl_secs = parse_u64(map, "SESSION_TTL", 60, "integer seconds")?;
        let session_ttl = Duration::from_secs(session_ttl_secs);

        let rate_limit_submissions =
            parse_u32(map, "RATE_LIMIT_SUBMISSIONS", 5, "positive integer")?;
        let rate_limit_window_secs = parse_u64(map, "RATE_LIMIT_WINDOW", 60, "integer seconds")?;
        let rate_limit_window = Duration::from_secs(rate_limit_window_secs);

        let database_url = map.get("DATABASE_URL").filter(|s| !s.is_empty()).cloned();

        let sandbox_enabled =
            parse_bool(map, "SANDBOX_ENABLED", false, "boolean (true/false/1/0)")?;
        let sandbox_image = map
            .get("SANDBOX_IMAGE")
            .cloned()
            .unwrap_or_else(|| "proofbattle/lean-sandbox:lean-4.21.0-rc3".to_string());

        let reconnect_grace_ms = parse_u64(
            map,
            "RECONNECT_GRACE_WINDOW_MS",
            10_000,
            "integer milliseconds",
        )?;
        let reconnect_grace_window = Duration::from_millis(reconnect_grace_ms);

        let protocol_version = parse_u32(map, "PROTOCOL_VERSION", 1, "positive integer")?;
        let log_filter = map
            .get("LOG_FILTER")
            .or_else(|| map.get("RUST_LOG"))
            .cloned()
            .unwrap_or_else(|| "proof_battle_server=info".to_string());

        let pb_env = map
            .get("PB_ENV")
            .or_else(|| map.get("ENVIRONMENT"))
            .cloned()
            .unwrap_or_else(|| "development".to_string());

        if pb_env == "production" && database_url.is_none() {
            return Err(ConfigError::InvalidValue {
                var: "DATABASE_URL".to_string(),
                val: "".to_string(),
                expected: "DATABASE_URL is strictly required when PB_ENV=production",
            });
        }

        if database_url.is_none() {
            tracing::warn!("in-memory problem source: 3 problems, ELO disabled");
        }

        if !sandbox_enabled {
            tracing::warn!("SANDBOX DISABLED — LOCAL DEVELOPMENT ONLY");
        }

        Ok(Config {
            bind_addr,
            lean_project_dir,
            proof_tmp_dir,
            proof_max_bytes,
            proof_max_lines,
            lean_timeout,
            lean_max_heartbeats,
            lean_max_memory_mb,
            lean_pool_size,
            job_queue_capacity,
            submit_lane_reserve,
            check_lane_capacity,
            session_ttl,
            rate_limit_submissions,
            rate_limit_window,
            database_url,
            sandbox_enabled,
            sandbox_image,
            reconnect_grace_window,
            protocol_version,
            log_filter,
        })
    }
}

fn parse_usize(
    map: &HashMap<String, String>,
    var: &str,
    default: usize,
    expected: &'static str,
) -> Result<usize, ConfigError> {
    match map.get(var) {
        None => Ok(default),
        Some(val) => val.parse::<usize>().map_err(|_| ConfigError::InvalidValue {
            var: var.to_string(),
            val: val.clone(),
            expected,
        }),
    }
}

fn parse_u64(
    map: &HashMap<String, String>,
    var: &str,
    default: u64,
    expected: &'static str,
) -> Result<u64, ConfigError> {
    match map.get(var) {
        None => Ok(default),
        Some(val) => val.parse::<u64>().map_err(|_| ConfigError::InvalidValue {
            var: var.to_string(),
            val: val.clone(),
            expected,
        }),
    }
}

fn parse_u32(
    map: &HashMap<String, String>,
    var: &str,
    default: u32,
    expected: &'static str,
) -> Result<u32, ConfigError> {
    match map.get(var) {
        None => Ok(default),
        Some(val) => val.parse::<u32>().map_err(|_| ConfigError::InvalidValue {
            var: var.to_string(),
            val: val.clone(),
            expected,
        }),
    }
}

fn parse_bool(
    map: &HashMap<String, String>,
    var: &str,
    default: bool,
    expected: &'static str,
) -> Result<bool, ConfigError> {
    match map.get(var) {
        None => Ok(default),
        Some(val) => match val.to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(ConfigError::InvalidValue {
                var: var.to_string(),
                val: val.clone(),
                expected,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_defaults() {
        let env = HashMap::new();
        let config = Config::from_map(&env).expect("default config should parse cleanly");
        assert_eq!(config.bind_addr, "0.0.0.0:3000");
        assert_eq!(config.lean_project_dir, PathBuf::from("lean"));
        assert_eq!(config.proof_tmp_dir, PathBuf::from("lean/proofs"));
        assert_eq!(config.proof_max_bytes, 8192);
        assert_eq!(config.proof_max_lines, 120);
        assert_eq!(config.lean_timeout, Duration::from_secs(30));
        assert_eq!(config.lean_max_heartbeats, 200_000);
        assert_eq!(config.lean_max_memory_mb, 2048);
        assert_eq!(config.lean_pool_size, 4);
        assert_eq!(config.job_queue_capacity, 256);
        assert_eq!(config.submit_lane_reserve, 2);
        assert_eq!(config.check_lane_capacity, 8);
        assert_eq!(config.session_ttl, Duration::from_secs(60));
        assert_eq!(config.rate_limit_submissions, 5);
        assert_eq!(config.rate_limit_window, Duration::from_secs(60));
        assert_eq!(config.database_url, None);
        assert!(!config.sandbox_enabled);
        assert_eq!(
            config.sandbox_image,
            "proofbattle/lean-sandbox:lean-4.21.0-rc3"
        );
        assert_eq!(config.protocol_version, 1);
        assert_eq!(config.log_filter, "proof_battle_server=info");
    }

    #[test]
    fn test_override_values() {
        let mut env = HashMap::new();
        env.insert("BIND_ADDR".to_string(), "127.0.0.1:8080".to_string());
        env.insert("LEAN_TIMEOUT".to_string(), "45".to_string());
        env.insert("SANDBOX_ENABLED".to_string(), "true".to_string());
        env.insert(
            "DATABASE_URL".to_string(),
            "postgres://user:pass@localhost:5432/db".to_string(),
        );

        let config = Config::from_map(&env).expect("overridden config should parse cleanly");
        assert_eq!(config.bind_addr, "127.0.0.1:8080");
        assert_eq!(config.lean_timeout, Duration::from_secs(45));
        assert!(config.sandbox_enabled);
        assert_eq!(
            config.database_url,
            Some("postgres://user:pass@localhost:5432/db".to_string())
        );
    }

    #[test]
    fn test_malformed_lean_timeout() {
        let mut env = HashMap::new();
        env.insert("LEAN_TIMEOUT".to_string(), "notanumber".to_string());

        let err = Config::from_map(&env).unwrap_err();
        assert_eq!(
            err,
            ConfigError::InvalidValue {
                var: "LEAN_TIMEOUT".to_string(),
                val: "notanumber".to_string(),
                expected: "integer seconds",
            }
        );
        assert_eq!(
            err.to_string(),
            "invalid value for LEAN_TIMEOUT: \"notanumber\" (expected integer seconds)"
        );
    }

    #[test]
    fn test_malformed_numeric_types() {
        let mut env = HashMap::new();
        env.insert("PROOF_MAX_BYTES".to_string(), "bad_bytes".to_string());
        assert_eq!(
            Config::from_map(&env).unwrap_err().to_string(),
            "invalid value for PROOF_MAX_BYTES: \"bad_bytes\" (expected positive integer bytes)"
        );

        let mut env = HashMap::new();
        env.insert("LEAN_MAX_HEARTBEATS".to_string(), "xyz".to_string());
        assert_eq!(
            Config::from_map(&env).unwrap_err().to_string(),
            "invalid value for LEAN_MAX_HEARTBEATS: \"xyz\" (expected positive integer)"
        );
    }

    #[test]
    fn test_bool_override_and_malformed() {
        let mut env = HashMap::new();
        env.insert("SANDBOX_ENABLED".to_string(), "1".to_string());
        let config = Config::from_map(&env).unwrap();
        assert!(config.sandbox_enabled);

        let mut env = HashMap::new();
        env.insert("SANDBOX_ENABLED".to_string(), "invalid_bool".to_string());
        assert_eq!(
            Config::from_map(&env).unwrap_err().to_string(),
            "invalid value for SANDBOX_ENABLED: \"invalid_bool\" (expected boolean (true/false/1/0))"
        );
    }

    #[test]
    fn test_production_database_url_requirement() {
        let mut env = HashMap::new();
        env.insert("PB_ENV".to_string(), "production".to_string());
        let err = Config::from_map(&env).unwrap_err();
        assert!(
            err.to_string()
                .contains("DATABASE_URL is strictly required")
        );
    }
}
