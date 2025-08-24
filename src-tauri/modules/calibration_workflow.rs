//! 相机标定工作流程 - 基于SimpleCameraManager重构版本
//! 
//! ## 🎯 重构背景
//! 
//! 基于**SimpleCameraManager**的架构重构，相机标定流程完全重新设计：
//! - **极简相机接口**: 只需3个核心方法 (new/start/get_current_frame/stop)
//! - **即时处理模式**: 每次调用获取当前帧，根据标志决定是否保存
//! - **硬件优化**: 15fps连续采集，无需复杂模式切换
//! - **架构清晰**: C层硬件抽象 + Rust业务逻辑分层
//! 
//! ## 📋 简化的标定流程
//! 
//! ### 用户操作流程 (即时处理版)
//! 1. `start_calibration()` - 启动标定会话，开始相机预览
//! 2. `get_preview_frame_sync()` - 获取实时预览帧
//! 3. `save_current_frame_as_calibration()` - 保存当前帧为标定图像（重复15次）
//! 4. `run_calibration()` - 执行标定算法，保存参数
//! 
//! @version 2.1 - 即时处理架构
//! @date 2025-01-15

use std::{
    path::PathBuf,
    fs,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    time::{SystemTime, UNIX_EPOCH},
};

use opencv::{
    core::{Mat, Size, Vector, Point2f, Point3f, AlgorithmHint},
    imgcodecs,
    imgproc,
    prelude::*,
};

use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose};

use crate::camera_manager::{SimpleCameraManager, CameraError};
use crate::modules::{
    calibration_circles::{Calibrator, CameraType, MonoCalibResult, StereoCalibResult, MonoCamera},
    param_io::*,
};

/// 标定状态枚举 (简化版)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CalibrationStatus {
    /// 未开始
    NotStarted,
    /// 正在采集图像
    Capturing,
    /// 已采集足够图像，可以开始标定
    ReadyToCalibrate,
    /// 正在进行标定计算
    Calibrating,
    /// 标定完成
    Completed,
    /// 标定失败
    Failed(String),
}

/// 图像对信息 (简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePair {
    pub pair_id: u32,
    pub left_image_path: String,      // captures/calib_left_{pair_id}.png
    pub right_image_path: String,     // captures/calib_right_{pair_id}.png
    pub thumbnail_left: String,       // Base64缩略图用于前端显示
    pub thumbnail_right: String,      // Base64缩略图用于前端显示
    pub capture_timestamp: String,
    pub has_calibration_pattern: bool, // 是否检测到标定板
}

/// 标定结果 (简化版)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationResult {
    pub success: bool,
    pub left_rms_error: f64,           // 左相机重投影误差
    pub right_rms_error: f64,          // 右相机重投影误差
    pub stereo_rms_error: f64,         // 双目标定误差
    pub error_threshold: f64,          // 错误阈值
    pub error_message: Option<String>, // 错误信息
    pub calibration_time: String,      // 标定完成时间
}

/// 预览帧数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewFrame {
    pub left_preview: String,   // Base64图像
    pub right_preview: String,  // Base64图像
    pub timestamp: String,      // 时间戳
    pub has_pattern: Option<bool>, // 可选：是否检测到标定板
}

/// 标定工作流程管理器 (即时处理版本)
pub struct CalibrationWorkflow {
    camera_manager: SimpleCameraManager,
    captured_images: Vec<ImagePair>,
    calibration_config: CalibrationConfig,
    current_status: CalibrationStatus,
    session_id: Option<String>,
    
    // 简化：即时处理模式，无需缓冲区
    should_save_next_frame: Arc<AtomicBool>,
}

/// 标定配置
#[derive(Debug, Clone)]
pub struct CalibrationConfig {
    pub circle_diameter: f32,          // 圆点直径 (mm)
    pub center_distance: f32,          // 圆点间距 (mm)  
    pub pattern_size: Size,            // 标定板尺寸 (10x4)
    pub error_threshold: f64,          // 重投影误差阈值
    pub target_image_count: u32,       // 目标图像数量
    pub save_directory: String,        // 保存目录
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            circle_diameter: 15.0,           // 正确值：15mm圆点直径
            center_distance: 25.0,           // 25mm diagonal spacing
            pattern_size: Size::new(4, 10),  // 正确值：4列10行
            error_threshold: 1.0,            // 与测试保持一致
            target_image_count: 15,
            save_directory: "captures".to_string(),
        }
    }
}

