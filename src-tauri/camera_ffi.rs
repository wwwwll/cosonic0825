use std::os::raw::{c_int, c_uint, c_uchar, c_char, c_void};
use serde::{Serialize, Deserialize};
use std::fmt;

//use serde::de::value::CowStrDeserializer;

/// 相机性能监控数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraPerformance {
    pub cam_index: u32,           // 相机索引
    pub target_fps: u32,          // 目标帧率
    pub actual_fps: f32,          // 实际帧率
    pub frames_dropped: u32,      // 丢帧数
    pub total_frames: u64,        // 总帧数（可选，用于详细统计）
    pub status: String,           // 状态描述
}

impl CameraPerformance {
    pub fn new(cam_index: u32) -> Self {
        Self {
            cam_index,
            target_fps: 10, // 默认10fps
            actual_fps: 0.0,
            frames_dropped: 0,
            total_frames: 0,
            status: "未初始化".to_string(),
        }
    }

    /// 更新性能数据
    pub fn update(&mut self, actual_fps: f32, frames_dropped: u32) {
        self.actual_fps = actual_fps;
        self.frames_dropped = frames_dropped;
        
        // 更新状态描述
        if actual_fps == 0.0 {
            self.status = "未运行".to_string();
        } else if (actual_fps - self.target_fps as f32).abs() <= 0.1 {
            self.status = "正常".to_string();
        } else if actual_fps < self.target_fps as f32 * 0.9 {
            self.status = "帧率偏低".to_string();
        } else {
            self.status = "运行中".to_string();
        }
    }

    /// 获取帧率准确率百分比
    pub fn get_accuracy_percentage(&self) -> f32 {
        if self.target_fps == 0 {
            return 0.0;
        }
        let accuracy = (self.actual_fps / self.target_fps as f32) * 100.0;
        accuracy.min(100.0) // 限制最大100%
    }

    /// 是否正常运行
    pub fn is_healthy(&self) -> bool {
        self.actual_fps > 0.0 && 
        (self.actual_fps - self.target_fps as f32).abs() <= self.target_fps as f32 * 0.1
    }
}

impl fmt::Display for CameraPerformance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Camera {} - Target: {}fps, Actual: {:.2}fps, Dropped: {}, Status: {} ({:.1}%)", 
               self.cam_index, self.target_fps, self.actual_fps, 
               self.frames_dropped, self.status, self.get_accuracy_percentage())
    }
}

/// 系统性能统计结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub left_camera: CameraPerformance,
    pub right_camera: CameraPerformance,
    pub system_status: String,
    pub timestamp: u64, // Unix时间戳
}

impl SystemStats {
    pub fn new() -> Self {
        Self {
            left_camera: CameraPerformance::new(0),
            right_camera: CameraPerformance::new(1),
            system_status: "初始化中".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// 更新系统统计
    pub fn update(&mut self, left_stats: (f32, u32), right_stats: (f32, u32)) {
        self.left_camera.update(left_stats.0, left_stats.1);
        self.right_camera.update(right_stats.0, right_stats.1);
        
        // 更新系统状态
        if self.left_camera.is_healthy() && self.right_camera.is_healthy() {
            self.system_status = "正常运行".to_string();
        } else if self.left_camera.actual_fps == 0.0 && self.right_camera.actual_fps == 0.0 {
            self.system_status = "未运行".to_string();
        } else {
            self.system_status = "部分异常".to_string();
        }
        
        // 更新时间戳
        self.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// 获取系统整体健康状态
    pub fn is_system_healthy(&self) -> bool {
        self.left_camera.is_healthy() && self.right_camera.is_healthy()
    }
}

impl fmt::Display for SystemStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "System Status: {}\n{}\n{}", 
               self.system_status, self.left_camera, self.right_camera)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CameraPosition {
    CamLeft = 0,
    CamRight = 1,
    Uninitailized = -1,
}

/// 触发模式枚举（对应C API）
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TriggerMode {
    Continuous = 0,  // 连续采集
    Software = 1,    // 软触发
    Hardware = 2,    // 硬触发
}

#[repr(C)]
pub struct Camera {
    pub handle: *mut c_void,       //camera handle
    pub serial: [c_char; 64],      //serial number 访问或设置时要用 CStr / CString 来安全地做编码转换
    pub opened: c_uchar,           //open status (0 = false, non-zero = true)
    pub position: CameraPosition,  //camera position
    pub trigger_mode: TriggerMode, //trigger mode
    pub frame_rate: c_uint,        //target frame rate
}

//#[link(name = "camera_sdk")]
unsafe extern "C" {
    // === 原有API ===
    pub fn camera_init() -> c_int;

    pub fn camera_start() -> c_int;
    pub fn camera_get_frame(out_bufs: *mut *mut c_uchar, out_sizes: *mut c_uint,) -> c_int;
    pub fn camera_get_frame_buf_size() -> c_uint;
    pub fn camera_release() -> c_int;
    
    // === 配置API ===
    // [配置系统 - 已注释] pub fn set_camera_mode(mode: c_int);
    
    // === 保留的监控API ===
    pub fn camera_get_status(cam_index: c_uint, fps_actual: *mut f32, frames_dropped: *mut c_uint) -> c_int;
    // pub fn camera_configure_for_stage(stage_name: *const c_char) -> c_int; // 已删除，使用SimpleCameraManager替代
    

}

/// 工作流程阶段枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    /// 预览模式：实时预览模式，连续采集图像，不进行合像检测
    Preview,
    /// 检测模式：获取单帧模式，只在前端事件触发时才获取当前的帧，不进行合像检测
    Detection,
    /// 合像模式：合像检测模式，连续采集图像，进行合像检测
    Alignment,
}

impl Stage {
    /// 转换为C字符串标识符
    pub fn as_c_str(&self) -> &'static str {
        match self {
            Stage::Preview => "preview",
            Stage::Detection => "detection", 
            Stage::Alignment => "alignment",
        }
    }
    
