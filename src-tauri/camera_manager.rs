/**
 * @file camera_manager.rs
 * @brief 简化的相机管理器 - 基于连续采集的统一接口
 * 
 * ## 🎯 设计原则
 * - **极简设计**: 只有3个核心方法 start/capture_and_process/stop
 * - **硬件优化**: 10fps硬件帧率控制，无软件干预
 * - **业务导向**: 通过save_current_frame控制图像处理逻辑
 * - **零状态管理**: 无复杂的工作流阶段，业务层自行控制
 * 
 * ## 📋 核心接口
 * ```rust
 * let manager = SimpleCameraManager::new()?;
 * manager.start()?;                                    // 启动10fps连续采集
 * let (left, right) = manager.capture_and_process(save_flag)?; // 获取图像，可选保存
 * manager.stop()?;                                     // 停止并释放资源
 * ```
 * 
 * @version 2.0
 * @date 2025-01-15
 * @author Camera Simplification Expert
 */

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
// use std::os::raw::{c_uchar, c_uint}; // 暂时未使用
use crate::camera_ffi::CameraHandle;

/// 简化的相机管理器
/// 
/// 基于硬件10fps连续采集，提供统一的图像获取接口
pub struct SimpleCameraManager {
    /// 相机FFI句柄
    cam_handle: CameraHandle,
    /// 运行状态标志
    running: Arc<AtomicBool>,
    /// 帧缓冲区大小
    frame_buf_size: u32,
    /// 帧计数器（用于文件命名）
    frame_counter: Arc<Mutex<u32>>,
}

/// 相机管理错误类型
#[derive(Debug, Clone)]
pub enum CameraError {
    /// 初始化失败
    InitFailed(i32),
    /// 启动失败
    StartFailed(i32),
    /// 采集失败
    CaptureFailed(i32),
    /// 停止失败
    StopFailed(i32),
    /// 相机未启动
    NotStarted,
    /// 相机已启动
    AlreadyStarted,
    /// 文件保存失败
    SaveFailed(String),
}

impl std::fmt::Display for CameraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraError::InitFailed(code) => write!(f, "Camera initialization failed: 0x{:x}", code),
            CameraError::StartFailed(code) => write!(f, "Camera start failed: 0x{:x}", code),
            CameraError::CaptureFailed(code) => write!(f, "Frame capture failed: 0x{:x}", code),
            CameraError::StopFailed(code) => write!(f, "Camera stop failed: 0x{:x}", code),
            CameraError::NotStarted => write!(f, "Camera not started"),
            CameraError::AlreadyStarted => write!(f, "Camera already started"),
            CameraError::SaveFailed(msg) => write!(f, "File save failed: {}", msg),
        }
    }
}

impl std::error::Error for CameraError {}

impl SimpleCameraManager {
    /// 创建新的相机管理器
    /// 
    /// # 返回值
    /// - `Ok(SimpleCameraManager)`: 创建成功
    /// - `Err(CameraError)`: 创建失败
    /// 
    /// # 示例
    /// ```rust
    /// let manager = SimpleCameraManager::new()?;
    /// ```
    pub fn new() -> Result<Self, CameraError> {
        println!("🏗️ SimpleCameraManager::new: 初始化相机管理器...");
        
        // 1. 初始化相机硬件
        let cam_handle = CameraHandle::camera_init_ffi()
            .map_err(|e| {
                eprintln!("❌ SimpleCameraManager::new: 相机初始化失败: 0x{:x}", e);
                CameraError::InitFailed(e)
            })?;
        
        // 2. 获取帧缓冲区大小
        let frame_buf_size = CameraHandle::camera_get_frame_buf_size_ffi()
            .map_err(|e| {
                eprintln!("❌ SimpleCameraManager::new: 获取帧缓冲区大小失败: 0x{:x}", e);
                CameraError::InitFailed(e)
            })? as u32;
        
        println!("✅ SimpleCameraManager::new: 相机初始化成功");
        println!("   - 帧缓冲区大小: {} bytes", frame_buf_size);
        println!("   - 硬件配置: 10fps连续采集模式");
        
        Ok(Self {
            cam_handle,
            running: Arc::new(AtomicBool::new(false)),
            frame_buf_size,
            frame_counter: Arc::new(Mutex::new(0)),
        })
    }
    
    /// 启动连续采集
    /// 
    /// 启动10fps硬件控制的连续采集模式。
    /// 
    /// # 返回值
    /// - `Ok(())`: 启动成功
    /// - `Err(CameraError)`: 启动失败
    /// 
    /// # 示例
    /// ```rust
    /// manager.start()?;
    /// ```
    pub fn start(&self) -> Result<(), CameraError> {
        println!("🚀 SimpleCameraManager::start: 启动连续采集...");
        
        // 检查是否已经启动
        if self.running.load(Ordering::SeqCst) {
            println!("⚠️ SimpleCameraManager::start: 相机已经启动");
            return Err(CameraError::AlreadyStarted);
        }
        
        // 启动相机采集
        self.cam_handle.camera_start_ffi()
            .map_err(|e| {
                eprintln!("❌ SimpleCameraManager::start: 启动失败: 0x{:x}", e);
                CameraError::StartFailed(e)
            })?;
        
        // 设置运行状态
        self.running.store(true, Ordering::SeqCst);
        
        println!("✅ SimpleCameraManager::start: 连续采集已启动");
        println!("   - 模式: 10fps硬件帧率控制");
        println!("   - 状态: 连续采集中...");
        
        Ok(())
    }
    
