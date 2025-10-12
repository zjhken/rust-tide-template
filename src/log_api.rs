use anyhow_ext::Result;
use serde::{Deserialize, Serialize};
use surf::StatusCode;
use tide::{Request, Response};

use crate::logging::{set_target_log_level, get_target_log_level, remove_target_log_level, list_target_log_levels};
use crate::server::make_resp;
use crate::test_logs::{generate_test_logs, generate_test_logs_for_current_module};

#[derive(Deserialize)]
pub struct SetLogLevelRequest {
    pub target: String,
    pub level: String,
}

#[derive(Serialize)]
pub struct LogLevelResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

#[derive(Serialize)]
pub struct TargetLogLevel {
    pub target: String,
    pub level: String,
}

#[derive(Serialize)]
pub struct ListLogLevelsResponse {
    pub success: bool,
    pub targets: Vec<TargetLogLevel>,
}

pub fn parse_log_level(level: &str) -> Result<log::LevelFilter> {
    match level.to_lowercase().as_str() {
        "off" => Ok(log::LevelFilter::Off),
        "error" => Ok(log::LevelFilter::Error),
        "warn" => Ok(log::LevelFilter::Warn),
        "info" => Ok(log::LevelFilter::Info),
        "debug" => Ok(log::LevelFilter::Debug),
        "trace" => Ok(log::LevelFilter::Trace),
        other => Err(anyhow_ext::anyhow!("Invalid log level: {}", other)),
    }
}

pub fn format_log_level(level: log::LevelFilter) -> String {
    match level {
        log::LevelFilter::Off => "off".to_string(),
        log::LevelFilter::Error => "error".to_string(),
        log::LevelFilter::Warn => "warn".to_string(),
        log::LevelFilter::Info => "info".to_string(),
        log::LevelFilter::Debug => "debug".to_string(),
        log::LevelFilter::Trace => "trace".to_string(),
    }
}

pub async fn handle_set_log_level(mut req: Request<()>) -> tide::Result<Response> {
    let body: SetLogLevelRequest = req.body_json().await?;

    match parse_log_level(&body.level) {
        Ok(level_filter) => {
            match set_target_log_level(body.target.clone(), level_filter) {
                Ok(()) => {
                    let response = LogLevelResponse {
                        success: true,
                        message: format!("Log level for '{}' set to {:?}", body.target, body.level),
                        level: Some(body.level),
                    };
                    Ok(make_resp(StatusCode::Ok, serde_json::to_string(&response)?))
                }
                Err(e) => {
                    let response = LogLevelResponse {
                        success: false,
                        message: format!("Failed to set log level: {}", e),
                        level: None,
                    };
                    Ok(make_resp(StatusCode::InternalServerError, serde_json::to_string(&response)?))
                }
            }
        }
        Err(e) => {
            let response = LogLevelResponse {
                success: false,
                message: format!("Invalid log level: {}", e),
                level: None,
            };
            Ok(make_resp(StatusCode::BadRequest, serde_json::to_string(&response)?))
        }
    }
}

pub async fn handle_get_log_level(req: Request<()>) -> tide::Result<Response> {
    let target = req.param("target")?;

    match get_target_log_level(target) {
        Some(level) => {
            let response = LogLevelResponse {
                success: true,
                message: format!("Log level for '{}'", target),
                level: Some(format_log_level(level)),
            };
            Ok(make_resp(StatusCode::Ok, serde_json::to_string(&response)?))
        }
        None => {
            let response = LogLevelResponse {
                success: false,
                message: format!("No custom log level set for target '{}'", target),
                level: None,
            };
            Ok(make_resp(StatusCode::NotFound, serde_json::to_string(&response)?))
        }
    }
}

pub async fn handle_delete_log_level(req: Request<()>) -> tide::Result<Response> {
    let target = req.param("target")?;

    match remove_target_log_level(target) {
        Ok(removed) => {
            if removed {
                let response = LogLevelResponse {
                    success: true,
                    message: format!("Log level for '{}' removed", target),
                    level: None,
                };
                Ok(make_resp(StatusCode::Ok, serde_json::to_string(&response)?))
            } else {
                let response = LogLevelResponse {
                    success: false,
                    message: format!("No custom log level found for target '{}'", target),
                    level: None,
                };
                Ok(make_resp(StatusCode::NotFound, serde_json::to_string(&response)?))
            }
        }
        Err(e) => {
            let response = LogLevelResponse {
                success: false,
                message: format!("Failed to remove log level: {}", e),
                level: None,
            };
            Ok(make_resp(StatusCode::InternalServerError, serde_json::to_string(&response)?))
        }
    }
}

pub async fn handle_list_log_levels(_req: Request<()>) -> tide::Result<Response> {
    let target_levels = list_target_log_levels();
    let targets: Vec<TargetLogLevel> = target_levels
        .into_iter()
        .map(|(target, level)| TargetLogLevel {
            target,
            level: format_log_level(level),
        })
        .collect();

    let response = ListLogLevelsResponse {
        success: true,
        targets,
    };

    Ok(make_resp(StatusCode::Ok, serde_json::to_string(&response)?))
}

#[derive(Deserialize)]
pub struct GenerateTestLogsRequest {
    pub target: String,
}

#[derive(Serialize)]
pub struct TestLogsResponse {
    pub success: bool,
    pub message: String,
    pub target: String,
    pub generated_levels: Vec<String>,
}

pub async fn handle_generate_test_logs(mut req: Request<()>) -> tide::Result<Response> {
    let body: GenerateTestLogsRequest = req.body_json().await?;

    // Generate logs at all levels for the specified target
    generate_test_logs(&body.target);

    // Also generate logs for the current module to test default behavior
    generate_test_logs_for_current_module();

    let response = TestLogsResponse {
        success: true,
        message: "Test logs generated. Check server logs to verify filtering behavior.".to_string(),
        target: body.target.clone(),
        generated_levels: vec!["trace".to_string(), "debug".to_string(), "info".to_string(), "warn".to_string(), "error".to_string()],
    };

    Ok(make_resp(StatusCode::Ok, serde_json::to_string(&response)?))
}