// simple_config.rs - 简单的键值对配置管理模块
// 不依赖YAML，使用简单的key=value格式

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 读取键值对配置文件
/// 格式：key=value，支持#注释
pub fn read_config_file(file_path: &str) -> HashMap<String, String> {
    let mut config = HashMap::new();
    
    // 如果文件不存在，返回空配置
    if !Path::new(file_path).exists() {
        println!("配置文件不存在: {}", file_path);
        return config;
    }
    
    // 读取文件内容
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            println!("读取配置文件失败: {}", e);
            return config;
        }
    };
    
    // 解析每一行
    for line in content.lines() {
        let line = line.trim();
        
        // 跳过注释和空行
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        
        // 查找等号
        if let Some(pos) = line.find('=') {
            let key = line[..pos].trim().to_string();
            let value = line[pos + 1..].trim().to_string();
            config.insert(key, value);
        }
    }
    
    config
}

/// 获取浮点数配置值
pub fn get_float_config(config: &HashMap<String, String>, key: &str, default: f32) -> f32 {
    config.get(key)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(default)
}

/// 获取布尔配置值
pub fn get_bool_config(config: &HashMap<String, String>, key: &str, default: bool) -> bool {
    config.get(key)
        .map(|v| v == "true" || v == "1" || v == "yes")
        .unwrap_or(default)
}

