// parameter_config_commands.rs - 参数配置Tauri命令接口
// 提供前端访问配置管理功能的接口

use crate::modules::parameter_config::{
    CalibrationConfig, AlignmentConfig,
    load_calibration_config, load_alignment_config,
    save_calibration_config, save_alignment_config
};
use serde::{Deserialize, Serialize};
use tauri::command;

// ============= 配置获取命令 =============

/// 获取标定配置
#[command]
pub fn get_calibration_config() -> Result<CalibrationConfig, String> {
    load_calibration_config()
        .map_err(|e| format!("加载标定配置失败: {}", e))
}

/// 获取合像检测配置
#[command]
pub fn get_alignment_config() -> Result<AlignmentConfig, String> {
    load_alignment_config()
        .map_err(|e| format!("加载合像检测配置失败: {}", e))
}

// ============= 配置保存命令 =============

/// 保存标定配置
#[command]
pub fn set_calibration_config(config: CalibrationConfig) -> Result<(), String> {
    save_calibration_config(&config)
        .map_err(|e| format!("保存标定配置失败: {}", e))
}

/// 保存合像检测配置
#[command]
pub fn set_alignment_config(config: AlignmentConfig) -> Result<(), String> {
    save_alignment_config(&config)
        .map_err(|e| format!("保存合像检测配置失败: {}", e))
}

// ============= 配置重置命令 =============

/// 重置标定配置为默认值
#[command]
pub fn reset_calibration_config() -> Result<CalibrationConfig, String> {
    let config = CalibrationConfig::default();
    save_calibration_config(&config)
        .map_err(|e| format!("重置标定配置失败: {}", e))?;
    Ok(config)
}

/// 重置合像检测配置为默认值
#[command]
pub fn reset_alignment_config() -> Result<AlignmentConfig, String> {
    let config = AlignmentConfig::default();
    save_alignment_config(&config)
        .map_err(|e| format!("重置合像检测配置失败: {}", e))?;
    Ok(config)
}

// ============= 相机参数配置结构（供前端使用）=============
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraParams {
    pub frame_rate: f32,
    pub exposure_time: f32,
    pub gain: f32,
    pub left_serial: String,
    pub right_serial: String,
}

/// 获取当前相机参数配置（从文件读取）
#[command]
pub fn get_camera_params() -> Result<CameraParams, String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::collections::HashMap;
    
    let config_path = "configs/camera_params.txt";
    let file = File::open(config_path)
        .map_err(|e| format!("打开相机配置文件失败: {}", e))?;
    
    let reader = BufReader::new(file);
    let mut params = HashMap::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("读取配置行失败: {}", e))?;
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim();
            let value = line[pos + 1..].trim();
            params.insert(key.to_string(), value.to_string());
        }
    }
    
    Ok(CameraParams {
        frame_rate: params.get("frame_rate")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10.0),
        exposure_time: params.get("exposure_time")
            .and_then(|v| v.parse().ok())
            .unwrap_or(60000.0),
        gain: params.get("gain")
            .and_then(|v| v.parse().ok())
            .unwrap_or(10.0),
        left_serial: params.get("left_camera_serial")
            .cloned()
            .unwrap_or_else(|| "DA4347673".to_string()),
        right_serial: params.get("right_camera_serial")
            .cloned()
            .unwrap_or_else(|| "DA4347675".to_string()),
    })
}

/// 保存相机参数配置（写入文件）
#[command]
pub fn set_camera_params(params: CameraParams) -> Result<(), String> {
    use std::fs;
    
    let config_content = format!(
        "# Camera Hardware Configuration\n\
        # Format: key=value\n\
        # Lines starting with # are comments\n\
        \n\
        # Camera Serial Numbers\n\
        left_camera_serial={}\n\
        right_camera_serial={}\n\
        \n\
        # Camera Parameters\n\
        frame_rate={}\n\
        exposure_time={}\n\
        gain={}\n\
        \n\
        # ROI Parameters (reserved for future use)\n\
        # roi_width=2448\n\
        # roi_height=2048\n\
        # roi_offset_x=0\n\
        # roi_offset_y=0",
        params.left_serial,
        params.right_serial,
        params.frame_rate,
        params.exposure_time,
        params.gain
    );
    
    fs::write("configs/camera_params.txt", config_content)
        .map_err(|e| format!("保存相机配置失败: {}", e))?;
    
    println!("相机参数已保存，需要重启相机才能生效");
    Ok(())
} 