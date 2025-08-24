use serde::{Deserialize, Serialize};

/// 相机配置 - 统一配置左右两个相机，保护现有camera_init.c实现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraConfig {
    /// 基础硬件参数 - 对应海康SDK参数，左右相机保持一致
    pub acquisition_frame_rate_enable: bool,  // "AcquisitionFrameRateEnable" 
    pub acquisition_frame_rate: f64,           // "AcquisitionFrameRate" (当前硬编码15fps)
    pub exposure_time: f64,                    // "ExposureTime" (μs) - 新增配置
    pub exposure_auto: i32,                    // "ExposureAuto" (始终为0: Off)
    pub gain: f64,                            // "Gain" (dB) - 新增配置
    pub gain_auto: i32,                       // "GainAuto" (始终为0: Off)
    
    /// 触发参数 - ⚠️ 注意与现有代码的兼容性
    pub trigger_mode: i32,                    // "TriggerMode" (0: Off, 1: On)
    pub trigger_source: String,               // "TriggerSource"
    
    /// 图像参数 - 当前从camera_init.c读取
    pub width: u32,                           // 图像宽度 (当前2448)
    pub height: u32,                          // 图像高度 (当前2048)
    
    /// ROI区域参数 - 新增功能
    pub roi: RoiConfig,                       // ROI区域设置
    
    /// 标定用SimpleBlobDetector参数 - 保留现有实现
    pub calibration_blob_detector: BlobDetectorConfig,
    
    /// 相机序列号 - 统一管理左右相机
    pub left_camera_serial: String,           // 左相机序列号
    pub right_camera_serial: String,          // 右相机序列号
    
    /// 兼容性设置
    pub use_legacy_camera_init: bool,         // 是否使用camera_init.c中的现有设置
    pub legacy_init_location: String,         // 记录原实现位置
}

/// ROI区域配置 - 新功能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoiConfig {
    pub offset_x: i32,                        // "OffsetX" ROI X方向偏移量
    pub offset_y: i32,                        // "OffsetY" ROI Y方向偏移量  
    pub width: i32,                           // "Width" ROI宽度
    pub height: i32,                          // "Height" ROI高度
    pub enabled: bool,                        // 是否启用ROI
    pub applies_to_both_cameras: bool,        // 是否对左右相机都生效
}

/// SimpleBlobDetector配置 - 用于标定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobDetectorConfig {
    /// ⚠️ 重要：不要修改calibration_circles.rs中现有的写死参数
    pub use_legacy_params: bool,              // 是否使用原有写死参数
    
    /// SimpleBlobDetector参数 - 基于calibration_circles.rs中的设置
    pub min_threshold: f32,                   // 当前: 10.0
    pub max_threshold: f32,                   // 当前: 200.0
    pub threshold_step: f32,                  // 当前: 10.0
    pub min_area: f32,                        // 当前: 50.0
    pub max_area: f32,                        // 当前: 5000.0
    pub filter_by_circularity: bool,          // 当前: true
    pub min_circularity: f32,                 // 当前: 0.5
    pub filter_by_convexity: bool,            // 当前: true
    pub min_convexity: f32,                   // 当前: 0.8
    pub filter_by_inertia: bool,              // 当前: true
    pub min_inertia_ratio: f32,               // 当前: 0.1
    pub filter_by_color: bool,                // 当前: false
    pub blob_color: u8,                       // 当前: 0
    
    /// 备注信息
    pub legacy_params_comment: String,
    pub legacy_params_location: String,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            // 基于camera_init.c中的现有设置
            acquisition_frame_rate_enable: true,    // 当前在camera_init.c:206中设置
            acquisition_frame_rate: 15.0,           // 当前在camera_init.c:212中硬编码
            exposure_time: 0.0,                     // 使用相机默认值，新增配置
            exposure_auto: 0,                       // Off
            gain: 0.0,                             // 默认无增益，新增配置
            gain_auto: 0,                          // Off
            
            // 触发参数 - 兼容现有实现
            trigger_mode: 0,                       // 连续模式 (当前在camera_init.c:200中设置)
            trigger_source: "Software".to_string(), // 默认软触发
            
            // 图像参数 - 从现有代码读取 (2448×2048)
            width: 2448,
            height: 2048,
            
            // ROI - 新功能，默认全图
            roi: RoiConfig {
                offset_x: 0,
                offset_y: 0,
                width: 2448,
                height: 2048,
                enabled: false,                    // 默认不启用
                applies_to_both_cameras: true,    // 默认对两个相机都生效
            },
            
            // 标定用检测器 - 保留现有实现
            calibration_blob_detector: BlobDetectorConfig {
                use_legacy_params: true,           // 默认使用现有参数
                
                // 基于calibration_circles.rs:50-75的参数
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
            
            // 相机序列号 - 统一管理左右相机
            left_camera_serial: "DA5158733".to_string(),   // 从camera_api.h读取
            right_camera_serial: "DA5158736".to_string(),  // 从camera_api.h读取
            
            // 兼容性设置
            use_legacy_camera_init: true,          // 默认使用现有camera_init.c实现
            legacy_init_location: "src-tauri/camera_sdk/src/camera_init.c:196-218".to_string(),
        }
    }
}

impl CameraConfig {
    /// 创建新的相机配置
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 创建带指定序列号的相机配置
    pub fn new_with_serials(left_serial: &str, right_serial: &str) -> Self {
        let mut config = Self::default();
        config.left_camera_serial = left_serial.to_string();
        config.right_camera_serial = right_serial.to_string();
        config
    }
    
    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), String> {
        // 验证帧率
        if self.acquisition_frame_rate <= 0.0 || self.acquisition_frame_rate > 60.0 {
            return Err("帧率必须在0-60fps范围内".to_string());
        }
        
        // 验证曝光时间 (如果设置了)
        if self.exposure_time < 0.0 {
            return Err("曝光时间不能为负数".to_string());
        }
        
        // 验证增益 (如果设置了)
        if self.gain < 0.0 {
            return Err("增益不能为负数".to_string());
        }
        
        // 验证相机序列号
        if self.left_camera_serial.is_empty() || self.right_camera_serial.is_empty() {
            return Err("左右相机序列号不能为空".to_string());
        }
        
        // 验证ROI参数
        if self.roi.enabled {
            if self.roi.offset_x < 0 || self.roi.offset_y < 0 ||
               self.roi.width <= 0 || self.roi.height <= 0 {
                return Err("ROI参数无效".to_string());
            }
            
            if (self.roi.offset_x + self.roi.width) > self.width as i32 ||
               (self.roi.offset_y + self.roi.height) > self.height as i32 {
                return Err("ROI区域超出图像范围".to_string());
            }
        }
        
        Ok(())
    }
    
    /// 获取当前有效的SimpleBlobDetector参数
    pub fn get_effective_blob_detector_params(&self) -> &BlobDetectorConfig {
        // 总是返回当前配置，但实际使用时检查use_legacy_params标志
        &self.calibration_blob_detector
    }
    
    /// 检查是否应该绕过现有的camera_init.c实现
    pub fn should_bypass_legacy_init(&self) -> bool {
        !self.use_legacy_camera_init
    }
    
    /// 获取相机序列号
    pub fn get_camera_serials(&self) -> (String, String) {
        (self.left_camera_serial.clone(), self.right_camera_serial.clone())
    }
} 