    /// 从字符串解析Stage
    pub fn from_str(s: &str) -> Result<Stage, WorkflowError> {
        match s {
            "preview" => Ok(Stage::Preview),
            "detection" => Ok(Stage::Detection),
            "alignment" => Ok(Stage::Alignment),
            _ => Err(WorkflowError::InvalidStage(s.to_string())),
        }
    }
}

/// 工作流程配置错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowError {
    /// 相机配置失败
    CameraConfigError { stage: String, error_code: i32 },
    /// 无效的阶段名称
    InvalidStage(String),
    /// 阶段切换超时
    SwitchTimeout { from_stage: String, to_stage: String },
    /// 系统错误
    SystemError(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::CameraConfigError { stage, error_code } => {
                write!(f, "Camera configuration failed for stage '{}': 0x{:x}", stage, error_code)
            }
            WorkflowError::InvalidStage(stage) => {
                write!(f, "Invalid stage name: '{}'", stage)
            }
            WorkflowError::SwitchTimeout { from_stage, to_stage } => {
                write!(f, "Stage switch timeout: '{}' -> '{}'", from_stage, to_stage)
            }
            WorkflowError::SystemError(msg) => {
                write!(f, "System error: {}", msg)
            }
        }
    }
}

impl std::error::Error for WorkflowError {}

#[derive(Clone)]
pub struct CameraHandle;

impl CameraHandle {
    pub fn camera_init_ffi() -> Result<Self, i32> {
        let code = unsafe {
            camera_init()
        };
        if code == 0 {
            Ok(CameraHandle)
        } else {
            Err(code)
        }
    }

    /// 手动释放相机资源
    /// 
    /// 注意：这个方法会释放所有相机资源，调用后该CameraHandle实例将不可用
    pub fn camera_release_ffi(&self) -> Result<(), i32> {
        let code = unsafe {
            camera_release()
        };
        if code == 0 {
            println!("CameraHandle: ✅ Camera resources released successfully");
            Ok(())
        } else {
            println!("CameraHandle: ⚠️  Camera release returned error: 0x{:x}", code);
            Err(code)
        }
    }



    pub fn camera_start_ffi(&self) -> Result<(), i32> {
        let code = unsafe {
            camera_start()
        };
        if code == 0 {
            Ok(())
        } else {
            Err(code)
        }
    }

    pub fn camera_get_frame_ffi(
        &self,
        out_bufs: &mut [*mut c_uchar; 2],
        out_sizes: &mut [c_uint; 2],
    ) -> Result<(), i32> {
        let code = unsafe {
            camera_get_frame(out_bufs.as_mut_ptr(), out_sizes.as_mut_ptr())
        };
        if code == 0 {
            Ok(())
        } else {
            Err(code)
        }
    }

    // pub fn camera_get_frame_ffi(&self, buffer: &mut [u32]) -> Result<usize, i32> {
    //     let mut out_ptr = buffer.as_mut_ptr() as *mut c_uchar;
    //     let mut received: c_uint = 0;
    //     let code = unsafe {
    //         camera_get_frame(&mut out_ptr as *mut _, &mut received as *mut _)
    //     };
    //     if code == 0 {
    //         Ok(received as usize)
    //     } else {
    //         Err(code)
    //     }
    // }