    /// 获取当前帧数据（纯内存操作）
    /// 
    /// 从连续采集中获取当前帧数据，不进行任何磁盘操作。
    /// 
    /// # 返回值
    /// - `Ok((left_data, right_data))`: 成功获取的图像数据
    /// - `Err(CameraError)`: 获取失败
    /// 
    /// # 示例
    /// ```rust
    /// // 获取当前帧到内存缓冲区
    /// let (left, right) = manager.get_current_frame()?;
    /// // 业务层决定如何处理这些数据
    /// ```
    pub fn get_current_frame(&self) -> Result<(Vec<u8>, Vec<u8>), CameraError> {
        // 检查相机是否已启动
        if !self.running.load(Ordering::SeqCst) {
            eprintln!("❌ SimpleCameraManager::get_current_frame: 相机未启动");
            return Err(CameraError::NotStarted);
        }
        
        // 分配缓冲区
        let mut left_buffer = vec![0u8; self.frame_buf_size as usize];
        let mut right_buffer = vec![0u8; self.frame_buf_size as usize];
        let mut out_bufs = [left_buffer.as_mut_ptr(), right_buffer.as_mut_ptr()];
        let mut out_sizes = [0u32; 2];
        
        // 调用C层获取图像
        self.cam_handle.camera_get_frame_ffi(&mut out_bufs, &mut out_sizes)
            .map_err(|e| {
                eprintln!("❌ SimpleCameraManager::get_current_frame: 获取帧数据失败: 0x{:x}", e);
                CameraError::CaptureFailed(e)
            })?;
        
        // 调整缓冲区大小到实际数据大小
        left_buffer.truncate(out_sizes[0] as usize);
        right_buffer.truncate(out_sizes[1] as usize);
        
        println!("✅ SimpleCameraManager::get_current_frame: 获取帧数据成功 (Left: {} bytes, Right: {} bytes)", 
                 out_sizes[0], out_sizes[1]);
        
        Ok((left_buffer, right_buffer))
    }

    /// 【已弃用】统一的图像获取和处理接口
    /// 
    /// ⚠️ **此方法已弃用，请使用以下替代方案**：
    /// - 仅获取数据: `get_current_frame()`
    /// - 保存到文件: `save_frame_to_file()`
    /// 
    /// 这样可以更好地分离关注点，支持缓冲区架构。
    #[deprecated(since = "2.1.0", note = "使用 get_current_frame() 和 save_frame_to_file() 替代")]
    pub fn capture_and_process(&self, save_current_frame: bool) -> Result<(Vec<u8>, Vec<u8>), CameraError> {
        println!("⚠️ capture_and_process() 已弃用，请使用 get_current_frame() 和 save_frame_to_file()");
        
        // 1. 获取图像数据
        let (left_data, right_data) = self.get_current_frame()?;
        
        // 2. 可选：保存当前帧到磁盘（使用旧的逻辑保持兼容性）
        if save_current_frame {
            self.save_frame_to_disk(&left_data, &right_data)?;
        }
        
        // 3. 返回图像数据供业务层使用
        Ok((left_data, right_data))
    }
    
    /// 停止采集并释放资源
    /// 
    /// 停止连续采集并释放所有相机资源。
    /// 
    /// # 返回值
    /// - `Ok(())`: 停止成功
    /// - `Err(CameraError)`: 停止失败
    /// 
    /// # 示例
    /// ```rust
    /// manager.stop()?;
    /// ```
    pub fn stop(&self) -> Result<(), CameraError> {
        println!("🛑 SimpleCameraManager::stop: 停止采集并释放资源...");
        
        // 检查是否正在运行
        if !self.running.load(Ordering::SeqCst) {
            println!("⚠️ SimpleCameraManager::stop: 相机未启动");
            return Ok(());
        }
        
        // 设置停止标志
        self.running.store(false, Ordering::SeqCst);
        
        // 释放相机资源
        self.cam_handle.camera_release_ffi()
            .map_err(|e| {
                eprintln!("❌ SimpleCameraManager::stop: 资源释放失败: 0x{:x}", e);
                CameraError::StopFailed(e)
            })?;
        
        println!("✅ SimpleCameraManager::stop: 资源释放完成");
        
        Ok(())
    }
    
    /// 检查相机运行状态
    /// 
    /// # 返回值
    /// - `true`: 相机正在运行
    /// - `false`: 相机已停止
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// 获取帧缓冲区大小
    /// 
    /// # 返回值
    /// 帧缓冲区大小（字节）
    pub fn get_frame_buffer_size(&self) -> u32 {
        self.frame_buf_size
    }
    
    // ==================== 内部方法 ====================
    