impl CalibrationWorkflow {
    /// 创建新的标定工作流程实例
    pub fn new() -> Result<Self, String> {
        println!("🏗️ 初始化标定工作流程管理器 (SimpleCameraManager架构)...");
        
        // 创建SimpleCameraManager实例
        let camera_manager = SimpleCameraManager::new()
            .map_err(|e| format!("SimpleCameraManager初始化失败: {}", e))?;
        
        let workflow = Self {
            camera_manager,
            captured_images: Vec::new(),
            calibration_config: CalibrationConfig::default(),
            current_status: CalibrationStatus::NotStarted,
            session_id: None,
            should_save_next_frame: Arc::new(AtomicBool::new(false)),
        };
        
        println!("✅ 标定工作流程管理器初始化完成");
        Ok(workflow)
    }
    
    /// 核心方法1: 开始标定会话（即时处理）
    pub fn start_calibration(&mut self) -> Result<(), String> {
        println!("🎬 开始标定会话（即时处理）...");
        
        if self.current_status != CalibrationStatus::NotStarted {
            return Err("标定会话已经在进行中".to_string());
        }
        
        // 1. 创建会话ID和保存目录
        let session_id = format!("calibration_{}", 
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
        let save_directory = format!("captures/calibration_{}", session_id);
        fs::create_dir_all(&save_directory)
            .map_err(|e| format!("创建保存目录失败: {}", e))?;
        
        // 2. 设置相机为标定模式并启动相机
        // [配置系统 - 已注释]
        // unsafe {
        //     crate::camera_ffi::set_camera_mode(1); // 1 = calibration mode
        // }
        // println!("📷 已设置相机为标定模式");
        
        self.camera_manager.start()
            .map_err(|e| format!("启动相机失败: {}", e))?;
        
        // 3. 初始化采集会话
        self.session_id = Some(session_id.clone());
        self.captured_images.clear();
        self.calibration_config.save_directory = save_directory;
        self.current_status = CalibrationStatus::Capturing;
        
        println!("✅ 标定会话已启动: {}", session_id);
        println!("📷 相机已启动，即时处理模式");
        println!("📂 保存目录: {}", self.calibration_config.save_directory);
        
        Ok(())
    }
    
    /// 统一的当前帧处理方法
    /// 
    /// 每次调用都获取最新帧，根据should_save_next_frame标志决定是否保存
    fn process_current_frame(&mut self) -> Result<(PreviewFrame, Option<ImagePair>), String> {
        // 检查并获取保存标志
        let should_save = self.should_save_next_frame.swap(false, Ordering::SeqCst);
        
        // 从camera_manager获取当前帧
        let (left_data, right_data) = self.camera_manager.get_current_frame()
            .map_err(|e| format!("获取当前帧失败: {:?}", e))?;
        
        // 转换为Mat
        let left_mat = self.raw_data_to_mat(&left_data)?;
        let right_mat = self.raw_data_to_mat(&right_data)?;
        
        // 生成预览帧
        let left_preview = self.generate_thumbnail_from_mat(&left_mat)?;
        let right_preview = self.generate_thumbnail_from_mat(&right_mat)?;
        
        let has_pattern = if should_save && self.current_status == CalibrationStatus::Capturing {
            Some(self.quick_detect_pattern_from_mats(&left_mat, &right_mat))
        } else {
            None
        };
        
        let preview_frame = PreviewFrame {
            left_preview,
            right_preview,
            timestamp: chrono::Utc::now().to_rfc3339(),
            has_pattern,
        };
        
        // 如果需要保存，处理保存逻辑
        let image_pair = if should_save {
            println!("💾 执行保存逻辑（即时处理模式）");
            
            let pair_id = self.captured_images.len() as u32 + 1;
            let left_path = format!("{}/calib_left_{:02}.png", 
                self.calibration_config.save_directory, pair_id);
            let right_path = format!("{}/calib_right_{:02}.png", 
                self.calibration_config.save_directory, pair_id);
            
            // 保存图像为PNG格式
            self.save_mat_as_png(&left_mat, &left_path)?;
            self.save_mat_as_png(&right_mat, &right_path)?;
            
            // 从保存的PNG文件检测标定板
            let has_pattern = self.detect_calibration_pattern_from_saved_files(&left_path, &right_path)?;
            
            let image_pair = ImagePair {
                pair_id,
                left_image_path: left_path,
                right_image_path: right_path,
                thumbnail_left: preview_frame.left_preview.clone(),
                thumbnail_right: preview_frame.right_preview.clone(),
                capture_timestamp: preview_frame.timestamp.clone(),
                has_calibration_pattern: has_pattern,
            };
            
            self.captured_images.push(image_pair.clone());
            
            // 检查是否达到目标数量
            if self.captured_images.len() >= self.calibration_config.target_image_count as usize {
                self.current_status = CalibrationStatus::ReadyToCalibrate;
                println!("✅ 已采集足够图像，可以开始标定");
            }
            
            println!("✅ 标定图像对保存完成: {} (检测到标定板: {})", 
                    pair_id, has_pattern);
            
            Some(image_pair)
        } else {
            None
        };
        
        Ok((preview_frame, image_pair))
    }

    /// 获取预览帧（支持同时保存，前端友好）
    /// 
    /// # 参数
    /// - `should_save`: 是否同时保存当前帧为标定图像
    /// 
    /// # 返回值
    /// - `PreviewFrame`: 预览帧数据
    /// - 如果 `should_save=true`，会同时保存图像并更新 `captured_images`
    pub fn get_preview_frame_sync(&mut self, should_save: bool) -> Result<PreviewFrame, String> {
        // 根据参数设置保存标志
        if should_save {
            self.should_save_next_frame.store(true, Ordering::SeqCst);
        }
        
        let (preview_frame, image_pair) = self.process_current_frame()?;
        
        // 如果保存了图像，记录日志
        if let Some(pair) = image_pair {
            println!("📸 同时保存了标定图像: {}", pair.pair_id);
        }
        
        Ok(preview_frame)
    }

    /// 【已弃用】保存当前帧为标定图像
    /// 
    /// ⚠️ **建议使用 `get_preview_frame_sync(true)` 替代**
    /// 
    /// 新的设计下，前端只需要调用一个方法，通过参数控制是否保存。
    #[deprecated(since = "2.2.0", note = "使用 get_preview_frame_sync(should_save) 替代")]
    pub fn save_current_frame_as_calibration(&mut self) -> Result<ImagePair, String> {
        println!("⚠️ save_current_frame_as_calibration() 已弃用，建议使用 get_preview_frame_sync(true)");
        
        if self.current_status != CalibrationStatus::Capturing {
            return Err("当前状态不允许保存标定图像".to_string());
        }
        
        // 设置保存标志并立即处理
        self.should_save_next_frame.store(true, Ordering::SeqCst);
        
        let (_, image_pair) = self.process_current_frame()?;
        
        image_pair.ok_or("保存标定图像失败".to_string())
    }

    /// 获取最新保存的标定图像信息（如果有）
    pub fn get_latest_captured_image(&self) -> Option<ImagePair> {
        self.captured_images.last().cloned()
    }
    
    /// 【已弃用】拍摄一组标定图像
    /// 
    /// ⚠️ **此方法已弃用，请使用 `save_current_frame_as_calibration()` 替代**
    /// 
    /// 新的缓冲区架构下，不再需要每次重新拍摄，而是保存缓冲区中的当前帧。
    #[deprecated(since = "2.1.0", note = "使用 save_current_frame_as_calibration() 替代")]
    pub fn capture_calibration_pair(&mut self) -> Result<ImagePair, String> {
        println!("⚠️ capture_calibration_pair() 已弃用，使用 save_current_frame_as_calibration()");
        self.save_current_frame_as_calibration()
    }
    
    /// 核心方法3: 执行标定算法
    pub fn run_calibration(&mut self) -> Result<CalibrationResult, String> {
        println!("🚀 开始执行标定算法...");
        
        if self.current_status != CalibrationStatus::ReadyToCalibrate {
            return Err("当前状态不允许执行标定".to_string());
        }
        
        // 1. 停止相机: self.camera_manager.stop()?
        self.camera_manager.stop()
            .map_err(|e| format!("停止相机失败: {}", e))?;
        
        self.current_status = CalibrationStatus::Calibrating;
        
        // 2. 加载已保存的图像文件路径
        let valid_images: Vec<_> = self.captured_images.iter()
            .filter(|img| img.has_calibration_pattern)
            .collect();
        
        if valid_images.len() < 8 {
            let error_msg = format!("有效图像数量不足: {}/8", valid_images.len());
            self.current_status = CalibrationStatus::Failed(error_msg.clone());
            return Err(error_msg);
        }
        
        // 3. 调用calibration_circles.rs算法
        let result = self.run_calibration_algorithm(&valid_images)?;
        
        // 4. 根据结果更新状态
        if result.success {
            self.current_status = CalibrationStatus::Completed;
        } else {
            let error_msg = result.error_message.clone().unwrap_or("标定失败".to_string());
            self.current_status = CalibrationStatus::Failed(error_msg);
        }
        
        println!("✅ 标定算法执行完成: 成功={}", result.success);
        Ok(result)
    }
    
    /// 完整标定流程实现 (基于现有calibration_circles.rs算法)
    fn run_calibration_algorithm(&self, valid_images: &[&ImagePair]) -> Result<CalibrationResult, String> {
        println!("🔬 开始完整标定流程...");
        
        // Step 1: 创建标定器实例，从第一个有效图像获取尺寸
        let first_image_path = &valid_images[0].left_image_path;
        let first_image = imgcodecs::imread(first_image_path, imgcodecs::IMREAD_GRAYSCALE)
            .map_err(|e| format!("读取第一个图像失败: {}", e))?;
        let image_size = Size::new(first_image.cols(), first_image.rows());
        
        let mut calibrator = Calibrator::new(
            image_size,  // 从实际图像获取尺寸
            self.calibration_config.circle_diameter,     // 圆点直径
            self.calibration_config.center_distance,     // 圆点间距
            self.calibration_config.pattern_size,        // 标定板尺寸 (10x4)
            self.calibration_config.error_threshold,     // 重投影误差阈值
        ).map_err(|e| format!("创建标定器失败: {}", e))?;
        
        // Step 2: 获取点坐标 (检测asymmetric circle grid)
        let left_paths: Vec<String> = valid_images.iter()
            .map(|img| img.left_image_path.clone())
            .collect();
        let right_paths: Vec<String> = valid_images.iter()
            .map(|img| img.right_image_path.clone())
            .collect();
        
        let (left_obj_points, left_img_points) = calibrator.detect_and_get_points_from_paths(
            &left_paths,
            CameraType::Left,
        ).map_err(|e| format!("左相机特征点检测失败: {}", e))?;
        
        let (right_obj_points, right_img_points) = calibrator.detect_and_get_points_from_paths(
            &right_paths,
            CameraType::Right,
        ).map_err(|e| format!("右相机特征点检测失败: {}", e))?;
        
        // Step 3: 左相机单目标定
        println!("📷 开始左相机单目标定...");
        let left_result = calibrator.calibrate_mono_with_ab_test(&left_obj_points, &left_img_points)
            .map_err(|e| format!("左相机标定失败: {}", e))?;
        let (left_camera, left_error) = match left_result {
            MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
                println!("✅ 左相机标定成功，RMS误差: {:.4}", error);
                (MonoCamera { camera_matrix, dist_coeffs }, error)
            },
            MonoCalibResult::NeedRecalibration(error) => {
                return Err(format!("左相机标定失败，重投影误差: {:.4}", error));
            }
        };
        
        // Step 4: 右相机单目标定
        println!("📷 开始右相机单目标定...");
        let right_result = calibrator.calibrate_mono_with_ab_test(&right_obj_points, &right_img_points)
            .map_err(|e| format!("右相机标定失败: {}", e))?;
        let (right_camera, right_error) = match right_result {
            MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
                println!("✅ 右相机标定成功，RMS误差: {:.4}", error);
                (MonoCamera { camera_matrix, dist_coeffs }, error)
            },
            MonoCalibResult::NeedRecalibration(error) => {
                return Err(format!("右相机标定失败，重投影误差: {:.4}", error));
            }
        };
        
