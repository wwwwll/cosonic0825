// alignment_workflow.rs - 光机合像检测工作流程
// 双线程架构：采集线程 + 处理线程
// 支持实时预览和阶段化合像检测

use std::sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use opencv::{core, imgcodecs, imgproc, prelude::*};
use tauri::{AppHandle, Emitter};
use serde::{Serialize, Deserialize};

use crate::camera_manager::{SimpleCameraManager, CameraError};
use crate::modules::{
    alignment::{AlignmentSystem, SingleEyePoseResult, DualEyeAlignmentResult, CenteringResult, AdjustmentVectors},
    param_io::*,
};

// ==================== 数据结构定义 ====================

/// 检测阶段枚举 (简化版 - 移除WorkflowStage依赖)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DetectionStage {
    Idle,                    // 空闲状态
    Loading,                 // 加载参数中
    Preview,                 // 预览模式
    LeftEyePoseCheck,        // 左眼姿态检测
    RightEyePoseCheck,       // 右眼姿态检测
    DualEyeAlignment,        // 双光机合像检测
    Completed,               // 检测完成
    Error { message: String }, // 错误状态
}

/// 帧数据结构 (原始数据版本)
#[derive(Clone)]
pub struct FrameData {
    pub left_image: Vec<u8>,
    pub right_image: Vec<u8>,
    pub timestamp: Instant,
}

/// 检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stage")]
pub enum DetectionResult {
    LeftEyePose {
        roll: f64,
        pitch: f64,
        yaw: f64,
        pass: bool,
        message: String,
    },
    RightEyePose {
        roll: f64,
        pitch: f64,
        yaw: f64,
        pass: bool,
        message: String,
    },
    DualEyeAlignment {
        mean_dx: f64,
        mean_dy: f64,
        rms: f64,
        p95: f64,
        max_err: f64,
        pass: bool,
        adjustment_hint: String,
    },
    Error {
        message: String,
    },
}

/// 环形缓冲区（优化版）
pub struct RingBuffer<T> {
    buffer: VecDeque<T>,
    capacity: usize,
    total_pushed: u64,
    dropped_count: u64,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            total_pushed: 0,
            dropped_count: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.total_pushed += 1;
        
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
            self.dropped_count += 1;
        }
        self.buffer.push_back(item);
    }

    pub fn latest(&self) -> Option<&T> {
        self.buffer.back()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// 获取性能统计
    pub fn get_stats(&self) -> (u64, u64, f64) {
        let drop_rate = if self.total_pushed > 0 {
            (self.dropped_count as f64 / self.total_pushed as f64) * 100.0
        } else {
            0.0
        };
        (self.total_pushed, self.dropped_count, drop_rate)
    }
}

// ==================== 主工作流程系统 ====================

pub struct AlignmentWorkflow {
    // 基础组件 (简化版)
    camera_manager: Arc<Mutex<SimpleCameraManager>>,
    alignment_system: Arc<Mutex<Option<AlignmentSystem>>>,
    app_handle: AppHandle,

    // 线程控制
    running: Arc<AtomicBool>,
    acquisition_thread: Option<thread::JoinHandle<()>>,
    processing_thread: Option<thread::JoinHandle<()>>,

    // 数据通信
    frame_buffer: Arc<Mutex<RingBuffer<FrameData>>>,
    stage: Arc<Mutex<DetectionStage>>,
    
    // 通道通信
    command_sender: Option<mpsc::Sender<WorkflowCommand>>,
}

/// 工作流程命令
#[derive(Debug)]
pub enum WorkflowCommand {
    StartPreview,
    StartDetection,
    NextStage,
    Reset,
    Stop,
}

impl AlignmentWorkflow {
    /// 创建合像检测工作流程 (SimpleCameraManager版本)
    pub fn new(
        app_handle: AppHandle,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("初始化合像检测工作流程 (SimpleCameraManager版本)...");

        // 创建SimpleCameraManager
        let camera_manager = Arc::new(Mutex::new(SimpleCameraManager::new()?));
        let frame_buffer = Arc::new(Mutex::new(RingBuffer::new(5))); // 保持最近5帧
        let stage = Arc::new(Mutex::new(DetectionStage::Idle));

        Ok(Self {
            camera_manager,
            alignment_system: Arc::new(Mutex::new(None)),
            app_handle,
            running: Arc::new(AtomicBool::new(false)),
            acquisition_thread: None,
            processing_thread: None,
            frame_buffer,
            stage,
            command_sender: None,
        })
    }

