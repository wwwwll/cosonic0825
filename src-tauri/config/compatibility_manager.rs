use std::collections::HashMap;
use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::config::{ConfigManager, SystemConfig, CameraConfig, AlignmentConfig};

/// 配置预设
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPreset {
    pub name: String,
    pub description: String,
    pub system: SystemConfig,
    pub camera: CameraConfig,
    pub alignment: AlignmentConfig,
    pub created_at: String,
    pub version: String,
    pub preset_type: String,  // "builtin" or "user"
}

/// 兼容性管理器 - 处理配置预设和现有代码兼容性
pub struct CompatibilityManager {
    presets: HashMap<String, ConfigPreset>,
    config_dir: String,
}

impl CompatibilityManager {
    pub fn new(config_dir: &str) -> Self {
        let mut manager = Self {
            presets: HashMap::new(),
            config_dir: config_dir.to_string(),
        };
        
        // 加载内置预设
        manager.load_builtin_presets();
        
        // 加载用户自定义预设
        if let Err(e) = manager.load_user_presets() {
            println!("⚠️ 加载用户预设失败: {}", e);
        }
        
        manager
    }
    
    /// 加载内置预设
    fn load_builtin_presets(&mut self) {
        // 生产环境预设 - 完全使用legacy实现
        let production_preset = ConfigPreset {
            name: "生产环境".to_string(),
            description: "适合生产环境的优化配置，完全使用现有legacy实现".to_string(),
            system: SystemConfig {
                pattern_layout: crate::config::PatternLayoutConfig {
                    use_legacy_coordinates: true,  // 强制使用legacy
                    pattern_type: "asymmetric_circles_grid".to_string(),
                    pattern_width: 10,
                    pattern_height: 4,
                    circle_diameter: 15.0,
                    diagonal_spacing: 25.0,
                    legacy_world_coords_comment: "生产环境：强制使用calibration_circles.rs中的原有世界坐标".to_string(),
                    legacy_params_location: "src-tauri/src/modules/calibration_circles.rs:generate_world_points_from_list".to_string(),
                },
                file_paths: crate::config::FilePathConfig {
                    camera_config_dir: "configs/camera/".to_string(),
                    calibration_images_dir: "calibration_images/".to_string(),
                    calibration_params_dir: "./".to_string(),
                    rectify_maps_path: "rectify_maps.yaml".to_string(),
                    alignment_config_dir: "configs/alignment/".to_string(),
                    use_param_io_paths: true,  // 强制使用现有路径
                    left_camera_params_path: "left_camera_params.yaml".to_string(),
                    right_camera_params_path: "right_camera_params.yaml".to_string(),
                    stereo_params_path: "stereo_params.yaml".to_string(),
                    rectify_params_path: "rectify_params.yaml".to_string(),
                },
                camera_serials: crate::config::CameraSerialConfig {
                    left_camera_serial: "DA5158733".to_string(),
                    right_camera_serial: "DA5158736".to_string(),
                    auto_detect_serials: false,
                    legacy_serial_location: "src-tauri/camera_sdk/include/camera_api.h:29-30".to_string(),
                },
                version: "1.0".to_string(),
                created_at: "2025-01-15T00:00:00Z".to_string(),
            },
            camera: CameraConfig {
                acquisition_frame_rate_enable: true,
                acquisition_frame_rate: 15.0,        // 生产环境优化帧率
                exposure_time: 0.0,                  // 使用相机默认
                exposure_auto: 0,
                gain: 0.0,                          // 无增益
                gain_auto: 0,
                trigger_mode: 0,
                trigger_source: "Software".to_string(),
                width: 2448,
                height: 2048,
                roi: crate::config::RoiConfig {
                    offset_x: 0,
                    offset_y: 0,
                    width: 2448,
                    height: 2048,
                    enabled: false,
                    applies_to_both_cameras: true,
                },
                calibration_blob_detector: crate::config::BlobDetectorConfig {
                    use_legacy_params: true,
                    min_threshold: 10.0,
                    max_threshold: 200.0,
                    threshold_step: 10.0,
                    min_area: 50.0,
                    max_area: 5000.0,
                    filter_by_circularity: true,
                    min_circularity: 0.5,
                    filter_by_convexity: true,
                    min_convexity: 0.8,
                    filter_by_inertia: true,
                    min_inertia_ratio: 0.1,
                    filter_by_color: false,
                    blob_color: 0,
                    legacy_params_comment: "保留calibration_circles.rs中的原有SimpleBlobDetector参数".to_string(),
                    legacy_params_location: "src-tauri/src/modules/calibration_circles.rs:49-80".to_string(),
                },
                left_camera_serial: "DA5158733".to_string(),
                right_camera_serial: "DA5158736".to_string(),
                use_legacy_camera_init: true,        // 强制使用legacy
                legacy_init_location: "src-tauri/camera_sdk/src/camera_init.c:196-218".to_string(),
            },
            alignment: AlignmentConfig {
                pattern_layout_ref: "system_config.pattern_layout".to_string(),
                use_system_pattern_layout: true,
                alignment_blob_detector: crate::config::AlignmentBlobDetectorConfig {
                    use_legacy_alignment_params: true,  // 强制使用legacy
                    min_threshold: 100.0,
                    max_threshold: 200.0,
                    threshold_step: 50.0,
                    min_area: 100.0,
                    max_area: 8000.0,
                    filter_by_circularity: true,
                    min_circularity: 0.6,
                    filter_by_convexity: false,  // 性能优化
                    min_convexity: 0.87,
                    filter_by_inertia: false,    // 性能优化
                    min_inertia_ratio: 0.1,
                    filter_by_color: false,
                    legacy_params_location: "src-tauri/src/modules/alignment.rs:create_optimized_blob_detector".to_string(),
                    optimization_notes: "生产环境：使用alignment.rs中优化后的参数".to_string(),
                },
                pose_thresholds: crate::config::PoseThresholds {
                    use_legacy_pose_thresholds: true,
                    left_eye_max_roll: 5.0,
                    left_eye_max_pitch: 10.0,
                    left_eye_max_yaw: 10.0,
                    left_eye_max_translation: 10.0,
                    right_eye_max_roll: 5.0,
                    right_eye_max_pitch: 10.0,
                    right_eye_max_yaw: 10.0,
                    legacy_thresholds_location: "src-tauri/src/modules/alignment.rs:17-21".to_string(),
                },
                alignment_thresholds: crate::config::AlignmentThresholds {
                    use_legacy_alignment_thresholds: true,
                    max_rms_error: 100.0,
                    max_p95_error: 100.0,
                    max_max_error: 200.0,
                    adjustment_hint_threshold: 1.0,
                    mean_dx_threshold: 0.5,
                    mean_dy_threshold: 0.5,
                    legacy_thresholds_location: "src-tauri/src/modules/alignment.rs:19-21".to_string(),
                },
                roi_config: crate::config::AlignmentRoiConfig {
                    right_roi_enabled: true,         // 启用ROI优化
                    right_roi_x: 900,
                    right_roi_y: 0,
                    right_roi_width: 1548,
                    right_roi_height: 1250,
                    left_roi_enabled: false,
                    left_roi_x: 0,
                    left_roi_y: 0,
                    left_roi_width: 2448,
                    left_roi_height: 2048,
                    roi_optimization_notes: "生产环境：启用右相机ROI以提升50%性能".to_string(),
                },
                use_legacy_alignment_params: true,   // 强制使用legacy
                legacy_params_location: "src-tauri/src/modules/alignment.rs".to_string(),
            },
            created_at: "2025-01-15T00:00:00Z".to_string(),
            version: "1.0".to_string(),
            preset_type: "builtin".to_string(),
        };
        
        // 调试环境预设 - 使用legacy实现但参数更宽松
        let debug_preset = ConfigPreset {
            name: "调试环境".to_string(),
            description: "适合调试和开发的配置，参数更宽松便于测试".to_string(),
            system: production_preset.system.clone(),
            camera: CameraConfig {
                acquisition_frame_rate: 5.0,        // 低帧率便于调试
                ..production_preset.camera.clone()
            },
            alignment: AlignmentConfig {
                pose_thresholds: crate::config::PoseThresholds {
                    use_legacy_pose_thresholds: true,  // 使用legacy但更宽松
                    left_eye_max_roll: 10.0,          // 更宽松的姿态要求
                    left_eye_max_pitch: 15.0,
                    left_eye_max_yaw: 15.0,
                    left_eye_max_translation: 20.0,
                    right_eye_max_roll: 10.0,
                    right_eye_max_pitch: 15.0,
                    right_eye_max_yaw: 15.0,
                    legacy_thresholds_location: "src-tauri/src/modules/alignment.rs:17-21".to_string(),
                },
                alignment_thresholds: crate::config::AlignmentThresholds {
                    use_legacy_alignment_thresholds: true,  // 使用legacy但更宽松
                    max_rms_error: 150.0,             // 更宽松的合像要求
                    max_p95_error: 200.0,
                    max_max_error: 300.0,
                    adjustment_hint_threshold: 2.0,
                    mean_dx_threshold: 1.0,
                    mean_dy_threshold: 1.0,
                    legacy_thresholds_location: "src-tauri/src/modules/alignment.rs:19-21".to_string(),
                },
                roi_config: crate::config::AlignmentRoiConfig {
                    right_roi_enabled: false,        // 调试时不启用ROI，全图检测
                    right_roi_x: 0,
                    right_roi_y: 0,
                    right_roi_width: 2448,
                    right_roi_height: 2048,
                    left_roi_enabled: false,
                    left_roi_x: 0,
                    left_roi_y: 0,
                    left_roi_width: 2448,
                    left_roi_height: 2048,
                    roi_optimization_notes: "调试环境：禁用ROI以便观察完整图像".to_string(),
                },
                ..production_preset.alignment.clone()
            },
            created_at: "2025-01-15T00:00:00Z".to_string(),
            version: "1.0".to_string(),
            preset_type: "builtin".to_string(),
        };
        
        // 高级配置预设 - 允许部分非legacy配置
        let advanced_preset = ConfigPreset {
            name: "高级配置".to_string(),
            description: "高级用户配置，允许部分自定义参数但保持核心legacy实现".to_string(),
            system: production_preset.system.clone(),
            camera: CameraConfig {
                acquisition_frame_rate: 10.0,       // 中等帧率
                exposure_time: 8000.0,              // 允许自定义曝光时间
                gain: 2.0,                         // 允许轻微增益
                use_legacy_camera_init: false,      // 允许非legacy相机初始化
                ..production_preset.camera.clone()
            },
            alignment: AlignmentConfig {
                use_legacy_alignment_params: true,  // 核心算法仍使用legacy
                legacy_params_location: "高级配置：核心算法使用legacy，但允许调整阈值".to_string(),
                ..production_preset.alignment.clone()
            },
            created_at: "2025-01-15T00:00:00Z".to_string(),
            version: "1.0".to_string(),
            preset_type: "builtin".to_string(),
        };
        
        self.presets.insert("production".to_string(), production_preset);
        self.presets.insert("debug".to_string(), debug_preset);
        self.presets.insert("advanced".to_string(), advanced_preset);
        
        println!("✓ 加载了 {} 个内置配置预设", self.presets.len());
    }
    