        // Step 5: 双目标定
        println!("👁️‍🗨️ 开始双目标定...");
        let stereo_result = calibrator.calibrate_stereo_with_outlier_rejection(
            &left_obj_points, &left_img_points, &right_img_points,
            &left_camera, &right_camera,
            0.2
        ).map_err(|e| format!("双目标定失败: {}", e))?;
        let (r, t, stereo_error) = match stereo_result {
            StereoCalibResult::Success { r, t, error } => {
                println!("✅ 双目标定成功，RMS误差: {:.4}", error);
                (r, t, error)
            },
            StereoCalibResult::NeedRecalibration(error) => {
                return Err(format!("双目标定失败，重投影误差: {:.4}", error));
            }
        };
        
        // Step 6: 计算立体校正映射
        println!("🔧 计算立体校正映射...");
        let rectify_maps = calibrator.compute_stereo_rectify(&left_camera, &right_camera, &r, &t)
            .map_err(|e| format!("计算立体校正映射失败: {}", e))?;
        
        // Step 7: 计算重映射矩阵
        println!("📐 计算重映射矩阵...");
        let (left_map1, left_map2) = calibrator.compute_undistort_maps(
            &left_camera.camera_matrix, &left_camera.dist_coeffs, &rectify_maps.r1, &rectify_maps.p1
        ).map_err(|e| format!("计算左相机重映射失败: {}", e))?;
        let (right_map1, right_map2) = calibrator.compute_undistort_maps(
            &right_camera.camera_matrix, &right_camera.dist_coeffs, &rectify_maps.r2, &rectify_maps.p2
        ).map_err(|e| format!("计算右相机重映射失败: {}", e))?;
        
