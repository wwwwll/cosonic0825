use tauri::State;
use std::sync::{Arc, Mutex};
use crate::config::{ConfigManager, SystemConfig, CameraConfig, AlignmentConfig, CompatibilityManager, ConfigPreset};

/// 系统参数配置命令
#[tauri::command]
pub async fn get_system_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<SystemConfig, String> {
    let manager = config_manager.lock().unwrap();
    Ok(manager.system_config.clone())
}

#[tauri::command]
pub async fn set_system_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    config: SystemConfig,
) -> Result<(), String> {
    let mut manager = config_manager.lock().unwrap();
    
    // 验证配置有效性
    config.validate()?;
    
    manager.system_config = config;
    println!("✓ 系统配置已更新");
    Ok(())
}

/// 相机参数配置命令 - 统一管理左右两个相机
#[tauri::command]
pub async fn get_camera_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<CameraConfig, String> {
    let manager = config_manager.lock().unwrap();
    Ok(manager.camera_config.clone())
}

#[tauri::command]
pub async fn set_camera_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    config: CameraConfig,
) -> Result<(), String> {
    let mut manager = config_manager.lock().unwrap();
    
    // 验证配置有效性
    config.validate()?;
    
    // ⚠️ 谨慎应用配置到硬件 - 默认绕过现有实现
    manager.apply_camera_config(0, &config)?;  // 左相机
    manager.apply_camera_config(1, &config)?;  // 右相机
    
    // 保存配置到内存
    manager.camera_config = config;
    
    println!("✓ 相机配置已更新 (左右相机统一配置)");
    Ok(())
}

/// 获取单个相机的序列号 - 兼容旧接口
#[tauri::command]
pub async fn get_camera_serial(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    camera_side: String, // "left" or "right"
) -> Result<String, String> {
    let manager = config_manager.lock().unwrap();
    let (left_serial, right_serial) = manager.camera_config.get_camera_serials();
    
    match camera_side.as_str() {
        "left" => Ok(left_serial),
        "right" => Ok(right_serial),
        _ => Err("无效的相机侧别，应为 'left' 或 'right'".to_string()),
    }
}

/// 合像参数配置命令
#[tauri::command]
pub async fn get_alignment_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<AlignmentConfig, String> {
    let manager = config_manager.lock().unwrap();
    Ok(manager.alignment_config.clone())
}

#[tauri::command]
pub async fn set_alignment_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    config: AlignmentConfig,
) -> Result<(), String> {
    let mut manager = config_manager.lock().unwrap();
    
    // 验证配置有效性
    config.validate()?;
    
    manager.alignment_config = config;
    
    // ⚠️ 重要：不直接修改alignment.rs中的写死参数
    println!("🔄 合像参数配置已保存，但未应用到alignment.rs (保护现有实现)");
    println!("   如需应用，请检查use_legacy_alignment_params标志");
    Ok(())
}

/// 配置文件管理命令
#[tauri::command]
pub async fn save_config_to_file(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    file_path: String,
) -> Result<(), String> {
    let manager = config_manager.lock().unwrap();
    
    // 验证所有配置
    manager.validate_all()?;
    
    manager.save_to_file(&file_path)
}

#[tauri::command]
pub async fn load_config_from_file(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    file_path: String,
) -> Result<(), String> {
    let loaded_manager = ConfigManager::load_from_file(&file_path)?;
    
    // 验证加载的配置
    loaded_manager.validate_all()?;
    
    // 替换当前配置管理器的内容
    let mut manager = config_manager.lock().unwrap();
    manager.system_config = loaded_manager.system_config;
    manager.camera_config = loaded_manager.camera_config;
    manager.alignment_config = loaded_manager.alignment_config;
    manager.config_root_dir = loaded_manager.config_root_dir;
    
    // ⚠️ 谨慎应用加载的配置到硬件
    if !manager.preserve_existing_implementations {
        println!("📝 TODO: 应用加载的配置到硬件");
        // manager.apply_camera_config(0, &manager.camera_config.clone())?;
        // manager.apply_camera_config(1, &manager.camera_config.clone())?;
    } else {
        println!("🔄 保护模式：配置已加载但未应用到硬件");
    }
    
    println!("✓ 配置文件已加载: {}", file_path);
    Ok(())
}