    /// 保存帧数据到磁盘（内部方法）
    fn save_frame_to_disk(&self, left_data: &[u8], right_data: &[u8]) -> Result<(), CameraError> {
        println!("💾 SimpleCameraManager::save_frame_to_disk: 保存帧数据到磁盘");
        
        // 生成唯一的帧编号
        let frame_number = {
            let mut counter = self.frame_counter.lock().unwrap();
            *counter += 1;
            *counter
        };
        
        // 生成文件名
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 使用项目根目录下的captures目录
        let captures_dir = "captures";
        let left_filename = format!("{}/frame_{}_{:06}_L.raw", captures_dir, timestamp, frame_number);
        let right_filename = format!("{}/frame_{}_{:06}_R.raw", captures_dir, timestamp, frame_number);
        
        // 确保目录存在
        std::fs::create_dir_all(captures_dir)
            .map_err(|e| CameraError::SaveFailed(format!("Failed to create directory: {}", e)))?;
        
        // 保存文件
        std::fs::write(&left_filename, left_data)
            .map_err(|e| CameraError::SaveFailed(format!("Failed to save left image: {}", e)))?;
        std::fs::write(&right_filename, right_data)
            .map_err(|e| CameraError::SaveFailed(format!("Failed to save right image: {}", e)))?;
        
        println!("✅ SimpleCameraManager::save_frame_to_disk: 保存完成");
        println!("   - 左图像: {} ({} bytes)", left_filename, left_data.len());
        println!("   - 右图像: {} ({} bytes)", right_filename, right_data.len());
        
        Ok(())
    }
}

impl Drop for SimpleCameraManager {
    /// 析构函数：确保C层资源正确释放
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            println!("⚠️ SimpleCameraManager::drop: 检测到未正常停止，强制释放C层资源");
            
            // 设置停止标志，避免其他线程继续使用
            self.running.store(false, Ordering::SeqCst);
            
            // 直接调用C层释放，不经过stop()避免重复检查
            if let Err(e) = self.cam_handle.camera_release_ffi() {
                eprintln!("❌ SimpleCameraManager::drop: C层资源释放失败: 0x{:x}", e);
            } else {
                println!("✅ SimpleCameraManager::drop: C层资源已强制释放");
            }
        }
    }
}

// ==================== 假的CameraManager用于编译兼容 ====================
// 
// 这是一个临时的假实现，用于让现有代码编译通过
// 真正的相机管理功能已迁移到 SimpleCameraManager

use tauri::AppHandle;

/// 假的CameraManager - 仅用于编译兼容
#[derive(Debug)]
pub struct CameraManager {
    _app_handle: AppHandle,
}

/// 假的WorkflowStage - 仅用于编译兼容
#[derive(Debug, Clone, Copy)]
pub enum WorkflowStage {
    Preview,
    Detection, 
    Alignment,
}

impl CameraManager {
    /// 假的构造函数
    pub fn new(app_handle: AppHandle) -> Result<Self, i32> {
        Ok(Self {
            _app_handle: app_handle,
        })
    }

    /// 假的预览启动
    pub fn start_preview(&self) -> Result<(), i32> {
        println!("⚠️ CameraManager::start_preview: 这是假实现，请使用SimpleCameraManager");
        Ok(())
    }

    /// 假的预览停止
    pub fn stop_preview(&self) -> Result<(), i32> {
        println!("⚠️ CameraManager::stop_preview: 这是假实现，请使用SimpleCameraManager");
        Ok(())
    }

    /// 假的帧捕获
    pub fn capture_frame(&self) -> Result<(String, String), i32> {
        println!("⚠️ CameraManager::capture_frame: 这是假实现，请使用SimpleCameraManager");
        Ok(("fake_left".to_string(), "fake_right".to_string()))
    }

    /// 假的单帧采集
    pub fn capture_single_frame(&self) -> Result<(Vec<u8>, Vec<u8>), String> {
        println!("⚠️ CameraManager::capture_single_frame: 这是假实现，请使用SimpleCameraManager");
        Ok((vec![0u8; 100], vec![0u8; 100]))
    }

    /// 假的原始图像采集
    pub fn capture_raw_images(&self, _count: usize) -> Result<Vec<(Vec<u8>, Vec<u8>)>, String> {
        println!("⚠️ CameraManager::capture_raw_images: 这是假实现，请使用SimpleCameraManager");
        Ok(vec![(vec![0u8; 100], vec![0u8; 100])])
    }

    /// 假的配置设置
    pub fn configure_for_stage(&mut self, _stage: WorkflowStage) -> Result<(), String> {
        println!("⚠️ CameraManager::configure_for_stage: 这是假实现，请使用SimpleCameraManager");
        Ok(())
    }
}

// ==================== 说明 ====================
// 
// 测试代码已移至独立的测试程序：
// - src/bin/simple_camera_manager_test.rs - 完整的功能测试
// 
// 这样做的好处：
// 1. 分离关注点：库代码专注于功能实现
// 2. 独立测试：可以单独运行，便于调试
// 3. 硬件测试：需要真实硬件的测试更适合独立程序
// 4. 用户友好：提供交互式测试体验