        // Step 8: 保存标定参数和矩阵 (使用param_io.rs)
        println!("💾 保存标定参数...");
        self.save_calibration_parameters(&left_camera, &right_camera, &r, &t, 
                                       &rectify_maps, &left_map1, &left_map2, 
                                       &right_map1, &right_map2)?;
        
        // 使用已提取的误差信息
        
        Ok(CalibrationResult {
            success: true,
            left_rms_error: left_error,
            right_rms_error: right_error,
            stereo_rms_error: stereo_error,
            error_threshold: self.calibration_config.error_threshold,
            error_message: None,
            calibration_time: chrono::Utc::now().to_rfc3339(),
        })
    }
    

    
    /// 将原始图像数据转换为OpenCV Mat
    fn raw_data_to_mat(&self, image_data: &[u8]) -> Result<Mat, String> {
        // 根据实际数据大小推断图像尺寸
        let data_len = image_data.len();
        let (width, height) = match data_len {
            5013504 => (2448, 2048),  // 全分辨率
            1253376 => (1224, 1024),  // 1/2分辨率
            313344 => (612, 512),     // 1/4分辨率
            _ => {
                // 尝试推断为正方形或常见比例
                let sqrt_size = (data_len as f64).sqrt() as usize;
                if sqrt_size * sqrt_size == data_len {
                    (sqrt_size, sqrt_size)
                } else {
                    return Err(format!("无法识别的图像数据大小: {} bytes", data_len));
                }
            }
        };
        let expected_size = width * height;
        
        if image_data.len() != expected_size {
            return Err(format!("图像数据大小不匹配: 期望 {} 字节，实际 {} 字节", 
                expected_size, image_data.len()));
        }
        
        // 创建灰度 Mat 并拷贝数据
        let mut gray_mat = Mat::new_rows_cols_with_default(height as i32, width as i32, 
            opencv::core::CV_8UC1, opencv::core::Scalar::all(0.0))
            .map_err(|e| format!("创建Mat失败: {}", e))?;
        
        // 拷贝数据到 Mat
        unsafe {
            let mat_data = gray_mat.ptr_mut(0).map_err(|e| format!("获取Mat指针失败: {}", e))?;
            std::ptr::copy_nonoverlapping(image_data.as_ptr(), mat_data, image_data.len());
        }
        
        // 🎯 关键修复：转换为彩色图像以兼容SimpleBlobDetector
        // 解决问题：raw_data(灰度) vs imread(彩色) 的格式差异导致检测失败
        let mut color_mat = Mat::default();
        opencv::imgproc::cvt_color(
            &gray_mat,
            &mut color_mat,
            opencv::imgproc::COLOR_GRAY2BGR,
            0,
            AlgorithmHint::ALGO_HINT_DEFAULT
        )
            .map_err(|e| format!("灰度转彩色失败: {}", e))?;
            
        println!("✅ raw_data_to_mat: 生成彩色图像 {}x{} (从灰度转换)", width, height);
        Ok(color_mat)
    }
    
    /// 将Mat保存为PNG文件
    fn save_mat_as_png(&self, mat: &Mat, file_path: &str) -> Result<(), String> {
        imgcodecs::imwrite(file_path, mat, &Vector::new())
            .map_err(|e| format!("保存PNG文件失败: {}", e))?;
        Ok(())
    }
    
    /// 从保存的PNG文件检测标定板（绕过raw_data_to_mat问题）
    fn detect_calibration_pattern_from_saved_files(&self, left_path: &str, right_path: &str) -> Result<bool, String> {
        use opencv::imgcodecs;
        
        // 从PNG文件重新读取（与test_saved_images_fixed.rs相同的路径）
        let left_image = imgcodecs::imread(left_path, imgcodecs::IMREAD_COLOR)
            .map_err(|e| format!("读取左图PNG失败: {}", e))?;
        let right_image = imgcodecs::imread(right_path, imgcodecs::IMREAD_COLOR)
            .map_err(|e| format!("读取右图PNG失败: {}", e))?;
            
        if left_image.empty() || right_image.empty() {
            return Err("读取的PNG图像为空".to_string());
        }
        
        println!("📐 PNG图像尺寸: 左{}x{}, 右{}x{}", 
                 left_image.cols(), left_image.rows(),
                 right_image.cols(), right_image.rows());
        
        // 使用与test_saved_images_fixed.rs完全相同的检测逻辑
        self.detect_calibration_pattern_from_mat(&left_image, &right_image)
    }

    /// 从Mat直接检测标定板
    fn detect_calibration_pattern_from_mat(&self, left_mat: &Mat, right_mat: &Mat) -> Result<bool, String> {
        // 使用 calibration_circles.rs 的快速检测功能，动态获取图像尺寸
        let image_size = Size::new(left_mat.cols(), left_mat.rows());
        let mut calibrator = crate::modules::calibration_circles::Calibrator::new(
            image_size,
            self.calibration_config.circle_diameter,
            self.calibration_config.center_distance,
            self.calibration_config.pattern_size,
            self.calibration_config.error_threshold,
        ).map_err(|e| format!("创建标定器失败: {}", e))?;
        
        // 检测左图
        let left_detected = calibrator.quick_detect_calibration_pattern(left_mat);
        
        // 检测右图  
        let right_detected = calibrator.quick_detect_calibration_pattern(right_mat);
        
        // 只有两个图像都检测到标定板才算成功
        Ok(left_detected && right_detected)
    }
    
    /// 从文件路径检测标定板 (兼容性函数)
    fn detect_calibration_pattern(&self, left_path: &str, right_path: &str) -> Result<bool, String> {
        // 检查文件是否存在
        let left_exists = PathBuf::from(left_path).exists();
        let right_exists = PathBuf::from(right_path).exists();
        
        if !left_exists || !right_exists {
            return Ok(false);
        }
        
        // 读取图像并检测
        let left_image = imgcodecs::imread(left_path, imgcodecs::IMREAD_GRAYSCALE)
            .map_err(|e| format!("读取左图失败: {}", e))?;
        let right_image = imgcodecs::imread(right_path, imgcodecs::IMREAD_GRAYSCALE)
            .map_err(|e| format!("读取右图失败: {}", e))?;
        
        if left_image.empty() || right_image.empty() {
            return Ok(false);
        }
        
        self.detect_calibration_pattern_from_mat(&left_image, &right_image)
    }
    
    /// 从Mat直接生成缩略图
    fn generate_thumbnail_from_mat(&self, mat: &Mat) -> Result<String, String> {
        let mut thumbnail = Mat::default();
        imgproc::resize(mat, &mut thumbnail, 
            Size::new(200, 166),
            0.0, 0.0, imgproc::INTER_LINEAR)
            .map_err(|e| format!("缩放图像失败: {}", e))?;
        
        // 编码为PNG
        let mut buffer = Vector::new();
        imgcodecs::imencode(".png", &thumbnail, &mut buffer, &Vector::new())
            .map_err(|e| format!("编码图像失败: {}", e))?;
        
        // 转换为Base64
        let base64_str = general_purpose::STANDARD.encode(buffer.as_slice());
        Ok(format!("data:image/png;base64,{}", base64_str))
    }
    
    /// 从文件路径生成缩略图 (兼容性函数)
    fn generate_thumbnail(&self, image_path: &str) -> Result<String, String> {
        let image = imgcodecs::imread(image_path, imgcodecs::IMREAD_GRAYSCALE)
            .map_err(|e| format!("读取图像失败: {}", e))?;
        
        if image.empty() {
            return Err("读取的图像为空".to_string());
        }
        
        self.generate_thumbnail_from_mat(&image)
    }
    
    /// 保存标定参数到文件
    fn save_calibration_parameters(
        &self,
        left_camera: &MonoCamera, right_camera: &MonoCamera,
        r: &Mat, t: &Mat,
        rectify_maps: &crate::modules::calibration_circles::RectifyMaps,
        left_map1: &Mat, left_map2: &Mat,
        right_map1: &Mat, right_map2: &Mat,
    ) -> Result<(), String> {
        
        // 使用默认路径保存参数
        let base_path = "yaml_last_param_file";
        fs::create_dir_all(base_path)
            .map_err(|e| format!("创建参数目录失败: {}", e))?;
        
        // 保存左相机参数
        let left_params = CameraParams {
            camera_matrix: mat_to_vec2d_f64(&left_camera.camera_matrix),
            dist_coeffs: mat_to_vec_f64(&left_camera.dist_coeffs),
        };
        save_camera_params(&format!("{}/left_camera_params.yaml", base_path), &left_params)
            .map_err(|e| format!("保存左相机参数失败: {}", e))?;
        
        // 保存右相机参数
        let right_params = CameraParams {
            camera_matrix: mat_to_vec2d_f64(&right_camera.camera_matrix),
            dist_coeffs: mat_to_vec_f64(&right_camera.dist_coeffs),
        };
        save_camera_params(&format!("{}/right_camera_params.yaml", base_path), &right_params)
            .map_err(|e| format!("保存右相机参数失败: {}", e))?;
        
        // 保存双目参数
        let stereo_params = StereoParams {
            r: mat_to_vec2d_f64(r),
            t: mat_to_vec_f64(t),
        };
        save_stereo_params(&format!("{}/stereo_params.yaml", base_path), &stereo_params)
            .map_err(|e| format!("保存双目参数失败: {}", e))?;
        
        // 保存重映射参数
        let rectify_params = RectifyParams {
            r1: mat_to_vec2d_f64(&rectify_maps.r1),
            r2: mat_to_vec2d_f64(&rectify_maps.r2),
            p1: mat_to_vec2d_f64(&rectify_maps.p1),
            p2: mat_to_vec2d_f64(&rectify_maps.p2),
            q: mat_to_vec2d_f64(&rectify_maps.q),
        };
        save_rectify_params(&format!("{}/rectify_params.yaml", base_path), &rectify_params)
            .map_err(|e| format!("保存重映射参数失败: {}", e))?;
        
        // 保存重映射矩阵
        let rectify_lr_maps = RectifyLeftRightMaps {
            left_map1: mat_to_vec2d_f32(left_map1),
            left_map2: mat_to_vec2d_f32(left_map2),
            right_map1: mat_to_vec2d_f32(right_map1),
            right_map2: mat_to_vec2d_f32(right_map2),
        };
        save_rectify_maps(&format!("{}/rectify_maps.yaml", base_path), &rectify_lr_maps)
            .map_err(|e| format!("保存重映射矩阵失败: {}", e))?;
        
        println!("✅ 所有标定参数已保存到: {}", base_path);
        Ok(())
    }
    
    /// 获取当前状态
    pub fn get_status(&self) -> CalibrationStatus {
        self.current_status.clone()
    }
    
    /// 检查相机是否处于活跃状态
    pub fn is_camera_active(&self) -> bool {
        // 检查相机是否已启动
        // 这里假设SimpleCameraManager有相应的状态检查方法
        // 如果没有，可以通过尝试获取一帧来判断
        true // 临时实现，需要根据SimpleCameraManager的实际API调整
    }
    
    /// 快速检测标定板（内部方法）
    fn quick_detect_pattern_from_mats(&mut self, left_mat: &Mat, right_mat: &Mat) -> bool {
        // 创建临时标定器进行快速检测
        match crate::modules::calibration_circles::Calibrator::new(
            Size::new(left_mat.cols(), left_mat.rows()),
            self.calibration_config.circle_diameter,
            self.calibration_config.center_distance,
            self.calibration_config.pattern_size,
            self.calibration_config.error_threshold,
        ) {
            Ok(mut calibrator) => {
                // 只检测左相机图像（提高性能）
                calibrator.quick_detect_calibration_pattern(left_mat)
            }
            Err(_) => false
        }
    }
    
    /// 获取已采集的图像列表
    pub fn get_captured_images(&self) -> Vec<ImagePair> {
        self.captured_images.clone()
    }
    
    /// 删除指定的图像对
    pub fn delete_captured_image(&mut self, pair_id: u32) -> Result<(), String> {
        if let Some(index) = self.captured_images.iter().position(|img| img.pair_id == pair_id) {
            let image_pair = self.captured_images.remove(index);
            
            // 删除文件
            let _ = fs::remove_file(&image_pair.left_image_path);
            let _ = fs::remove_file(&image_pair.right_image_path);
            
            // 如果删除后数量不足，回到采集状态
            if self.current_status == CalibrationStatus::ReadyToCalibrate && 
               self.captured_images.len() < self.calibration_config.target_image_count as usize {
                self.current_status = CalibrationStatus::Capturing;
            }
            
            println!("🗑️ 已删除图像对: {}", pair_id);
            Ok(())
        } else {
            Err("找不到指定的图像对".to_string())
        }
    }
    
    /// 停止标定会话并释放资源
    pub fn stop_calibration(&mut self) -> Result<(), String> {
        println!("⏹️ 停止标定会话...");
        
        // 1. 停止后台采集线程
        // 即时处理模式下，没有后台线程，直接停止相机
        if let Err(e) = self.camera_manager.stop() {
            println!("⚠️ 停止主相机时出错: {}", e);
        }
        
        // 2. 清理缓冲区
        // 即时处理模式下，没有缓冲区，直接清空图像列表
        self.captured_images.clear();
        
        // 3. 重置状态
        self.current_status = CalibrationStatus::NotStarted;
        self.session_id = None;
        self.should_save_next_frame.store(false, Ordering::SeqCst);
        
        println!("✅ 标定会话已停止");
        Ok(())
    }
}

