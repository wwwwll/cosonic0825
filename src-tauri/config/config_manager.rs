use std::path::Path;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::config::{SystemConfig, CameraConfig, AlignmentConfig};

/// 配置管理器 - 负责所有配置的统一管理
pub struct ConfigManager {
    /// 系统配置
    pub system_config: SystemConfig,
    
    /// 相机配置 - 统一管理左右两个相机
    pub camera_config: CameraConfig,
    
    /// 合像配置
    pub alignment_config: AlignmentConfig,
    
    /// 保护现有实现的标志
    pub preserve_existing_implementations: bool,
    
    /// 配置文件根目录
    pub config_root_dir: String,
}

/// 完整的配置数据结构 - 用于序列化保存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    pub system: SystemConfig,
    pub camera: CameraConfig,
    pub alignment: AlignmentConfig,
    pub version: String,
    pub created_at: String,
    pub last_modified: String,
}

impl ConfigManager {
    /// 创建新的配置管理器实例
    pub fn new() -> Self {
        let system_config = SystemConfig::new();
        let (left_serial, right_serial) = system_config.get_effective_camera_serials();
        
        Self {
            camera_config: CameraConfig::new_with_serials(&left_serial, &right_serial),
            alignment_config: AlignmentConfig::new(),
            system_config,
            preserve_existing_implementations: true,  // 默认保护现有代码
            config_root_dir: "configs".to_string(),
        }
    }
    