#[tauri::command]
pub async fn save_config_to_default_dir(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(), String> {
    let manager = config_manager.lock().unwrap();
    manager.save_to_default_dir()
}

#[tauri::command]
pub async fn list_config_files(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<Vec<String>, String> {
    let manager = config_manager.lock().unwrap();
    manager.list_config_files()
}

/// 配置验证命令
#[tauri::command]
pub async fn validate_all_configs(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(), String> {
    let manager = config_manager.lock().unwrap();
    manager.validate_all()
}

/// 配置报告命令
#[tauri::command]
pub async fn generate_config_report(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<String, String> {
    let manager = config_manager.lock().unwrap();
    Ok(manager.generate_config_report())
}

/// 获取当前有效参数命令
#[tauri::command]
pub async fn get_effective_pattern_params(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(f32, f32, (i32, i32)), String> {
    let manager = config_manager.lock().unwrap();
    let (diameter, spacing, size) = manager.get_effective_pattern_params();
    Ok((diameter, spacing, (size.width, size.height)))
}

#[tauri::command]
pub async fn get_effective_camera_serials(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(String, String), String> {
    let manager = config_manager.lock().unwrap();
    Ok(manager.get_effective_camera_serials())
}

#[tauri::command]
pub async fn should_use_legacy_implementations(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<bool, String> {
    let manager = config_manager.lock().unwrap();
    Ok(manager.should_use_legacy_implementations())
}

/// ROI配置预览命令 - 支持拖拽设定ROI
#[tauri::command]
pub async fn get_camera_preview_for_roi(
    camera_side: String,
) -> Result<String, String> {
    // 获取单帧图像用于ROI设定
    // 返回Base64编码的压缩图像
    println!("📝 TODO: 实现相机预览图像获取用于ROI设定");
    println!("   相机侧别: {}", camera_side);
    println!("   需要实现: camera_capture_single_frame_ffi(cam_index) -> base64_image");
    
    // 当前返回占位符
    Ok("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string())
}

#[tauri::command]
pub async fn apply_roi_config(
    camera_side: String,
    roi_config: crate::config::RoiConfig,
) -> Result<(), String> {
    println!("📝 TODO: 实现ROI配置应用到硬件");
    println!("   相机侧别: {}", camera_side);
    println!("   ROI区域: x={}, y={}, w={}, h={}", 
        roi_config.offset_x, roi_config.offset_y, roi_config.width, roi_config.height);
    println!("   需要实现: camera_set_roi_ffi(cam_index, roi_config)");
    
    Ok(())
}

/// 配置预设管理命令
#[tauri::command]
pub async fn list_config_presets(
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
) -> Result<Vec<String>, String> {
    let manager = compatibility_manager.lock().unwrap();
    Ok(manager.list_presets())
}

#[tauri::command]
pub async fn list_builtin_presets(
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
) -> Result<Vec<String>, String> {
    let manager = compatibility_manager.lock().unwrap();
    Ok(manager.list_builtin_presets())
}

#[tauri::command]
pub async fn list_user_presets(
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
) -> Result<Vec<String>, String> {
    let manager = compatibility_manager.lock().unwrap();
    Ok(manager.list_user_presets())
}

#[tauri::command]
pub async fn get_config_preset(
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
    preset_name: String,
) -> Result<ConfigPreset, String> {
    let manager = compatibility_manager.lock().unwrap();
    manager.get_preset(&preset_name)
        .cloned()
        .ok_or_else(|| format!("预设不存在: {}", preset_name))
}

#[tauri::command]
pub async fn apply_config_preset(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
    preset_name: String,
) -> Result<(), String> {
    let compat_manager = compatibility_manager.lock().unwrap();
    let mut config_manager = config_manager.lock().unwrap();
    
    compat_manager.apply_preset_to_manager(&preset_name, &mut config_manager)?;
    
    println!("✓ 已应用配置预设: {}", preset_name);
    Ok(())
}

#[tauri::command]
pub async fn save_config_preset(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
    preset_name: String,
    description: String,
) -> Result<(), String> {
    let config_manager = config_manager.lock().unwrap();
    let mut compat_manager = compatibility_manager.lock().unwrap();
    
    let preset = compat_manager.create_preset_from_manager(preset_name, description, &config_manager);
    compat_manager.save_user_preset(preset)?;
    
    println!("✓ 用户配置预设已保存");
    Ok(())
}

#[tauri::command]
pub async fn generate_compatibility_report(
    compatibility_manager: State<'_, Arc<Mutex<CompatibilityManager>>>,
) -> Result<String, String> {
    let manager = compatibility_manager.lock().unwrap();
    Ok(manager.generate_compatibility_report())
}

/// 硬件状态读取命令 (预留接口)
#[tauri::command]
pub async fn load_current_hardware_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    cam_index: u32,
) -> Result<(), String> {
    let mut manager = config_manager.lock().unwrap();
    manager.load_current_hardware_config(cam_index)
}

/// 配置重置命令
#[tauri::command]
pub async fn reset_to_default_config(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<(), String> {
    let mut manager = config_manager.lock().unwrap();
    
    // 重置为默认配置
    let default_manager = ConfigManager::new();
    manager.system_config = default_manager.system_config;
    manager.camera_config = default_manager.camera_config;
    manager.alignment_config = default_manager.alignment_config;
    manager.preserve_existing_implementations = true;  // 强制保护现有实现
    
    println!("✓ 已重置为默认配置 (保护现有实现)");
    Ok(())
}

/// 配置导出/导入命令 (预留接口)
#[tauri::command]
pub async fn export_config_to_json(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
) -> Result<String, String> {
    let manager = config_manager.lock().unwrap();
    
    let config_data = crate::config::ConfigData {
        system: manager.system_config.clone(),
        camera: manager.camera_config.clone(),
        alignment: manager.alignment_config.clone(),
        version: "1.0".to_string(),
        created_at: manager.system_config.created_at.clone(),
        last_modified: chrono::Utc::now().to_rfc3339(),
    };
    
    serde_json::to_string_pretty(&config_data)
        .map_err(|e| format!("导出配置失败: {}", e))
}

#[tauri::command]
pub async fn import_config_from_json(
    config_manager: State<'_, Arc<Mutex<ConfigManager>>>,
    json_data: String,
) -> Result<(), String> {
    let config_data: crate::config::ConfigData = serde_json::from_str(&json_data)
        .map_err(|e| format!("解析配置JSON失败: {}", e))?;
    
    let mut manager = config_manager.lock().unwrap();
    manager.system_config = config_data.system;
    manager.camera_config = config_data.camera;
    manager.alignment_config = config_data.alignment;
    
    // 验证导入的配置
    manager.validate_all()?;
    
    println!("✓ 配置已从JSON导入");
    Ok(())
} 

// ============= 简单配置管理命令 =============

/// 读取配置文件内容
#[tauri::command]
pub fn read_config_file(config_type: String) -> Result<String, String> {
    let file_path = match config_type.as_str() {
        "calibration" => "src-tauri/configs/calibration_config.txt",
        "alignment" => "src-tauri/configs/alignment_config.txt",
        "default" => "src-tauri/configs/default_config.txt",
        _ => return Err("无效的配置类型".to_string()),
    };
    
    std::fs::read_to_string(file_path)
        .map_err(|e| format!("读取配置文件失败: {}", e))
}

/// 保存配置文件内容
#[tauri::command]
pub fn write_config_file(config_type: String, content: String) -> Result<(), String> {
    let file_path = match config_type.as_str() {
        "calibration" => "src-tauri/configs/calibration_config.txt",
        "alignment" => "src-tauri/configs/alignment_config.txt",
        "default" => "src-tauri/configs/default_config.txt",
        _ => return Err("无效的配置类型".to_string()),
    };
    
    std::fs::write(file_path, content)
        .map_err(|e| format!("保存配置文件失败: {}", e))?;
    
    println!("配置文件已保存: {}", file_path);
    Ok(())
}

/// 获取当前配置参数（解析后的）
#[tauri::command]
pub fn get_current_config(config_type: String) -> Result<serde_json::Value, String> {
    use crate::modules::simple_config;
    use serde_json::json;
    
    match config_type.as_str() {
        "calibration" => {
            let camera_params = simple_config::load_calibration_camera_params();
            let blob_params = simple_config::load_calibration_blob_params();
            
            Ok(json!({
                "camera": {
                    "frame_rate": camera_params.frame_rate,
                    "exposure_time": camera_params.exposure_time,
                    "gain": camera_params.gain,
                    "left_serial": camera_params.left_serial,
                    "right_serial": camera_params.right_serial,
                },
                "blob_detector": {
                    "min_threshold": blob_params.min_threshold,
                    "max_threshold": blob_params.max_threshold,
                    "threshold_step": blob_params.threshold_step,
                    "filter_by_area": blob_params.filter_by_area,
                    "min_area": blob_params.min_area,
                    "max_area": blob_params.max_area,
                    "filter_by_circularity": blob_params.filter_by_circularity,
                    "min_circularity": blob_params.min_circularity,
                    "max_circularity": blob_params.max_circularity,
                    "filter_by_convexity": blob_params.filter_by_convexity,
                    "min_convexity": blob_params.min_convexity,
                    "max_convexity": blob_params.max_convexity,
                    "filter_by_inertia": blob_params.filter_by_inertia,
                    "min_inertia_ratio": blob_params.min_inertia_ratio,
                    "max_inertia_ratio": blob_params.max_inertia_ratio,
                }
            }))
        },
        "alignment" => {
            let camera_params = simple_config::load_alignment_camera_params();
            let blob_params = simple_config::load_alignment_blob_params();
            
            Ok(json!({
                "camera": {
                    "frame_rate": camera_params.frame_rate,
                    "exposure_time": camera_params.exposure_time,
                    "gain": camera_params.gain,
                    "left_serial": camera_params.left_serial,
                    "right_serial": camera_params.right_serial,
                },
                "blob_detector": {
                    "min_threshold": blob_params.min_threshold,
                    "max_threshold": blob_params.max_threshold,
                    "threshold_step": blob_params.threshold_step,
                    "filter_by_area": blob_params.filter_by_area,
                    "min_area": blob_params.min_area,
                    "max_area": blob_params.max_area,
                    "filter_by_circularity": blob_params.filter_by_circularity,
                    "min_circularity": blob_params.min_circularity,
                    "max_circularity": blob_params.max_circularity,
                    "filter_by_convexity": blob_params.filter_by_convexity,
                    "min_convexity": blob_params.min_convexity,
                    "max_convexity": blob_params.max_convexity,
                    "filter_by_inertia": blob_params.filter_by_inertia,
                    "min_inertia_ratio": blob_params.min_inertia_ratio,
                    "max_inertia_ratio": blob_params.max_inertia_ratio,
                }
            }))
        },
        _ => Err("无效的配置类型".to_string())
    }
}

// ============= 原有配置管理命令 ============= 