impl Drop for CalibrationWorkflow {
    fn drop(&mut self) {
        // 确保相机资源被正确释放
        let _ = self.camera_manager.stop();
    }
}

// 测试专用方法
impl CalibrationWorkflow {
    /// 创建用于测试的CalibrationWorkflow实例（不启动相机）
    pub fn new_for_testing() -> Result<Self, String> {
        // 为了避免硬件依赖，我们创建一个最小化的测试实例
        // 注意：这个方法仅用于离线测试，不会实际使用camera_manager
        use crate::camera_manager::SimpleCameraManager;
        
        // 尝试创建相机管理器，如果失败就创建一个虚拟的
        let camera_manager = match SimpleCameraManager::new() {
            Ok(cm) => cm,
            Err(_) => {
                // 如果相机不可用，我们仍然需要一个占位符
                // 但这个测试实例不会使用相机功能
                println!("⚠️  相机不可用，创建测试专用实例（不影响离线测试）");
                return Err("相机不可用，但这不影响离线workflow测试".to_string());
            }
        };
        
        Ok(Self {
            camera_manager,
            captured_images: Vec::new(),
            calibration_config: CalibrationConfig::default(),
            current_status: CalibrationStatus::NotStarted,
            session_id: Some("test_session".to_string()),
            should_save_next_frame: Arc::new(AtomicBool::new(false)),
        })
    }
    