    /// 加载用户自定义预设
    fn load_user_presets(&mut self) -> Result<(), String> {
        let presets_dir = Path::new(&self.config_dir).join("presets");
        if !presets_dir.exists() {
            return Ok(());
        }
        
        let mut user_preset_count = 0;
        for entry in fs::read_dir(&presets_dir)
            .map_err(|e| format!("读取预设目录失败: {}", e))? 
        {
            let entry = entry.map_err(|e| format!("读取预设文件失败: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml") {
                match self.load_preset_from_file(&path) {
                    Ok(preset) => {
                        let preset_name = preset.name.clone();
                        self.presets.insert(preset_name.clone(), preset);
                        user_preset_count += 1;
                        println!("✓ 加载用户预设: {}", preset_name);
                    },
                    Err(e) => {
                        println!("⚠️ 加载预设文件失败 {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        if user_preset_count > 0 {
            println!("✓ 加载了 {} 个用户自定义预设", user_preset_count);
        }
        
        Ok(())
    }
    
    /// 从文件加载单个预设
    fn load_preset_from_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ConfigPreset, String> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("读取预设文件失败: {}", e))?;
            
        let mut preset: ConfigPreset = serde_yaml::from_str(&content)
            .map_err(|e| format!("解析预设文件失败: {}", e))?;
            
        preset.preset_type = "user".to_string();  // 标记为用户预设
        Ok(preset)
    }
    
    /// 获取预设
    pub fn get_preset(&self, name: &str) -> Option<&ConfigPreset> {
        self.presets.get(name)
    }
    
    /// 列出所有预设名称
    pub fn list_presets(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }
    
    /// 列出内置预设
    pub fn list_builtin_presets(&self) -> Vec<String> {
        self.presets.iter()
            .filter(|(_, preset)| preset.preset_type == "builtin")
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// 列出用户预设
    pub fn list_user_presets(&self) -> Vec<String> {
        self.presets.iter()
            .filter(|(_, preset)| preset.preset_type == "user")
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    /// 保存用户预设
    pub fn save_user_preset(&mut self, preset: ConfigPreset) -> Result<(), String> {
        let presets_dir = Path::new(&self.config_dir).join("presets");
        fs::create_dir_all(&presets_dir)
            .map_err(|e| format!("创建预设目录失败: {}", e))?;
            
        let file_path = presets_dir.join(format!("{}.yaml", preset.name));
        
        let mut preset_to_save = preset.clone();
        preset_to_save.preset_type = "user".to_string();
        preset_to_save.created_at = chrono::Utc::now().to_rfc3339();
        
        let content = serde_yaml::to_string(&preset_to_save)
            .map_err(|e| format!("序列化预设失败: {}", e))?;
            
        fs::write(&file_path, content)
            .map_err(|e| format!("保存预设文件失败: {}", e))?;
            
        self.presets.insert(preset.name.clone(), preset_to_save);
        println!("✓ 用户预设已保存: {}", preset.name);
        Ok(())
    }
    
    /// 应用预设到配置管理器
    pub fn apply_preset_to_manager(&self, preset_name: &str, manager: &mut ConfigManager) -> Result<(), String> {
        let preset = self.get_preset(preset_name)
            .ok_or_else(|| format!("预设不存在: {}", preset_name))?;
            
        manager.system_config = preset.system.clone();
        manager.camera_config = preset.camera.clone();
        manager.alignment_config = preset.alignment.clone();
        
        // 根据预设类型设置保护模式
        manager.preserve_existing_implementations = match preset_name {
            "production" | "debug" => true,   // 内置预设强制保护
            "advanced" => false,              // 高级预设允许部分修改
            _ => true,                        // 用户预设默认保护
        };
        
        println!("✓ 已应用预设 '{}' 到配置管理器", preset_name);
        Ok(())
    }
    
    /// 从配置管理器创建预设
    pub fn create_preset_from_manager(&self, name: String, description: String, manager: &ConfigManager) -> ConfigPreset {
        ConfigPreset {
            name,
            description,
            system: manager.system_config.clone(),
            camera: manager.camera_config.clone(),
            alignment: manager.alignment_config.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
            preset_type: "user".to_string(),
        }
    }
    
    /// 生成兼容性报告
    pub fn generate_compatibility_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== 兼容性管理器报告 ===\n\n");
        
        // 预设统计
        let builtin_count = self.list_builtin_presets().len();
        let user_count = self.list_user_presets().len();
        report.push_str(&format!("📋 配置预设统计:\n"));
        report.push_str(&format!("  - 内置预设: {} 个\n", builtin_count));
        report.push_str(&format!("  - 用户预设: {} 个\n", user_count));
        report.push_str(&format!("  - 总计: {} 个\n", self.presets.len()));
        
        // 内置预设详情
        report.push_str("\n🔧 内置预设详情:\n");
        for preset_name in self.list_builtin_presets() {
            if let Some(preset) = self.get_preset(&preset_name) {
                report.push_str(&format!("  - {}: {}\n", preset.name, preset.description));
                report.push_str(&format!("    Legacy保护: 系统={}, 相机={}, 合像={}\n",
                    preset.system.pattern_layout.use_legacy_coordinates,
                    preset.camera.use_legacy_camera_init,
                    preset.alignment.use_legacy_alignment_params));
            }
        }
        
        // 用户预设详情
        if user_count > 0 {
            report.push_str("\n👤 用户预设详情:\n");
            for preset_name in self.list_user_presets() {
                if let Some(preset) = self.get_preset(&preset_name) {
                    report.push_str(&format!("  - {}: {}\n", preset.name, preset.description));
                }
            }
        }
        
        report.push_str("\n=== 报告结束 ===\n");
        report
    }
} 