    /// 初始化合像检测系统（加载参数）
    pub fn initialize_alignment_system(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== 初始化合像检测系统 ===");
        
        // 更新状态
        *self.stage.lock().unwrap() = DetectionStage::Loading;
        self.emit_stage_update()?;

        // 加载标定参数
        let image_size = core::Size::new(2448, 2048);
        
        // 🔧 修正参数文件路径 - 使用yaml_last_param_file目录
        // 旧路径 (注释掉):
        // "left_camera_params.yaml",
        // "right_camera_params.yaml", 
        // "stereo_params.yaml",
        // "rectify_params.yaml",
        
        let alignment_sys = AlignmentSystem::new(
            image_size,
            "yaml_last_param_file/left_camera_params.yaml",
            "yaml_last_param_file/right_camera_params.yaml", 
            "yaml_last_param_file/stereo_params.yaml",
            "yaml_last_param_file/rectify_params.yaml",
        )?;

        *self.alignment_system.lock().unwrap() = Some(alignment_sys);
        
        println!("✓ 合像检测系统初始化完成");
        *self.stage.lock().unwrap() = DetectionStage::Idle;
        self.emit_stage_update()?;
        
        Ok(())
    }

    /// 启动工作流程（双线程模式 - SimpleCameraManager版本）
    pub fn start_workflow(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 启动合像检测工作流程...");
        
        if self.running.load(Ordering::SeqCst) {
            return Err("工作流程已经在运行中".into());
        }
        
        // [配置系统 - 已注释] 设置相机为合像检测模式
        // unsafe {
        //     crate::camera_ffi::set_camera_mode(2); // 2 = alignment mode
        // }
        // println!("📷 已设置相机为合像检测模式");
        
        // 创建SimpleCameraManager并启动
        {
            let mut cam = self.camera_manager.lock()
                .map_err(|e| format!("获取相机管理器失败: {}", e))?;
            cam.start()
                .map_err(|e| format!("启动相机失败: {:?}", e))?;
        }

        // 初始化系统（如果还没有）
        if self.alignment_system.lock().unwrap().is_none() {
            self.initialize_alignment_system()?;
        }

        // 优化OpenCV性能配置
        self.configure_opencv_performance()?;

        self.running.store(true, Ordering::SeqCst);
        
        // 创建命令通道
        let (cmd_tx, cmd_rx) = mpsc::channel();
        self.command_sender = Some(cmd_tx);

        // 启动采集线程
        self.start_acquisition_thread()?;
        
        // 启动处理线程
        self.start_processing_thread(cmd_rx)?;

        // 启动预览模式
        self.send_command(WorkflowCommand::StartPreview)?;

        println!("✓ 工作流程启动完成");
        Ok(())
    }

    /// 配置OpenCV性能优化
    fn configure_opencv_performance(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 设置OpenCV线程数为CPU核心数的一半，避免过度并行
        let cpu_cores = num_cpus::get();
        let opencv_threads = (cpu_cores / 2).max(1).min(4); // 限制在1-4之间
        
        #[cfg(feature = "opencv")]
        {
            opencv::core::set_num_threads(opencv_threads as i32)
                .map_err(|e| format!("设置OpenCV线程数失败: {}", e))?;
            println!("🔧 OpenCV线程数设置为: {} (CPU核心: {})", opencv_threads, cpu_cores);
        }

        // 启用OpenCV优化
        #[cfg(feature = "opencv")]
        {
            opencv::core::set_use_optimized(true)
                .map_err(|e| format!("启用OpenCV优化失败: {}", e))?;
            println!("🚀 OpenCV优化已启用");
        }

        Ok(())
    }