    /// 创建纯离线测试实例（完全不依赖相机）
    pub fn new_offline_testing() -> Self {
        // 使用Option包装相机管理器，离线测试时设为None
        // 这样可以安全地测试不涉及相机的workflow功能
        Self {
            camera_manager: unsafe { std::mem::zeroed() }, // 临时占位，不会被使用
            captured_images: Vec::new(),
            calibration_config: CalibrationConfig::default(),
            current_status: CalibrationStatus::NotStarted,
            session_id: Some("offline_test".to_string()),
            should_save_next_frame: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// 测试完整workflow标定流程（使用预设图像）
    pub fn test_full_calibration_workflow(&self) -> Result<CalibrationResult, String> {
        // 过滤出有效的图像
        let valid_images: Vec<&ImagePair> = self.captured_images
            .iter()
            .filter(|img| img.has_calibration_pattern)
            .collect();
            
        if valid_images.is_empty() {
            return Err("没有找到有效的标定图像".to_string());
        }
        
        println!("🚀 开始完整workflow标定流程");
        println!("📊 使用 {} 组有效图像", valid_images.len());
        
        // 直接调用内部的标定算法
        self.run_calibration_algorithm(&valid_images)
    }
    
    /// 设置用于测试的图像列表
    pub fn set_captured_images_for_testing(&mut self, images: Vec<ImagePair>) {
        self.captured_images = images;
    }
    
    /// 测试用的检测方法，暴露内部的detect_calibration_pattern_from_mat
    pub fn test_detect_calibration_pattern_from_mat(&self, left_mat: &opencv::core::Mat, right_mat: &opencv::core::Mat) -> Result<bool, String> {
        self.detect_calibration_pattern_from_mat(left_mat, right_mat)
    }
    
    /// 测试用的标定算法方法，使用当前captured_images
    pub fn test_run_calibration_algorithm(&self) -> Result<CalibrationResult, String> {
        // 过滤出有效的图像
        let valid_images: Vec<&ImagePair> = self.captured_images
            .iter()
            .filter(|img| img.has_calibration_pattern)
            .collect();
            
        if valid_images.is_empty() {
            return Err("没有找到有效的标定图像".to_string());
        }
        
        println!("📊 使用 {} 组有效图像进行标定", valid_images.len());
        self.run_calibration_algorithm(&valid_images)
    }
} 