/// 获取字符串配置值
pub fn get_string_config(config: &HashMap<String, String>, key: &str, default: &str) -> String {
    config.get(key)
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

// ============= 标定参数配置 =============

/// 加载标定SimpleBlobDetector参数
pub fn load_calibration_blob_params() -> BlobDetectorParams {
    let config = read_config_file("src-tauri/configs/calibration_config.txt");
    
    if config.is_empty() {
        println!("使用默认标定参数");
        return get_default_calibration_params();
    }
    
    BlobDetectorParams {
        min_threshold: get_float_config(&config, "blob_min_threshold", 10.0),
        max_threshold: get_float_config(&config, "blob_max_threshold", 200.0),
        threshold_step: get_float_config(&config, "blob_threshold_step", 10.0),
        filter_by_area: get_bool_config(&config, "filter_by_area", true),
        min_area: get_float_config(&config, "min_area", 1000.0),
        max_area: get_float_config(&config, "max_area", 70000.0),
        filter_by_circularity: get_bool_config(&config, "filter_by_circularity", true),
        min_circularity: get_float_config(&config, "min_circularity", 0.5),
        max_circularity: get_float_config(&config, "max_circularity", 1.0),
        filter_by_convexity: get_bool_config(&config, "filter_by_convexity", true),
        min_convexity: get_float_config(&config, "min_convexity", 0.8),
        max_convexity: get_float_config(&config, "max_convexity", 1.0),
        filter_by_inertia: get_bool_config(&config, "filter_by_inertia", true),
        min_inertia_ratio: get_float_config(&config, "min_inertia_ratio", 0.1),
        max_inertia_ratio: get_float_config(&config, "max_inertia_ratio", 1.0),
    }
}

/// 加载合像检测SimpleBlobDetector参数
pub fn load_alignment_blob_params() -> BlobDetectorParams {
    let config = read_config_file("src-tauri/configs/alignment_config.txt");
    
    if config.is_empty() {
        println!("使用默认合像检测参数");
        return get_default_alignment_params();
    }
    
    BlobDetectorParams {
        min_threshold: get_float_config(&config, "blob_min_threshold", 10.0),
        max_threshold: get_float_config(&config, "blob_max_threshold", 200.0),
        threshold_step: get_float_config(&config, "blob_threshold_step", 20.0), // 注意：合像用20.0
        filter_by_area: get_bool_config(&config, "filter_by_area", true),
        min_area: get_float_config(&config, "min_area", 1000.0),
        max_area: get_float_config(&config, "max_area", 70000.0),
        filter_by_circularity: get_bool_config(&config, "filter_by_circularity", true),
        min_circularity: get_float_config(&config, "min_circularity", 0.6),
        max_circularity: get_float_config(&config, "max_circularity", 1.0),
        filter_by_convexity: get_bool_config(&config, "filter_by_convexity", false), // 性能优化：禁用
        min_convexity: get_float_config(&config, "min_convexity", 0.87),
        max_convexity: get_float_config(&config, "max_convexity", 1.0),
        filter_by_inertia: get_bool_config(&config, "filter_by_inertia", false), // 性能优化：禁用
        min_inertia_ratio: get_float_config(&config, "min_inertia_ratio", 0.1),
        max_inertia_ratio: get_float_config(&config, "max_inertia_ratio", 1.0),
    }
}

/// 相机参数结构
#[derive(Debug, Clone)]
pub struct CameraParams {
    pub frame_rate: f32,
    pub exposure_time: f32,
    pub gain: f32,
    pub left_serial: String,
    pub right_serial: String,
}

/// 加载标定相机参数
pub fn load_calibration_camera_params() -> CameraParams {
    let config = read_config_file("src-tauri/configs/calibration_config.txt");
    
    CameraParams {
        frame_rate: get_float_config(&config, "camera_frame_rate", 5.0),
        exposure_time: get_float_config(&config, "camera_exposure_time", 90000.0),
        gain: get_float_config(&config, "camera_gain", 5.0),
        left_serial: get_string_config(&config, "left_camera_serial", "DA4347673"),
        right_serial: get_string_config(&config, "right_camera_serial", "DA4347675"),
    }
}

/// 加载合像检测相机参数
pub fn load_alignment_camera_params() -> CameraParams {
    let config = read_config_file("src-tauri/configs/alignment_config.txt");
    
    CameraParams {
        frame_rate: get_float_config(&config, "camera_frame_rate", 10.0),
        exposure_time: get_float_config(&config, "camera_exposure_time", 60000.0),
        gain: get_float_config(&config, "camera_gain", 5.0),
        left_serial: get_string_config(&config, "left_camera_serial", "DA4347673"),
        right_serial: get_string_config(&config, "right_camera_serial", "DA4347675"),
    }
}

/// SimpleBlobDetector参数结构
#[derive(Debug, Clone)]
pub struct BlobDetectorParams {
    pub min_threshold: f32,
    pub max_threshold: f32,
    pub threshold_step: f32,
    pub filter_by_area: bool,
    pub min_area: f32,
    pub max_area: f32,
    pub filter_by_circularity: bool,
    pub min_circularity: f32,
    pub max_circularity: f32,
    pub filter_by_convexity: bool,
    pub min_convexity: f32,
    pub max_convexity: f32,
    pub filter_by_inertia: bool,
    pub min_inertia_ratio: f32,
    pub max_inertia_ratio: f32,
}

/// 获取默认标定参数
fn get_default_calibration_params() -> BlobDetectorParams {
    BlobDetectorParams {
        min_threshold: 10.0,
        max_threshold: 200.0,
        threshold_step: 10.0,
        filter_by_area: true,
        min_area: 1000.0,
        max_area: 70000.0,
        filter_by_circularity: true,
        min_circularity: 0.5,
        max_circularity: 1.0,
        filter_by_convexity: true,
        min_convexity: 0.8,
        max_convexity: 1.0,
        filter_by_inertia: true,
        min_inertia_ratio: 0.1,
        max_inertia_ratio: 1.0,
    }
}

/// 获取默认合像检测参数
fn get_default_alignment_params() -> BlobDetectorParams {
    BlobDetectorParams {
        min_threshold: 10.0,
        max_threshold: 200.0,
        threshold_step: 20.0,  // 注意：合像用20.0
        filter_by_area: true,
        min_area: 1000.0,
        max_area: 70000.0,
        filter_by_circularity: true,
        min_circularity: 0.6,
        max_circularity: 1.0,
        filter_by_convexity: false,  // 性能优化：禁用
        min_convexity: 0.87,
        max_convexity: 1.0,
        filter_by_inertia: false,    // 性能优化：禁用
        min_inertia_ratio: 0.1,
        max_inertia_ratio: 1.0,
    }
} 