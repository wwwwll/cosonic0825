use serde::{Deserialize, Serialize};

/// 合像参数配置 - 保护现有alignment.rs实现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentConfig {
    /// ⚠️ 重要：标定板layout与系统参数保持一致
    pub pattern_layout_ref: String,    // 引用系统配置中的标定板layout
    pub use_system_pattern_layout: bool, // 是否使用系统配置的layout
    
    /// 合像检测用SimpleBlobDetector参数 - 保留现有实现
    pub alignment_blob_detector: AlignmentBlobDetectorConfig,
    
    /// 姿态检测阈值设置 - 当前在alignment.rs中写死
    pub pose_thresholds: PoseThresholds,
    
    /// 合像判定阈值 - 当前在alignment.rs中写死  
    pub alignment_thresholds: AlignmentThresholds,
    
    /// ROI区域设置 - 基于性能优化结果
    pub roi_config: AlignmentRoiConfig,
    
    /// 兼容性设置
    pub use_legacy_alignment_params: bool,  // 是否使用alignment.rs中的原有参数
    pub legacy_params_location: String,     // 记录原参数位置
}

/// 合像检测用SimpleBlobDetector配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentBlobDetectorConfig {
    /// ⚠️ 重要：不要修改alignment.rs中现有的写死参数
    pub use_legacy_alignment_params: bool,  // 是否使用alignment.rs中的原有参数
    
    /// SimpleBlobDetector参数 - 基于性能优化结果 (alignment.rs:311-330)
    pub min_threshold: f32,             // 当前在alignment.rs中: 100.0
    pub max_threshold: f32,             // 当前在alignment.rs中: 200.0  
    pub threshold_step: f32,            // 当前在alignment.rs中: 50.0
    pub min_area: f32,                  // 当前在alignment.rs中: 100.0
    pub max_area: f32,                  // 当前在alignment.rs中: 8000.0
    pub filter_by_circularity: bool,    // 当前在alignment.rs中: true
    pub min_circularity: f32,           // 当前在alignment.rs中: 0.6
    pub filter_by_convexity: bool,      // 当前在alignment.rs中: false
    pub min_convexity: f32,             // 当前在alignment.rs中: 0.87
    pub filter_by_inertia: bool,        // 当前在alignment.rs中: false
    pub min_inertia_ratio: f32,         // 当前在alignment.rs中: 0.1
    pub filter_by_color: bool,          // 当前在alignment.rs中: false
    
    /// 备注信息
    pub legacy_params_location: String, // 记录原参数位置
    pub optimization_notes: String,     // 性能优化说明
}

/// 姿态检测阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseThresholds {
    /// ⚠️ 重要：当前在alignment.rs中写死，保留原有实现
    pub use_legacy_pose_thresholds: bool,  // 是否使用原有阈值
    
    /// 左眼基准姿态判定阈值 - 基于alignment.rs:17-18的常量
    pub left_eye_max_roll: f64,        // 左眼最大滚转角 (度) - 当前ROLL_TH: 5.0
    pub left_eye_max_pitch: f64,       // 左眼最大俯仰角 (度) - 当前PITCH_YAW_TH: 10.0
    pub left_eye_max_yaw: f64,         // 左眼最大偏航角 (度) - 当前PITCH_YAW_TH: 10.0
    pub left_eye_max_translation: f64, // 左眼最大平移距离 (mm) - 居中判定
    
    /// 右眼姿态判定 - 不判定是否居中，基于alignment.rs:523-525
    pub right_eye_max_roll: f64,       // 右眼最大滚转角 (度) - 当前ROLL_TH: 5.0
    pub right_eye_max_pitch: f64,      // 右眼最大俯仰角 (度) - 当前PITCH_YAW_TH: 10.0
    pub right_eye_max_yaw: f64,        // 右眼最大偏航角 (度) - 当前PITCH_YAW_TH: 10.0
    // 注意：右眼不检查平移距离（居中性）
    
    /// 备注信息
    pub legacy_thresholds_location: String,
}