    /// 启动采集线程 (SimpleCameraManager版本)
    fn start_acquisition_thread(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let camera_manager = Arc::clone(&self.camera_manager);
        let frame_buffer = Arc::clone(&self.frame_buffer);
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            println!("📷 采集线程启动 (SimpleCameraManager版本)");
            
            // 相机已经在 start_workflow() 中启动，这里不需要重复启动
            // 移除重复的启动代码：
            // if let Err(e) = camera_manager.lock().unwrap().start() {
            //     eprintln!("相机启动失败: {:?}", e);
            //     return;
            // }

            let mut frame_count = 0u64;
            let mut last_stats_time = Instant::now();

            // 10fps = 100ms间隔
            let frame_interval = Duration::from_millis(100);
            let mut last_capture_time = Instant::now();

            while running.load(Ordering::SeqCst) {
                let now = Instant::now();
                
                // 控制帧率
                if now.duration_since(last_capture_time) >= frame_interval {
                    match camera_manager.lock().unwrap().get_current_frame() {
                        Ok((left_data, right_data)) => {
                            let frame = FrameData {
                                left_image: left_data,
                                right_image: right_data,
                                timestamp: now,
                            };

                            // 推入环形缓冲区
                            frame_buffer.lock().unwrap().push(frame);
                            frame_count += 1;
                            last_capture_time = now;
                        }
                        Err(e) => {
                            eprintln!("采集帧失败: {:?}", e);
                            // 检查是否需要停止
                            if !running.load(Ordering::SeqCst) {
                                break;
                            }
                            thread::sleep(Duration::from_millis(50));
                        }
                    }
                }

                // 统计信息（每5秒输出一次）
                if now.duration_since(last_stats_time) >= Duration::from_secs(5) {
                    println!("📊 采集统计: {}帧, 缓冲区: {}帧", 
                             frame_count, frame_buffer.lock().unwrap().len());
                    last_stats_time = now;
                }

                // 检查是否需要停止
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                
                thread::sleep(Duration::from_millis(10));
            }

            // 停止相机
            let _ = camera_manager.lock().unwrap().stop();
            println!("📷 采集线程结束");
        });

        self.acquisition_thread = Some(handle);
        Ok(())
    }

    /// 启动处理线程
    fn start_processing_thread(
        &mut self,
        cmd_rx: mpsc::Receiver<WorkflowCommand>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let frame_buffer = Arc::clone(&self.frame_buffer);
        let stage = Arc::clone(&self.stage);
        let alignment_system = Arc::clone(&self.alignment_system);
        let running = Arc::clone(&self.running);
        let app_handle = self.app_handle.clone();

        let handle = thread::spawn(move || {
            println!("🔄 处理线程启动");

            while running.load(Ordering::SeqCst) {
                // 处理命令
                if let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        WorkflowCommand::StartPreview => {
                            *stage.lock().unwrap() = DetectionStage::Preview;
                            let _ = app_handle.emit("alignment-stage", DetectionStage::Preview);
                        }
                        WorkflowCommand::StartDetection => {
                            *stage.lock().unwrap() = DetectionStage::LeftEyePoseCheck;
                            let _ = app_handle.emit("alignment-stage", DetectionStage::LeftEyePoseCheck);
                        }
                        WorkflowCommand::NextStage => {
                            // 处理阶段转换逻辑
                            Self::handle_stage_transition(&stage, &app_handle);
                        }
                        WorkflowCommand::Reset => {
                            *stage.lock().unwrap() = DetectionStage::Preview;
                            let _ = app_handle.emit("alignment-stage", DetectionStage::Preview);
                        }
                        WorkflowCommand::Stop => {
                            running.store(false, Ordering::SeqCst);
                            break;
                        }
                    }
                }

                // 根据当前阶段处理图像
                let current_stage = stage.lock().unwrap().clone();
                match current_stage {
                    DetectionStage::Preview => {
                        // 预览模式：定期发送预览图像
                        Self::handle_preview_mode(&frame_buffer, &app_handle);
                    }
                    DetectionStage::LeftEyePoseCheck |
                    DetectionStage::RightEyePoseCheck |
                    DetectionStage::DualEyeAlignment => {
                        // 检测模式：处理最新帧
                        Self::handle_detection_mode(
                            &frame_buffer,
                            &alignment_system,
                            &current_stage,
                            &app_handle,
                        );
                    }
                    _ => {}
                }

                thread::sleep(Duration::from_millis(50));
            }

            println!("🔄 处理线程结束");
        });

        self.processing_thread = Some(handle);
        Ok(())
    }

    /// 处理预览模式 (原始数据版本)
    fn handle_preview_mode(
        frame_buffer: &Arc<Mutex<RingBuffer<FrameData>>>,
        app_handle: &AppHandle,
    ) {
        if let Some(frame) = frame_buffer.lock().unwrap().latest() {
            // 每200ms发送一次预览图像（5fps预览）
            // 注意：这里发送原始数据，前端需要相应处理
            let preview_data = serde_json::json!({
                "left_preview_size": frame.left_image.len(),
                "right_preview_size": frame.right_image.len(),
                "timestamp": frame.timestamp.elapsed().as_millis(),
                "width": 2448,
                "height": 2048,
                "format": "grayscale"
            });
            
            let _ = app_handle.emit("alignment-preview", preview_data);
        }
        
        thread::sleep(Duration::from_millis(200));
    }

    /// 处理检测模式
    fn handle_detection_mode(
        frame_buffer: &Arc<Mutex<RingBuffer<FrameData>>>,
        alignment_system: &Arc<Mutex<Option<AlignmentSystem>>>,
        stage: &DetectionStage,
        app_handle: &AppHandle,
    ) {
        let start_time = Instant::now();
        
        let frame = {
            let buffer = frame_buffer.lock().unwrap();
            buffer.latest().cloned()
        };

        if let Some(frame_data) = frame {
            let mut alignment_sys = alignment_system.lock().unwrap();
            if let Some(ref mut sys) = *alignment_sys {
                match Self::process_detection_frame(sys, &frame_data, stage) {
                    Ok(result) => {
                        let processing_time = start_time.elapsed();
                        println!("🔍 检测处理耗时: {:.1}ms", processing_time.as_millis());
                        
                        let _ = app_handle.emit("alignment-result", result);
                    }
                    Err(e) => {
                        let error_result = DetectionResult::Error {
                            message: format!("检测处理失败: {}", e),
                        };
                        let _ = app_handle.emit("alignment-result", error_result);
                    }
                }
            }
        }

        // 检测模式下降低处理频率，避免CPU过载
        thread::sleep(Duration::from_millis(200));
    }

    /// 处理检测帧（优化版）
    fn process_detection_frame(
        alignment_sys: &mut AlignmentSystem,
        frame_data: &FrameData,
        stage: &DetectionStage,
    ) -> Result<DetectionResult, Box<dyn std::error::Error>> {
        // 将原始数据转换为OpenCV Mat
        let left_image = Self::raw_data_to_mat(&frame_data.left_image, 2448, 2048)?;
        let right_image = Self::raw_data_to_mat(&frame_data.right_image, 2448, 2048)?;

        // 根据检测阶段优化处理策略
        match stage {
            DetectionStage::LeftEyePoseCheck => {
                // 只检测左眼圆心，提高效率
                let (corners_left, _) = alignment_sys.detect_circles_grid(
                    &left_image,
                    &right_image, // 仍需传入，但内部可以优化只处理左眼
                    "yaml_last_param_file/rectify_maps.yaml", // 🔧 修正路径
                )?;
                
                // 使用向后兼容的左眼姿态检测方法
                let result = alignment_sys.check_left_eye_pose(&corners_left)?;
                Ok(DetectionResult::LeftEyePose {
                    roll: result.roll,
                    pitch: result.pitch,
                    yaw: result.yaw,
                    pass: result.pass,
                    message: if result.pass {
                        "✓ 左眼姿态检测通过".to_string()
                    } else {
                        format!("❌ 左眼姿态超出容差 - roll={:.3}°, pitch={:.3}°, yaw={:.3}°", 
                               result.roll, result.pitch, result.yaw)
                    },
                })
            }
            DetectionStage::RightEyePoseCheck => {
                // 只检测右眼圆心
                let (_, corners_right) = alignment_sys.detect_circles_grid(
                    &left_image,
                    &right_image,
                    "yaml_last_param_file/rectify_maps.yaml", // 🔧 修正路径
                )?;
                
                // 使用向后兼容的右眼姿态检测方法
                let result = alignment_sys.check_right_eye_pose(&corners_right)?;
                Ok(DetectionResult::RightEyePose {
                    roll: result.roll,
                    pitch: result.pitch,
                    yaw: result.yaw,
                    pass: result.pass,
                    message: if result.pass {
                        "✓ 右眼姿态检测通过".to_string()
                    } else {
                        format!("❌ 右眼姿态超出容差 - roll={:.3}°, pitch={:.3}°, yaw={:.3}°", 
                               result.roll, result.pitch, result.yaw)
                    },
                })
            }
            DetectionStage::DualEyeAlignment => {
                // 双眼同时检测，最高精度
                let (corners_left, corners_right) = alignment_sys.detect_circles_grid(
                    &left_image,
                    &right_image,
                    "yaml_last_param_file/rectify_maps.yaml", // 🔧 修正路径
                )?;
                
                let result = alignment_sys.check_dual_eye_alignment(&corners_left, &corners_right, true)?;
                let adjustment_hint = format!(
                    "调整提示: Δx={:.3}px {}, Δy={:.3}px {}",
                    result.mean_dx,
                    if result.mean_dx > 0.0 { "(右眼向左调)" } else { "(右眼向右调)" },
                    result.mean_dy,
                    if result.mean_dy < 0.0 { "(右眼向上调)" } else { "(右眼向下调)" }
                );

                Ok(DetectionResult::DualEyeAlignment {
                    mean_dx: result.mean_dx,
                    mean_dy: result.mean_dy,
                    rms: result.rms,
                    p95: result.p95,
                    max_err: result.max_err,
                    pass: result.pass,
                    adjustment_hint,
                })
            }
            _ => Err("不支持的检测阶段".into()),
        }
    }

    /// 将原始数据转换为OpenCV Mat
    fn raw_data_to_mat(data: &[u8], width: i32, height: i32) -> Result<core::Mat, opencv::Error> {
        // 创建空的Mat
        let mut mat = core::Mat::new_rows_cols_with_default(
            height,
            width,
            core::CV_8UC1,
            core::Scalar::default(),
        )?;
        
        // 将数据拷贝到Mat中
        let mat_data = mat.data_mut();
        let expected_size = (width * height) as usize;
        
        if data.len() >= expected_size {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    mat_data,
                    expected_size,
                );
            }
        } else {
            return Err(opencv::Error::new(
                opencv::core::StsError, 
                format!("数据长度不足: 需要{}字节，实际{}字节", expected_size, data.len())
            ));
        }
        
        Ok(mat)
    }

    /// 处理阶段转换
    fn handle_stage_transition(
        stage: &Arc<Mutex<DetectionStage>>,
        app_handle: &AppHandle,
    ) {
        let mut current_stage = stage.lock().unwrap();
        let next_stage = match *current_stage {
            DetectionStage::LeftEyePoseCheck => DetectionStage::RightEyePoseCheck,
            DetectionStage::RightEyePoseCheck => DetectionStage::DualEyeAlignment,
            DetectionStage::DualEyeAlignment => DetectionStage::Completed,
            _ => return,
        };

        *current_stage = next_stage.clone();
        let _ = app_handle.emit("alignment-stage", next_stage);
    }

    // ==================== 公共接口方法 ====================

    /// 发送命令
    pub fn send_command(&self, cmd: WorkflowCommand) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref sender) = self.command_sender {
            sender.send(cmd)?;
        }
        Ok(())
    }

    /// 开始检测
    pub fn start_detection(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(WorkflowCommand::StartDetection)
    }

    /// 下一阶段
    pub fn next_stage(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(WorkflowCommand::NextStage)
    }

    /// 重置到预览模式
    pub fn reset_to_preview(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_command(WorkflowCommand::Reset)
    }

    /// 停止工作流程
    pub fn stop_workflow(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        println!("=== 停止合像检测工作流程 ===");
        
        // 立即设置停止标志
        self.running.store(false, Ordering::SeqCst);
        
        // 发送停止命令
        if let Some(ref sender) = self.command_sender {
            let _ = sender.send(WorkflowCommand::Stop);
        }
        
        // 强制停止相机（如果线程没有及时响应）
        if let Ok(camera_manager) = self.camera_manager.lock() {
            let _ = camera_manager.stop();
            println!("🛑 强制停止相机");
        }
        
        // 等待线程结束（设置超时）
        if let Some(handle) = self.acquisition_thread.take() {
            println!("⏳ 等待采集线程结束...");
            match handle.join() {
                Ok(_) => println!("✓ 采集线程已结束"),
                Err(e) => println!("⚠️ 采集线程结束异常: {:?}", e),
            }
        }
        
        if let Some(handle) = self.processing_thread.take() {
            println!("⏳ 等待处理线程结束...");
            match handle.join() {
                Ok(_) => println!("✓ 处理线程已结束"),
                Err(e) => println!("⚠️ 处理线程结束异常: {:?}", e),
            }
        }

        println!("✓ 工作流程已停止");
        Ok(())
    }

    /// 获取当前状态
    pub fn get_current_stage(&self) -> DetectionStage {
        self.stage.lock().unwrap().clone()
    }

    /// 发送状态更新事件
    fn emit_stage_update(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stage = self.get_current_stage();
        self.app_handle.emit("alignment-stage", stage)?;
        Ok(())
    }

    /// 获取当前预览帧（Base64格式）
    pub fn get_current_preview_frame(&self) -> Result<crate::commands::alignment_commands::CameraPreviewData, Box<dyn std::error::Error>> {
        use base64::{Engine as _, engine::general_purpose};
        
        // 从缓冲区获取最新帧
        let frame_data = {
            let buffer = self.frame_buffer.lock().unwrap();
            buffer.latest().cloned()
        };
        
        if let Some(frame) = frame_data {
            // // ===== DEBUG START: 可在正式版本中删除 =====
            // // 🔍 DEBUG: 保存原始图像用于调试（每100次调用保存一次）
            // static mut DEBUG_COUNTER: u32 = 0;
            // unsafe {
            //     DEBUG_COUNTER += 1;
            //     if DEBUG_COUNTER % 100 == 0 {
            //         self.save_debug_images(&frame)?;
            //     }
            // }
            // // ===== DEBUG END: 可在正式版本中删除 =====
            
            // 将原始数据转换为Base64图像
            let left_base64 = raw_data_to_base64_image(&frame.left_image, 2448, 2048)?;
            let right_base64 = raw_data_to_base64_image(&frame.right_image, 2448, 2048)?;
            
            Ok(crate::commands::alignment_commands::CameraPreviewData {
                left_image_base64: left_base64,
                right_image_base64: right_base64,
                timestamp: frame.timestamp.elapsed().as_millis() as u64,
                width: 2448,
                height: 2048,
                fps: 10.0,
            })
        } else {
            Err("没有可用的帧数据".into())
        }
    }

    /// 获取当前检测结果
    pub fn get_current_detection_result(&self) -> Result<DetectionResult, Box<dyn std::error::Error>> {
        // 从缓冲区获取最新帧
        let frame_data = {
            let buffer = self.frame_buffer.lock().unwrap();
            buffer.latest().cloned()
        };
        
        if let Some(frame) = frame_data {
            let mut alignment_sys = self.alignment_system.lock().unwrap();
            if let Some(ref mut sys) = *alignment_sys {
                // 执行完整的检测流程
                let left_image = Self::raw_data_to_mat(&frame.left_image, 2448, 2048)?;
                let right_image = Self::raw_data_to_mat(&frame.right_image, 2448, 2048)?;
                
                // 使用单帧检测方法
                self.detect_single_frame_internal(sys, left_image, right_image)
            } else {
                Err("合像检测系统未初始化".into())
            }
        } else {
            Err("没有可用的帧数据".into())
        }
    }
    
    /// 内部单帧检测方法
    fn detect_single_frame_internal(
        &self,
        alignment_sys: &mut crate::modules::alignment::AlignmentSystem,
        left_image: opencv::core::Mat,
        right_image: opencv::core::Mat,
    ) -> Result<DetectionResult, Box<dyn std::error::Error>> {
        // 1. 执行圆心检测
        let (left_corners, right_corners) = alignment_sys.detect_circles_grid(
            &left_image,
            &right_image,
            "yaml_last_param_file/rectify_maps.yaml", // 🔧 修正路径
        )?;
        
        // 2. 左眼姿态检测
        let left_pose = alignment_sys.check_left_eye_pose(&left_corners)?;
        if !left_pose.pass {
            return Ok(DetectionResult::LeftEyePose {
                roll: left_pose.roll,
                pitch: left_pose.pitch,
                yaw: left_pose.yaw,
                pass: false,
                message: format!("❌ 左眼姿态超出容差 - roll={:.3}°, pitch={:.3}°, yaw={:.3}°", 
                               left_pose.roll, left_pose.pitch, left_pose.yaw),
            });
        }
        
        // 3. 右眼姿态检测
        let right_pose = alignment_sys.check_right_eye_pose(&right_corners)?;
        if !right_pose.pass {
            return Ok(DetectionResult::RightEyePose {
                roll: right_pose.roll,
                pitch: right_pose.pitch,
                yaw: right_pose.yaw,
                pass: false,
                message: format!("❌ 右眼姿态超出容差 - roll={:.3}°, pitch={:.3}°, yaw={:.3}°", 
                               right_pose.roll, right_pose.pitch, right_pose.yaw),
            });
        }
        
        // 4. 双眼合像检测
        let alignment_result = alignment_sys.check_dual_eye_alignment(&left_corners, &right_corners, false)?;
        let adjustment_hint = format!(
            "调整提示: Δx={:.3}px {}, Δy={:.3}px {}",
            alignment_result.mean_dx,
            if alignment_result.mean_dx > 0.0 { "(右眼向左调)" } else { "(右眼向右调)" },
            alignment_result.mean_dy,
            if alignment_result.mean_dy < 0.0 { "(右眼向上调)" } else { "(右眼向下调)" }
        );
        
        Ok(DetectionResult::DualEyeAlignment {
            mean_dx: alignment_result.mean_dx,
            mean_dy: alignment_result.mean_dy,
            rms: alignment_result.rms,
            p95: alignment_result.p95,
            max_err: alignment_result.max_err,
            pass: alignment_result.pass,
            adjustment_hint,
        })
    }

    /// 获取系统性能统计
    pub fn get_performance_stats(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let buffer_stats = {
            let buffer = self.frame_buffer.lock().unwrap();
            buffer.get_stats()
        };

        let stats = serde_json::json!({
            "buffer": {
                "total_frames": buffer_stats.0,
                "dropped_frames": buffer_stats.1,
                "drop_rate_percent": buffer_stats.2,
                "current_size": self.frame_buffer.lock().unwrap().len(),
                "capacity": 5
            },
            "system": {
                "cpu_cores": num_cpus::get(),
                "opencv_threads": 2, // 已在configure_opencv_performance中设置
                "thread_count": 2,   // 采集线程 + 处理线程
                "running": self.running.load(Ordering::SeqCst)
            },
            "stage": self.get_current_stage()
        });

        Ok(stats)
    }

    /// 手动保存调试图像（公开接口）
    pub fn save_debug_images_manual(&self) -> Result<(), Box<dyn std::error::Error>> {
        let frame_data = {
            let buffer = self.frame_buffer.lock().unwrap();
            buffer.latest().cloned()
        };
        
        if let Some(frame) = frame_data {
            self.save_debug_images(&frame)
        } else {
            Err("没有可用的帧数据".into())
        }
    }
    
    // ===== DEBUG START: 可在正式版本中删除 =====
    /// 🔍 DEBUG: 保存调试图像
    fn save_debug_images(&self, frame: &FrameData) -> Result<(), Box<dyn std::error::Error>> {
        use opencv::{imgcodecs, core::Vector};
        use std::time::SystemTime;
        
        println!("📸 保存调试图像...");
        
        // 转换为Mat格式
        let left_mat = Self::raw_data_to_mat(&frame.left_image, 2448, 2048)?;
        let right_mat = Self::raw_data_to_mat(&frame.right_image, 2448, 2048)?;
        
        // 生成时间戳文件名
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 确保调试目录存在
        let debug_dir = "src-tauri/captures/alignment_workflow_debug";
        std::fs::create_dir_all(debug_dir)?;
        
        let left_path = format!("{}/debug_left_{}.png", debug_dir, timestamp);
        let right_path = format!("{}/debug_right_{}.png", debug_dir, timestamp);
        
        // 保存原始图像
        imgcodecs::imwrite(&left_path, &left_mat, &Vector::new())?;
        imgcodecs::imwrite(&right_path, &right_mat, &Vector::new())?;
        println!("✅ 已保存调试图像: {} 和 {}", left_path, right_path);
        
        // 如果alignment_system已初始化，也保存重映射后的图像
        if let Ok(alignment_sys) = self.alignment_system.lock() {
            if let Some(sys) = alignment_sys.as_ref() {
                // 确保重映射矩阵已加载
                if sys.get_rectify_maps().is_some() {
                    println!("📸 保存重映射后的图像...");
                    
                    // 执行重映射
                    let (left_map1, left_map2, right_map1, right_map2) = sys.get_rectify_maps().unwrap();
                    let rectifier = sys.get_rectifier();
                    
                    let left_rect = rectifier.remap_image_adaptive(&left_mat, left_map1, left_map2)?;
                    let right_rect = rectifier.remap_image_adaptive(&right_mat, right_map1, right_map2)?;
                    
                    let left_rect_path = format!("{}/debug_left_rectified_{}.png", debug_dir, timestamp);
                    let right_rect_path = format!("{}/debug_right_rectified_{}.png", debug_dir, timestamp);
                    
                    imgcodecs::imwrite(&left_rect_path, &left_rect, &Vector::new())?;
                    imgcodecs::imwrite(&right_rect_path, &right_rect, &Vector::new())?;
                    println!("✅ 已保存重映射图像: {} 和 {}", left_rect_path, right_rect_path);
                }
            }
        }
        
        Ok(())
    }
    // ===== DEBUG END: 可在正式版本中删除 =====
    
    /// 打印性能报告
    pub fn print_performance_report(&self) {
        if let Ok(stats) = self.get_performance_stats() {
            println!("📊 === 性能统计报告 ===");
            if let Some(buffer) = stats.get("buffer") {
                println!("🗂️  缓冲区统计:");
                println!("   总帧数: {}", buffer.get("total_frames").unwrap_or(&serde_json::Value::Null));
                println!("   丢帧数: {}", buffer.get("dropped_frames").unwrap_or(&serde_json::Value::Null));
                println!("   丢帧率: {:.2}%", buffer.get("drop_rate_percent").unwrap_or(&serde_json::Value::Null));
                println!("   当前大小: {}/{}", 
                    buffer.get("current_size").unwrap_or(&serde_json::Value::Null),
                    buffer.get("capacity").unwrap_or(&serde_json::Value::Null));
            }
            if let Some(system) = stats.get("system") {
                println!("💻 系统配置:");
                println!("   CPU核心: {}", system.get("cpu_cores").unwrap_or(&serde_json::Value::Null));
                println!("   OpenCV线程: {}", system.get("opencv_threads").unwrap_or(&serde_json::Value::Null));
                println!("   工作线程: {}", system.get("thread_count").unwrap_or(&serde_json::Value::Null));
                println!("   运行状态: {}", system.get("running").unwrap_or(&serde_json::Value::Null));
            }
            println!("========================");
        }
    }
    
    /// 🎯 单帧检测方法 - 适合前端Tauri命令调用
    /// 
    /// 这个方法封装了完整的检测流程：圆心检测 -> 姿态分析 -> 合像检测
    /// 适合前端按钮触发的单次检测操作
    pub fn detect_single_frame(
        &mut self,
        left_image: core::Mat,
        right_image: core::Mat,
    ) -> Result<DetectionResult, Box<dyn std::error::Error>> {
        println!("🎯 工作流单帧检测开始...");
        let start_time = Instant::now();
        
        // 确保alignment_system已初始化
        let mut alignment_sys = self.alignment_system.lock().unwrap();
        if alignment_sys.is_none() {
            return Err("合像检测系统未初始化".into());
        }
        
        let sys = alignment_sys.as_mut().unwrap();
        
        // 1. 执行圆心检测
        let (left_corners, right_corners) = sys.detect_circles_grid(
            &left_image,
            &right_image,
            "yaml_last_param_file/rectify_maps.yaml", // 🔧 修正路径
        )?;
        
        // 2. 左眼姿态检测
        let left_pose = sys.check_left_eye_pose(&left_corners)?;
        if !left_pose.pass {
            return Ok(DetectionResult::LeftEyePose {
                roll: left_pose.roll,
                pitch: left_pose.pitch,
                yaw: left_pose.yaw,
                pass: false,
                message: format!("❌ 左眼姿态超出容差 - roll={:.3}°, pitch={:.3}°, yaw={:.3}°", 
                               left_pose.roll, left_pose.pitch, left_pose.yaw),
            });
        }
        
        // 3. 右眼姿态检测
        let right_pose = sys.check_right_eye_pose(&right_corners)?;
        if !right_pose.pass {
            return Ok(DetectionResult::RightEyePose {
                roll: right_pose.roll,
                pitch: right_pose.pitch,
                yaw: right_pose.yaw,
                pass: false,
                message: format!("❌ 右眼姿态超出容差 - roll={:.3}°, pitch={:.3}°, yaw={:.3}°", 
                               right_pose.roll, right_pose.pitch, right_pose.yaw),
            });
        }
        
        // 4. 双眼合像检测
        let alignment_result = sys.check_dual_eye_alignment(&left_corners, &right_corners, true)?;
        let adjustment_hint = format!(
            "调整提示: Δx={:.3}px {}, Δy={:.3}px {}",
            alignment_result.mean_dx,
            if alignment_result.mean_dx > 0.0 { "(右眼向左调)" } else { "(右眼向右调)" },
            alignment_result.mean_dy,
            if alignment_result.mean_dy < 0.0 { "(右眼向上调)" } else { "(右眼向下调)" }
        );
        
        let processing_time = start_time.elapsed();
        println!("✓ 工作流单帧检测完成，总耗时: {:.1} ms", processing_time.as_millis());
        
        Ok(DetectionResult::DualEyeAlignment {
            mean_dx: alignment_result.mean_dx,
            mean_dy: alignment_result.mean_dy,
            rms: alignment_result.rms,
            p95: alignment_result.p95,
            max_err: alignment_result.max_err,
            pass: alignment_result.pass,
            adjustment_hint,
        })
    }
    
    /// 🎯 仅执行圆心检测 - 用于快速验证图像质量
    pub fn detect_circles_only(
        &mut self,
        left_image: core::Mat,
        right_image: core::Mat,
    ) -> Result<(opencv::core::Vector<opencv::core::Point2f>, opencv::core::Vector<opencv::core::Point2f>), Box<dyn std::error::Error>> {
        let mut alignment_sys = self.alignment_system.lock().unwrap();
        if alignment_sys.is_none() {
            return Err("合像检测系统未初始化".into());
        }
        
        let sys = alignment_sys.as_mut().unwrap();
        // 🔧 修正重映射矩阵路径 - 使用yaml_last_param_file目录
        // 旧路径: "rectify_maps.yaml"
        sys.detect_circles_grid(&left_image, &right_image, "yaml_last_param_file/rectify_maps.yaml")
    }
}

