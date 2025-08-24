use serde::{Deserialize, Serialize};

/// 系统配置 - 标定板layout、文件路径、相机序列号等核心设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// 标定板layout配置 - 保留现有实现
    pub pattern_layout: PatternLayoutConfig,
    
    /// 文件路径配置
    pub file_paths: FilePathConfig,
    
    /// 相机序列号配置
    pub camera_serials: CameraSerialConfig,
    
    /// 配置版本和元信息
    pub version: String,
    pub created_at: String,
}

/// 标定板layout配置 - 谨慎处理世界坐标问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternLayoutConfig {
    /// ⚠️ 重要：是否使用原有的写死坐标 (默认true，保护现有实现)
    pub use_legacy_coordinates: bool,
    
    /// 标定板基础参数
    pub pattern_type: String,          // "asymmetric_circles_grid" 
    pub pattern_width: i32,            // 圆点列数 (当前: 10)
    pub pattern_height: i32,           // 圆点行数 (当前: 4)
    
    /// 世界坐标设置 - 谨慎修改
    pub circle_diameter: f64,          // 圆点直径 (mm) - 当前alignment.rs中15.0
    pub diagonal_spacing: f64,         // 对角间距 (mm) - 当前25.0
    
    /// 备注：保留calibration_circles.rs中现有的写死配置作为fallback
    pub legacy_world_coords_comment: String,
    pub legacy_params_location: String,
}

/// 文件路径配置 - 兼容现有的param_io.rs路径传参方式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePathConfig {
    /// 相机配置文件路径
    pub camera_config_dir: String,           // 默认: "configs/camera/"
    
    /// 标定相关路径 - 兼容现有param_io.rs
    pub calibration_images_dir: String,      // 标定图像保存路径
    pub calibration_params_dir: String,      // 内参外参保存路径 (当前: src-tauri/)
    pub rectify_maps_path: String,           // 重投影矩阵路径 (当前: rectify_maps.yaml)
    
    /// 合像参数路径
    pub alignment_config_dir: String,        // 合像参数配置路径
    
    /// 兼容性设置
    pub use_param_io_paths: bool,            // 是否使用param_io.rs的现有路径传参方式
    
    /// 当前使用的标定参数文件路径 (与alignment_workflow.rs保持一致)
    pub left_camera_params_path: String,     // "left_camera_params.yaml"
    pub right_camera_params_path: String,    // "right_camera_params.yaml"  
    pub stereo_params_path: String,          // "stereo_params.yaml"
    pub rectify_params_path: String,         // "rectify_params.yaml"
}

/// 相机序列号配置 - 从camera_api.h迁移
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSerialConfig {
    /// 当前在camera_api.h中定义的序列号
    pub left_camera_serial: String,         // 当前: "DA5158733"
    pub right_camera_serial: String,        // 当前: "DA5158736"
    pub auto_detect_serials: bool,          // 是否自动检测序列号
    
    /// 备注信息
    pub legacy_serial_location: String,     // 记录原序列号定义位置
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            pattern_layout: PatternLayoutConfig {
                use_legacy_coordinates: true,  // 默认使用原有坐标，保护现有实现
                pattern_type: "asymmetric_circles_grid".to_string(),
                pattern_width: 10,  // 当前calibration_circles.rs中的设置
                pattern_height: 4,  // 当前calibration_circles.rs中的设置
                circle_diameter: 15.0,  // 当前alignment.rs中的设置
                diagonal_spacing: 25.0,  // 当前alignment.rs中的设置
                legacy_world_coords_comment: "保留calibration_circles.rs中的原有世界坐标设置，包括generate_world_points_from_list函数".to_string(),
                legacy_params_location: "src-tauri/src/modules/calibration_circles.rs:generate_world_points_from_list".to_string(),
            },
            file_paths: FilePathConfig {
                camera_config_dir: "configs/camera/".to_string(),
                calibration_images_dir: "calibration_images/".to_string(),
                calibration_params_dir: "./".to_string(),  // 当前在src-tauri/根目录
                rectify_maps_path: "rectify_maps.yaml".to_string(),
                alignment_config_dir: "configs/alignment/".to_string(),
                use_param_io_paths: true,  // 兼容现有param_io.rs
                
                // 当前alignment_workflow.rs中使用的路径
                left_camera_params_path: "left_camera_params.yaml".to_string(),
                right_camera_params_path: "right_camera_params.yaml".to_string(),
                stereo_params_path: "stereo_params.yaml".to_string(),
                rectify_params_path: "rectify_params.yaml".to_string(),
            },
            camera_serials: CameraSerialConfig {
                left_camera_serial: "DA5158733".to_string(),   // 从camera_api.h读取
                right_camera_serial: "DA5158736".to_string(),  // 从camera_api.h读取
                auto_detect_serials: false,  // 当前使用固定序列号
                legacy_serial_location: "src-tauri/camera_sdk/include/camera_api.h:29-30".to_string(),
            },
            version: "1.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

impl SystemConfig {
    /// 创建新的系统配置实例
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 验证标定板参数
        if self.pattern_layout.pattern_width <= 0 || self.pattern_layout.pattern_height <= 0 {
            return Err("标定板尺寸必须为正数".to_string());
        }
        
        if self.pattern_layout.circle_diameter <= 0.0 || self.pattern_layout.diagonal_spacing <= 0.0 {
            return Err("圆点直径和间距必须为正数".to_string());
        }
        
        // 验证相机序列号
        if self.camera_serials.left_camera_serial.is_empty() || 
           self.camera_serials.right_camera_serial.is_empty() {
            return Err("相机序列号不能为空".to_string());
        }
        
        Ok(())
    }
    
    /// 获取当前使用的标定板参数 (优先使用legacy实现)
    pub fn get_effective_pattern_params(&self) -> (f32, f32, opencv::core::Size) {
        if self.pattern_layout.use_legacy_coordinates {
            // 使用calibration_circles.rs中的原有参数
            (15.0, 25.0, opencv::core::Size::new(4, 10))
        } else {
            // 使用配置文件中的参数
            (
                self.pattern_layout.circle_diameter as f32,
                self.pattern_layout.diagonal_spacing as f32,
                opencv::core::Size::new(self.pattern_layout.pattern_height, self.pattern_layout.pattern_width)
            )
        }
    }
    
    /// 获取当前使用的相机序列号
    pub fn get_effective_camera_serials(&self) -> (String, String) {
        (
            self.camera_serials.left_camera_serial.clone(),
            self.camera_serials.right_camera_serial.clone()
        )
    }
} 