/// 合像判定阈值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentThresholds {
    /// ⚠️ 重要：当前在alignment.rs中写死，保留原有实现
    pub use_legacy_alignment_thresholds: bool,  // 是否使用原有阈值
    
    /// 双光机合像判定阈值 - 基于alignment.rs:19-21的常量
    pub max_rms_error: f64,            // 最大RMS误差 (像素) - 当前RMS_TH: 100.0
    pub max_p95_error: f64,            // 最大P95误差 (像素) - 当前P95_TH: 100.0
    pub max_max_error: f64,            // 最大最大误差 (像素) - 当前MAX_TH: 200.0
    
    /// 调整提示阈值 - 用于指导调整方向
    pub adjustment_hint_threshold: f64, // 调整提示阈值 (像素)
    pub mean_dx_threshold: f64,        // X方向均值阈值
    pub mean_dy_threshold: f64,        // Y方向均值阈值
    
    /// 备注信息
    pub legacy_thresholds_location: String,
}

/// 合像ROI配置 - 基于性能优化结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentRoiConfig {
    /// 右相机ROI设置 - 基于性能优化测试结果
    pub right_roi_enabled: bool,       // 是否启用右相机ROI
    pub right_roi_x: i32,              // 右相机ROI X偏移 - 基于测试优化结果
    pub right_roi_y: i32,              // 右相机ROI Y偏移
    pub right_roi_width: i32,          // 右相机ROI宽度
    pub right_roi_height: i32,         // 右相机ROI高度
    
    /// 左相机ROI设置 - 通常不需要ROI
    pub left_roi_enabled: bool,        // 是否启用左相机ROI
    pub left_roi_x: i32,               // 左相机ROI X偏移
    pub left_roi_y: i32,               // 左相机ROI Y偏移
    pub left_roi_width: i32,           // 左相机ROI宽度
    pub left_roi_height: i32,          // 左相机ROI高度
    
    /// ROI优化说明
    pub roi_optimization_notes: String,
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            // 标定板layout引用系统配置
            pattern_layout_ref: "system_config.pattern_layout".to_string(),
            use_system_pattern_layout: true,  // 默认使用系统配置
            
            // 合像检测参数 - 基于性能优化结果，但保留原有实现
            alignment_blob_detector: AlignmentBlobDetectorConfig {
                use_legacy_alignment_params: true,  // 默认使用原有参数
                
                // 基于alignment.rs:311-330的优化参数值
                min_threshold: 100.0,
                max_threshold: 200.0,
                threshold_step: 50.0,
                min_area: 100.0,
                max_area: 8000.0,
                filter_by_circularity: true,
                min_circularity: 0.6,
                filter_by_convexity: false,  // 性能优化：禁用凸性检查
                min_convexity: 0.87,
                filter_by_inertia: false,    // 性能优化：禁用惯性滤波
                min_inertia_ratio: 0.1,
                filter_by_color: false,
                
                legacy_params_location: "src-tauri/src/modules/alignment.rs:create_optimized_blob_detector".to_string(),
                optimization_notes: "基于PERFORMANCE_OPTIMIZATION_SUMMARY.md的优化结果，10.1倍性能提升".to_string(),
            },
            
            // 姿态检测阈值 - 保留原有实现
            pose_thresholds: PoseThresholds {
                use_legacy_pose_thresholds: true,  // 默认使用原有阈值
                
                // 左眼基准姿态判定 - 基于alignment.rs:17-18
                left_eye_max_roll: 5.0,      // ROLL_TH
                left_eye_max_pitch: 10.0,    // PITCH_YAW_TH
                left_eye_max_yaw: 10.0,      // PITCH_YAW_TH
                left_eye_max_translation: 10.0,  // 居中判定阈值
                
                // 右眼姿态判定 - 不判定居中
                right_eye_max_roll: 5.0,     // ROLL_TH
                right_eye_max_pitch: 10.0,   // PITCH_YAW_TH
                right_eye_max_yaw: 10.0,     // PITCH_YAW_TH
                
                legacy_thresholds_location: "src-tauri/src/modules/alignment.rs:17-21".to_string(),
            },
            
            // 合像判定阈值 - 保留原有实现
            alignment_thresholds: AlignmentThresholds {
                use_legacy_alignment_thresholds: true,  // 默认使用原有阈值
                
                // 基于alignment.rs:19-21的阈值常量
                max_rms_error: 100.0,        // RMS_TH
                max_p95_error: 100.0,        // P95_TH
                max_max_error: 200.0,        // MAX_TH
                
                adjustment_hint_threshold: 1.0,
                mean_dx_threshold: 0.5,
                mean_dy_threshold: 0.5,
                
                legacy_thresholds_location: "src-tauri/src/modules/alignment.rs:19-21".to_string(),
            },
            
            // ROI配置 - 基于性能优化结果
            roi_config: AlignmentRoiConfig {
                // 右相机ROI - 基于性能优化测试结果
                right_roi_enabled: false,    // 默认不启用，可根据需要开启
                right_roi_x: 900,           // 基于测试优化的典型值
                right_roi_y: 0,
                right_roi_width: 1548,      // 2448 - 900
                right_roi_height: 1250,     // 减少搜索范围50%
                
                // 左相机ROI - 通常全图检测
                left_roi_enabled: false,
                left_roi_x: 0,
                left_roi_y: 0,
                left_roi_width: 2448,
                left_roi_height: 2048,
                
                roi_optimization_notes: "右相机ROI可减少50%搜索范围，提升检测性能".to_string(),
            },
            
            // 兼容性设置
            use_legacy_alignment_params: true,  // 默认使用原有参数
            legacy_params_location: "src-tauri/src/modules/alignment.rs".to_string(),
        }
    }
}