    /// 从配置文件目录加载配置管理器
    pub fn load_from_dir<P: AsRef<Path>>(config_dir: P) -> Result<Self, String> {
        let config_path = config_dir.as_ref().join("system_config.yaml");
        
        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            // 配置文件不存在，创建默认配置
            let mut manager = Self::new();
            manager.config_root_dir = config_dir.as_ref().to_string_lossy().to_string();
            Ok(manager)
        }
    }
    
    /// 从单个配置文件加载完整配置
    pub fn load_from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, String> {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
            
        let config_data: ConfigData = serde_yaml::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;
            
        let config_dir = file_path.as_ref().parent()
            .unwrap_or(Path::new("."))
            .to_string_lossy()
            .to_string();
            
        Ok(Self {
            system_config: config_data.system,
            camera_config: config_data.camera,
            alignment_config: config_data.alignment,
            preserve_existing_implementations: true,  // 始终保护现有实现
            config_root_dir: config_dir,
        })
    }
    
    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, file_path: P) -> Result<(), String> {
        let config_data = ConfigData {
            system: self.system_config.clone(),
            camera: self.camera_config.clone(),
            alignment: self.alignment_config.clone(),
            version: "1.0".to_string(),
            created_at: self.system_config.created_at.clone(),
            last_modified: chrono::Utc::now().to_rfc3339(),
        };
        
        let content = serde_yaml::to_string(&config_data)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
            
        // 确保目录存在
        if let Some(parent) = file_path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
            
        fs::write(&file_path, content)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;
            
        println!("✓ 配置已保存到: {}", file_path.as_ref().display());
        Ok(())
    }
    
    /// 保存配置到默认目录
    pub fn save_to_default_dir(&self) -> Result<(), String> {
        let config_path = Path::new(&self.config_root_dir).join("system_config.yaml");
        self.save_to_file(config_path)
    }
    
    /// 验证所有配置的有效性
    pub fn validate_all(&self) -> Result<(), String> {
        // 验证系统配置
        self.system_config.validate()
            .map_err(|e| format!("系统配置验证失败: {}", e))?;
        
        // 验证相机配置
        self.camera_config.validate()
            .map_err(|e| format!("相机配置验证失败: {}", e))?;
        
        // 验证合像配置
        self.alignment_config.validate()
            .map_err(|e| format!("合像配置验证失败: {}", e))?;
        
        Ok(())
    }
    
    /// ⚠️ 应用相机配置到硬件 - 谨慎操作，默认绕过现有实现
    pub fn apply_camera_config(&self, cam_index: u32, config: &CameraConfig) -> Result<(), String> {
        // 检查是否需要绕过现有实现
        if self.preserve_existing_implementations || config.use_legacy_camera_init {
            println!("🔄 保护模式：绕过现有camera_init.c中的硬件设置");
            println!("   相机{}配置已保存但未应用到硬件", cam_index);
            println!("   原因: 保护现有camera_init.c实现 ({})", config.legacy_init_location);
            return Ok(());
        }
        
        // ⚠️ 注意：这些FFI函数可能不存在，需要先实现
        println!("📝 TODO: 实现相机配置应用到硬件");
        println!("   需要实现的FFI函数:");
        println!("   - camera_set_exposure_time_ffi(cam_index, exposure_time)");
        println!("   - camera_set_gain_ffi(cam_index, gain)"); 
        println!("   - camera_set_roi_ffi(cam_index, roi_config)");
        
        // 当前只记录配置，不实际应用
        println!("🔄 相机{}配置记录完成，等待FFI接口实现", cam_index);
        Ok(())
    }
    
    /// 从现有代码读取当前硬件配置状态
    pub fn load_current_hardware_config(&mut self, cam_index: u32) -> Result<(), String> {
        println!("📝 TODO: 从硬件读取当前配置状态");
        println!("   可以从以下位置读取:");
        println!("   - camera_api.h: 相机序列号定义");
        println!("   - camera_init.c: 当前硬件参数设置");
        println!("   - camera_status.c: 运行时状态监控");
        
        // 当前使用默认配置
        println!("🔄 使用默认配置作为当前硬件状态");
        Ok(())
    }
    
    /// 获取当前有效的标定板参数 (优先使用legacy实现)
    pub fn get_effective_pattern_params(&self) -> (f32, f32, opencv::core::Size) {
        self.system_config.get_effective_pattern_params()
    }
    
    /// 获取当前有效的相机序列号
    pub fn get_effective_camera_serials(&self) -> (String, String) {
        self.camera_config.get_camera_serials()
    }
    
    /// 检查是否应该使用legacy实现
    pub fn should_use_legacy_implementations(&self) -> bool {
        self.preserve_existing_implementations &&
        self.system_config.pattern_layout.use_legacy_coordinates &&
        self.camera_config.use_legacy_camera_init &&
        self.alignment_config.use_legacy_alignment_params
    }
    
    /// 生成配置报告
    pub fn generate_config_report(&self) -> String {
        let mut report = String::new();
        report.push_str("=== 参数配置系统报告 ===\n\n");
        
        // 系统配置报告
        report.push_str("📋 系统配置:\n");
        report.push_str(&format!("  - 标定板类型: {}\n", self.system_config.pattern_layout.pattern_type));
        report.push_str(&format!("  - 标定板尺寸: {}×{}\n", 
            self.system_config.pattern_layout.pattern_width,
            self.system_config.pattern_layout.pattern_height));
        report.push_str(&format!("  - 使用legacy坐标: {}\n", 
            self.system_config.pattern_layout.use_legacy_coordinates));
        report.push_str(&format!("  - 左相机序列号: {}\n", 
            self.system_config.camera_serials.left_camera_serial));
        report.push_str(&format!("  - 右相机序列号: {}\n", 
            self.system_config.camera_serials.right_camera_serial));
        
        // 相机配置报告
        report.push_str("\n📷 相机配置:\n");
        report.push_str(&format!("  - 统一帧率: {:.1} fps\n", 
            self.camera_config.acquisition_frame_rate));
        report.push_str(&format!("  - 曝光时间: {:.1} μs\n", 
            self.camera_config.exposure_time));
        report.push_str(&format!("  - 增益: {:.1} dB\n", 
            self.camera_config.gain));
        report.push_str(&format!("  - 使用legacy初始化: {}\n", 
            self.camera_config.use_legacy_camera_init));
        report.push_str(&format!("  - 左相机序列号: {}\n", 
            self.camera_config.left_camera_serial));
        report.push_str(&format!("  - 右相机序列号: {}\n", 
            self.camera_config.right_camera_serial));
        
        // 合像配置报告
        report.push_str("\n🎯 合像配置:\n");
        let pose_th = &self.alignment_config.pose_thresholds;
        report.push_str(&format!("  - 左眼姿态阈值: roll≤{:.1}°, pitch≤{:.1}°, yaw≤{:.1}°\n",
            pose_th.left_eye_max_roll, pose_th.left_eye_max_pitch, pose_th.left_eye_max_yaw));
        let align_th = &self.alignment_config.alignment_thresholds;
        report.push_str(&format!("  - 合像阈值: RMS≤{:.1}px, P95≤{:.1}px, Max≤{:.1}px\n",
            align_th.max_rms_error, align_th.max_p95_error, align_th.max_max_error));
        report.push_str(&format!("  - 使用legacy参数: {}\n", 
            self.alignment_config.use_legacy_alignment_params));
        
        // 兼容性报告
        report.push_str("\n🛡️ 兼容性保护:\n");
        report.push_str(&format!("  - 保护现有实现: {}\n", self.preserve_existing_implementations));
        report.push_str(&format!("  - 使用legacy实现: {}\n", self.should_use_legacy_implementations()));
        
        report.push_str("\n=== 报告结束 ===\n");
        report
    }
    
    /// 列出配置目录中的所有配置文件
    pub fn list_config_files(&self) -> Result<Vec<String>, String> {
        let config_dir = Path::new(&self.config_root_dir);
        if !config_dir.exists() {
            return Ok(vec![]);
        }
        
        let mut files = vec![];
        for entry in fs::read_dir(config_dir)
            .map_err(|e| format!("读取配置目录失败: {}", e))? 
        {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "yaml") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    files.push(name.to_string());
                }
            }
        }
        
        Ok(files)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
} 