impl Drop for AlignmentWorkflow {
    fn drop(&mut self) {
        let _ = self.stop_workflow();
    }
}

// ==================== 辅助函数 ====================

/// 将原始图像数据转换为Base64格式的PNG图像
fn raw_data_to_base64_image(raw_data: &[u8], width: i32, height: i32) -> Result<String, Box<dyn std::error::Error>> {
    use base64::{Engine as _, engine::general_purpose};
    use opencv::{core, imgcodecs, prelude::*};
    
    // 将原始数据转换为OpenCV Mat
    let mat = AlignmentWorkflow::raw_data_to_mat(raw_data, width, height)?;
    
    // 创建缩略图 (缩放到400x300以减少传输数据量)
    let thumbnail_width = 400;
    let thumbnail_height = (height as f32 * thumbnail_width as f32 / width as f32) as i32;
    
    let mut resized_mat = core::Mat::default();
    opencv::imgproc::resize(
        &mat,
        &mut resized_mat,
        core::Size::new(thumbnail_width, thumbnail_height),
        0.0,
        0.0,
        opencv::imgproc::INTER_LINEAR,
    )?;
    
    // 转换为PNG格式的字节数组
    let mut buffer = opencv::core::Vector::<u8>::new();
    imgcodecs::imencode(".png", &resized_mat, &mut buffer, &opencv::core::Vector::new())?;
    
    // 转换为Base64
    let base64_data = general_purpose::STANDARD.encode(buffer.as_slice());
    Ok(format!("data:image/png;base64,{}", base64_data))
} 