impl AlignmentConfig {
    /// 创建新的合像配置实例
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 验证姿态阈值
        if self.pose_thresholds.left_eye_max_roll <= 0.0 ||
           self.pose_thresholds.left_eye_max_pitch <= 0.0 ||
           self.pose_thresholds.left_eye_max_yaw <= 0.0 {
            return Err("姿态阈值必须为正数".to_string());
        }
        
        // 验证合像阈值
        if self.alignment_thresholds.max_rms_error <= 0.0 ||
           self.alignment_thresholds.max_p95_error <= 0.0 ||
           self.alignment_thresholds.max_max_error <= 0.0 {
            return Err("合像阈值必须为正数".to_string());
        }
        
        // 验证ROI参数
        if self.roi_config.right_roi_enabled {
            if self.roi_config.right_roi_x < 0 || self.roi_config.right_roi_y < 0 ||
               self.roi_config.right_roi_width <= 0 || self.roi_config.right_roi_height <= 0 {
                return Err("右相机ROI参数无效".to_string());
            }
        }
        
        Ok(())
    }
    
    /// 获取当前有效的姿态阈值 (优先使用legacy实现)
    pub fn get_effective_pose_thresholds(&self) -> &PoseThresholds {
        // 总是返回当前配置，但实际使用时检查use_legacy_pose_thresholds标志
        &self.pose_thresholds
    }
    
    /// 获取当前有效的合像阈值 (优先使用legacy实现)
    pub fn get_effective_alignment_thresholds(&self) -> &AlignmentThresholds {
        // 总是返回当前配置，但实际使用时检查use_legacy_alignment_thresholds标志
        &self.alignment_thresholds
    }
    
    /// 获取当前有效的SimpleBlobDetector参数 (优先使用legacy实现)
    pub fn get_effective_blob_detector_params(&self) -> &AlignmentBlobDetectorConfig {
        // 总是返回当前配置，但实际使用时检查use_legacy_alignment_params标志
        &self.alignment_blob_detector
    }
    
    /// 检查是否应该绕过现有的alignment.rs实现
    pub fn should_bypass_legacy_alignment(&self) -> bool {
        !self.use_legacy_alignment_params
    }
} 