    pub fn camera_get_frame_buf_size_ffi() -> Result<usize, i32> {
        let sz = unsafe {
            camera_get_frame_buf_size()
        } as usize;
        if sz > 0 {
            Ok(sz)
        } else {
            Err(sz as i32)
        }
    }

    // === 新增FFI函数 ===

    // 已删除触发模式、帧率设置和软触发函数 - 新架构下不再需要

    /// 获取相机状态
    pub fn camera_get_status_ffi(&self, cam_index: u32) -> Result<(f32, u32), i32> {
        let mut fps_actual: f32 = 0.0;
        let mut frames_dropped: c_uint = 0;
        
        let code = unsafe {
            camera_get_status(cam_index, &mut fps_actual, &mut frames_dropped)
        };
        
        if code == 0 {
            Ok((fps_actual, frames_dropped))
        } else {
            Err(code)
        }
    }

    // === 已删除的工作流程配置函数 ===
    // 这些函数已被SimpleCameraManager替代，不再需要
    /*
    /// 配置工作流程阶段 - 已删除
    pub fn camera_configure_for_stage_ffi(&self, stage: Stage) -> Result<(), WorkflowError> {
        println!("⚠️ camera_configure_for_stage_ffi: 此函数已删除，请使用SimpleCameraManager");
        Err(WorkflowError::SystemError("Function removed, use SimpleCameraManager".to_string()))
    }

    /// 配置相机工作流程阶段（字符串版本）- 已删除
    pub fn camera_configure_for_stage_str_ffi(&self, stage_name: &str) -> Result<(), WorkflowError> {
        println!("⚠️ camera_configure_for_stage_str_ffi: 此函数已删除，请使用SimpleCameraManager");
        Err(WorkflowError::SystemError("Function removed, use SimpleCameraManager".to_string()))
    }
    */

    // === 新增性能监控函数 ===

    // 已删除曝光时间、增益设置和软件帧率控制函数 - 参数在camera_init.c中写死
}

/// 基于实际硬件测试的性能参数常量
pub mod performance_constants {
    use std::time::Duration;
    
    /// 软触发性能参数
    pub mod software_trigger {
        use super::Duration;
        
        /// 首次软触发预期延迟（包含相机初始化开销）
        pub const FIRST_TRIGGER_DELAY: Duration = Duration::from_millis(70);
        
        /// 后续软触发稳定延迟
        pub const STABLE_TRIGGER_DELAY: Duration = Duration::from_millis(35);
        
        /// 软触发超时时间
        pub const TRIGGER_TIMEOUT: Duration = Duration::from_millis(200);
    }
    
    /// 连续采集性能参数
    pub mod continuous_capture {
        /// 10fps模式的实际帧率准确度
        pub const FPS_ACCURACY_PERCENT: f32 = 99.1;
        
        /// 相机硬件最大帧率（测试发现）
        pub const HARDWARE_MAX_FPS: f32 = 47.0;
        
        /// 推荐的稳定工作帧率范围
        pub const STABLE_FPS_RANGE: (f32, f32) = (2.0, 15.0);
        
        /// 高性能模式帧率范围（可能不够稳定）
        pub const HIGH_PERFORMANCE_FPS_RANGE: (f32, f32) = (15.0, 25.0);
    }
    
    /// 模式切换性能参数
    pub mod mode_switching {
        use super::Duration;
        
        /// 平均模式切换时间
        pub const AVERAGE_SWITCH_TIME: Duration = Duration::from_millis(108);
        
        /// 最大模式切换时间
        pub const MAX_SWITCH_TIME: Duration = Duration::from_millis(150);
        
        /// 推荐的切换后稳定等待时间
        pub const STABILIZATION_WAIT: Duration = Duration::from_millis(200);
        
        /// 切换超时时间
        pub const SWITCH_TIMEOUT: Duration = Duration::from_millis(500);
    }
    
    /// 缓冲区管理参数
    pub mod buffer_management {
        /// 单帧缓冲区大小（字节）
        pub const FRAME_BUFFER_SIZE: usize = 5_013_504;
        
        /// 双相机总缓冲区需求
        pub const DUAL_CAMERA_BUFFER_SIZE: usize = FRAME_BUFFER_SIZE * 2;
        
        /// 推荐的缓冲区池大小（帧数）
        pub const RECOMMENDED_BUFFER_POOL_SIZE: usize = 4;
    }
}