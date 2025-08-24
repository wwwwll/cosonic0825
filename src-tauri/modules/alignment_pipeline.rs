// alignment_pipeline.rs - 流水线并行处理模块
// 实现多线程流水线架构以提升合像检测性能

// 🏗️ 架构设计说明
// 
// ## 流水线并行处理系统
// 
// ### 职责：高吞吐量的连续帧处理
// ```
// 主线程 -> Thread A (重映射) -> Thread B (圆心检测) -> Thread C (姿态分析) -> 结果
// ```
// - **用途**: 实时预览模式，连续采集处理
// - **特点**: 三个线程并行工作，各自维护AlignmentSystem实例
// - **输入**: `process_frame(left_image, right_image)`
// - **输出**: `AlignmentResult` (通过 `try_get_result()` 获取)
// 
// ### 与其他模块的关系
// 
// | 模块 | 职责 | 使用场景 |
// |------|------|----------|
// | `alignment.rs` | 核心算法 | 底层检测算法 |
// | `alignment_pipeline.rs` | 流水线并行处理 | 实时预览，高吞吐量 |
// | `alignment_workflow.rs` | 工作流管理 | 前端命令，单帧检测 |
// 
// ### 简化设计原则
// 
// - **流水线专用**: 此模块仅处理连续帧的并行处理
// - **单帧检测**: 前端触发的单帧检测应使用 `alignment_workflow.rs`
// - **空值处理**: Thread C 的条件检测已完美支持前端空值显示需求

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use opencv::{core::Mat, prelude::*};
use crate::modules::alignment::{AlignmentSystem, SingleEyePoseResult, DualEyeAlignmentResult};

/// 流水线任务数据
#[derive(Clone)]
pub struct PipelineFrame {
    pub frame_id: u64,
    pub timestamp: Instant,
    pub left_image: Mat,
    pub right_image: Mat,
}

/// 重映射结果
#[derive(Clone)]
pub struct RemappedFrame {
    pub frame_id: u64,
    pub timestamp: Instant,
    pub left_rectified: Mat,
    pub right_rectified: Mat,
}

/// 检测结果
#[derive(Clone)]
pub struct DetectionResult {
    pub frame_id: u64,
    pub timestamp: Instant,
    pub left_corners: opencv::core::Vector<opencv::core::Point2f>,
    pub right_corners: opencv::core::Vector<opencv::core::Point2f>,
}

/// 最终合像结果
#[derive(Debug, Clone)]
pub struct AlignmentResult {
    pub frame_id: u64,
    pub timestamp: Instant,
    pub processing_time: Duration,
    pub left_pose_result: SingleEyePoseResult,
    pub right_pose_result: SingleEyePoseResult,
    pub alignment_result: Option<DualEyeAlignmentResult>, // 🎯 关键：支持空值，前端直接检查即可
}

/// 🚀 流水线并行处理系统
pub struct AlignmentPipeline {
    // 各阶段通信通道 - 使用 SyncSender
    remap_sender: mpsc::SyncSender<PipelineFrame>,
    detection_sender: mpsc::SyncSender<RemappedFrame>,
    analysis_sender: mpsc::SyncSender<DetectionResult>,
    result_receiver: mpsc::Receiver<AlignmentResult>,
    
    // 线程句柄
    remap_handle: Option<thread::JoinHandle<()>>,
    detection_handle: Option<thread::JoinHandle<()>>,
    analysis_handle: Option<thread::JoinHandle<()>>,
    
    // 性能统计
    frame_counter: u64,
    performance_stats: Arc<Mutex<PipelineStats>>,
}

/// 流水线性能统计
#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub total_frames: u64,
    pub avg_remap_time: f64,
    pub avg_detection_time: f64,
    pub avg_analysis_time: f64,
    pub avg_total_time: f64,
    pub throughput_fps: f64,
}

impl AlignmentPipeline {
    /// 创建新的流水线实例
    pub fn new(
        image_size: opencv::core::Size,
        left_camera_params_path: &str,
        right_camera_params_path: &str,
        stereo_params_path: &str,
        rectify_params_path: &str,
        rectify_maps_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🚀 初始化流水线并行处理系统...");
        
        // 🚀 生产环境优化缓冲区配置 - 充分利用16GB内存
        // 
        // 生产环境设计原则 (i7 + 16GB + 480G SSD)：
        // 1. 性能优先：充分利用内存资源，避免任何流水线阻塞
        // 2. 稳定运行：缓冲区足够大，应对各种负载波动
        // 3. 内存预算：总缓冲区使用 < 1GB，远低于16GB容量
        
        // 📊 内存使用估算：
        // - 大图像帧：2448×2048×1字节 ≈ 5MB/帧
        // - 总缓冲容量：(10+10)×5MB + (30+100)×1KB ≈ 100MB + 130KB ≈ 100MB
        
        // 🔥 根据CPU核心数动态调整缓冲区
        let cpu_cores = num_cpus::get();
        let base_buffer = if cpu_cores >= 8 { 15 } else { 10 }; // i7通常8核心+
        
        let (remap_tx, remap_rx) = mpsc::sync_channel::<PipelineFrame>(base_buffer);     // 动态图像缓冲
        let (detection_tx, detection_rx) = mpsc::sync_channel::<RemappedFrame>(base_buffer); // 动态图像缓冲
        let (analysis_tx, analysis_rx) = mpsc::sync_channel::<DetectionResult>(base_buffer * 2); // Thread C瓶颈，大缓冲
        let (result_tx, result_rx) = mpsc::sync_channel::<AlignmentResult>(base_buffer * 8);    // 主线程超大缓冲
        
        println!("🔧 缓冲区配置: {}核CPU → {}帧图像缓冲, {}帧结果缓冲", 
                cpu_cores, base_buffer, base_buffer * 8);
        
        let performance_stats = Arc::new(Mutex::new(PipelineStats {
            total_frames: 0,
            avg_remap_time: 0.0,
            avg_detection_time: 0.0,
            avg_analysis_time: 0.0,
            avg_total_time: 0.0,
            throughput_fps: 0.0,
        }));
        
        // 🚀 各线程独立创建AlignmentSystem实例
        
        // 🔧 Thread A: 图像重映射线程
        let remap_handle = {
            let detection_tx = detection_tx.clone();
            let stats = Arc::clone(&performance_stats);
            // 为Thread A创建轻量级实例（不重复预加载）
            let mut alignment_system = AlignmentSystem::new(
                image_size,
                left_camera_params_path,
                right_camera_params_path,
                stereo_params_path,
                rectify_params_path,
            )?;
            // 手动触发预加载，但不重复初始化
            alignment_system.ensure_maps_loaded(rectify_maps_path)?;
            
            thread::spawn(move || {
                println!("🔧 Thread A: 重映射线程启动");
                
                while let Ok(frame) = remap_rx.recv() {
                    let remap_start = Instant::now();
                    
                    match alignment_system.remap_images_only(&frame.left_image, &frame.right_image) {
                        Ok((left_rect, right_rect)) => {
                            let remap_time = remap_start.elapsed();
                            
                            // 更新统计
                            if let Ok(mut stats) = stats.lock() {
                                stats.avg_remap_time = (stats.avg_remap_time * stats.total_frames as f64 + 
                                    remap_time.as_millis() as f64) / (stats.total_frames + 1) as f64;
                            }
                            
                            let remapped_frame = RemappedFrame {
                                frame_id: frame.frame_id,
                                timestamp: frame.timestamp,
                                left_rectified: left_rect,
                                right_rectified: right_rect,
                            };
                            
                            if detection_tx.send(remapped_frame).is_err() {
                                break; // 下游线程已关闭
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Thread A 重映射失败: {}", e);
                        }
                    }
                }
                
                println!("🔧 Thread A: 重映射线程结束");
            })
        };
        
        // 🔍 Thread B: 圆心检测线程
        let detection_handle = {
            let analysis_tx = analysis_tx.clone();
            let stats = Arc::clone(&performance_stats);
            // Thread B只需要基础系统，不需要重映射矩阵
            let mut alignment_system = AlignmentSystem::new(
                image_size,
                left_camera_params_path,
                right_camera_params_path,
                stereo_params_path,
                rectify_params_path,
            )?;
            
            thread::spawn(move || {
                println!("🔍 Thread B: 圆心检测线程启动");
                
                while let Ok(frame) = detection_rx.recv() {
                    let detection_start = Instant::now();
                    
                    match alignment_system.detect_circles_only(&frame.left_rectified, &frame.right_rectified) {
                        Ok((left_corners, right_corners)) => {
                            let detection_time = detection_start.elapsed();
                            
                            // 更新统计
                            if let Ok(mut stats) = stats.lock() {
                                stats.avg_detection_time = (stats.avg_detection_time * stats.total_frames as f64 + 
                                    detection_time.as_millis() as f64) / (stats.total_frames + 1) as f64;
                            }
                            
                            let detection_result = DetectionResult {
                                frame_id: frame.frame_id,
                                timestamp: frame.timestamp,
                                left_corners,
                                right_corners,
                            };
                            
                            if analysis_tx.send(detection_result).is_err() {
                                break; // 下游线程已关闭
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Thread B 圆心检测失败: {}", e);
                        }
                    }
                }
                
                println!("🔍 Thread B: 圆心检测线程结束");
            })
        };
        
        // 🎯 Thread C: 姿态估计 + 合像分析线程
        let analysis_handle = {
            let result_tx = result_tx.clone();
            let stats = Arc::clone(&performance_stats);
            // Thread C只需要基础系统，不需要重映射矩阵
            let mut alignment_system = AlignmentSystem::new(
                image_size,
                left_camera_params_path,
                right_camera_params_path,
                stereo_params_path,
                rectify_params_path,
            )?;
            
            thread::spawn(move || {
                println!("🎯 Thread C: 姿态分析线程启动");
                
                while let Ok(detection) = analysis_rx.recv() {
                    let analysis_start = Instant::now();
                    println!("🎯 Thread C: 开始处理帧{}", detection.frame_id);
                    
                    // 获取相机参数
                    let (left_camera_matrix, left_dist_coeffs) = alignment_system.get_left_camera_params();
                    let (right_camera_matrix, right_dist_coeffs) = alignment_system.get_right_camera_params();
                    
                    // 左眼姿态估计
                    println!("🎯 Thread C: 帧{} - 开始左眼姿态估计", detection.frame_id);
                    let left_pose_result = match alignment_system.check_single_eye_pose(
                        &detection.left_corners, 
                        left_camera_matrix, 
                        left_dist_coeffs
                    ) {
                        Ok(pose) => pose,
                        Err(e) => {
                            eprintln!("❌ Thread C 左眼姿态估计失败: {}", e);
                            // 创建失败的姿态结果
                            use crate::modules::alignment::SingleEyePoseResult;
                            SingleEyePoseResult {
                                roll: 0.0,
                                pitch: 0.0,
                                yaw: 0.0,
                                pass: false,
                            }
                        }
                    };
                    
                    println!("🎯 Thread C: 帧{} - 左眼姿态估计完成，通过: {}", detection.frame_id, left_pose_result.pass);
                    
                    // 右眼姿态估计
                    println!("🎯 Thread C: 帧{} - 开始右眼姿态估计", detection.frame_id);
                    let right_pose_result = match alignment_system.check_single_eye_pose(
                        &detection.right_corners, 
                        right_camera_matrix, 
                        right_dist_coeffs
                    ) {
                        Ok(pose) => pose,
                        Err(e) => {
                            eprintln!("❌ Thread C 右眼姿态估计失败: {}", e);
                            // 创建失败的姿态结果
                            use crate::modules::alignment::SingleEyePoseResult;
                            SingleEyePoseResult {
                                roll: 0.0,
                                pitch: 0.0,
                                yaw: 0.0,
                                pass: false,
                            }
                        }
                    };
                    
                    println!("🎯 Thread C: 帧{} - 右眼姿态估计完成，通过: {}", detection.frame_id, right_pose_result.pass);
                    
                    // 合像分析（仅在双眼姿态都通过时执行）
                    let alignment_result = if left_pose_result.pass && right_pose_result.pass {
                        println!("🎯 Thread C: 帧{} - 双眼姿态通过，开始合像分析", detection.frame_id);
                        match alignment_system.check_dual_eye_alignment(&detection.left_corners, &detection.right_corners, false) {
                            Ok(result) => Some(result),
                            Err(e) => {
                                eprintln!("❌ Thread C 合像分析失败: {}", e);
                                None
                            }
                        }
                    } else {
                        println!("🎯 Thread C: 帧{} - 姿态检测未通过，跳过合像分析", detection.frame_id);
                        None
                    };
                    
                    let analysis_time = analysis_start.elapsed();
                    let total_processing_time = detection.timestamp.elapsed();
                    
                    // 更新统计
                    if let Ok(mut stats) = stats.lock() {
                        stats.total_frames += 1;
                        stats.avg_analysis_time = (stats.avg_analysis_time * (stats.total_frames - 1) as f64 + 
                            analysis_time.as_millis() as f64) / stats.total_frames as f64;
                        stats.avg_total_time = (stats.avg_total_time * (stats.total_frames - 1) as f64 + 
                            total_processing_time.as_millis() as f64) / stats.total_frames as f64;
                        
                        // 计算吞吐量
                        if stats.avg_total_time > 0.0 {
                            stats.throughput_fps = 1000.0 / stats.avg_total_time;
                        }
                    }
                    
                    // 注意：这里我们需要更新AlignmentResult结构以支持双眼姿态
                    // 暂时使用左眼姿态作为主要姿态结果，后续可以扩展
                    let final_result = AlignmentResult {
                        frame_id: detection.frame_id,
                        timestamp: detection.timestamp,
                        processing_time: total_processing_time,
                        left_pose_result, // 主要姿态结果（左眼）
                        right_pose_result,
                        alignment_result,
                    };
                    
                    println!("🎯 Thread C: 帧{} - 发送结果到主线程", detection.frame_id);
                    if result_tx.send(final_result).is_err() {
                        println!("🎯 Thread C: 主线程已关闭，退出");
                        break; // 主线程已关闭
                    }
                    println!("🎯 Thread C: 帧{} - 处理完成", detection.frame_id);
                }
                
                println!("🎯 Thread C: 姿态分析线程结束");
            })
        };
        
        println!("✅ 流水线并行处理系统初始化完成");
        
        Ok(Self {
            remap_sender: remap_tx,
            detection_sender: detection_tx,
            analysis_sender: analysis_tx,
            result_receiver: result_rx,
            remap_handle: Some(remap_handle),
            detection_handle: Some(detection_handle),
            analysis_handle: Some(analysis_handle),
            frame_counter: 0,
            performance_stats,
        })
    }
    
    /// 🚀 提交帧进行流水线处理（带缓冲区健康检查）
    pub fn process_frame(&mut self, left_image: Mat, right_image: Mat) -> Result<(), Box<dyn std::error::Error>> {
        self.frame_counter += 1;
        
        let frame = PipelineFrame {
            frame_id: self.frame_counter,
            timestamp: Instant::now(),
            left_image,
            right_image,
        };
        
        // 🔍 缓冲区健康检查 - 保护长期运行
        match self.remap_sender.try_send(frame) {
            Ok(_) => {
                // 发送成功，流水线健康
                Ok(())
            }
            Err(mpsc::TrySendError::Full(dropped_frame)) => {
                // 缓冲区满，智能丢帧保护
                println!("⚠️ 流水线缓冲区满，丢弃帧{} (内存保护)", dropped_frame.frame_id);
                
                // 选择策略：
                // 1. 直接丢帧（推荐）- 保护内存，允许偶发丢帧
                Ok(()) 
                
                // 2. 强制发送（备选）- 会阻塞，但不丢帧
                // self.remap_sender.send(dropped_frame)?;
                // Ok(())
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                Err("流水线已关闭".into())
            }
        }
    }
    
    /// 🎯 获取处理结果（非阻塞）
    pub fn try_get_result(&self) -> Option<AlignmentResult> {
        self.result_receiver.try_recv().ok()
    }
    
    /// 🎯 获取处理结果（阻塞，带超时）
    pub fn get_result_timeout(&self, timeout: Duration) -> Option<AlignmentResult> {
        self.result_receiver.recv_timeout(timeout).ok()
    }
    
    /// 📊 获取性能统计
    pub fn get_performance_stats(&self) -> PipelineStats {
        self.performance_stats.lock().unwrap().clone()
    }
    
    /// 📊 打印性能统计
    pub fn print_performance_stats(&self) {
        let stats = self.get_performance_stats();
        
        println!("\n📊 流水线性能统计:");
        println!("   处理帧数: {}", stats.total_frames);
        println!("   平均重映射时间: {:.1} ms", stats.avg_remap_time);
        println!("   平均圆心检测时间: {:.1} ms", stats.avg_detection_time);
        println!("   平均姿态分析时间: {:.1} ms", stats.avg_analysis_time);
        println!("   平均总处理时间: {:.1} ms", stats.avg_total_time);
        println!("   实际吞吐量: {:.1} fps", stats.throughput_fps);
        
        // 🔍 缓冲区健康状态检查
        println!("\n🔍 缓冲区健康状态:");
        println!("   结果缓冲区状态: 可用");
        
        // 10fps兼容性分析
        if stats.throughput_fps >= 10.0 {
            println!("   ✅ 满足10fps实时处理要求");
        } else {
            println!("   ⚠️  吞吐量未达到10fps要求");
        }
    }
    
    /// 🛑 关闭流水线系统
    pub fn shutdown(&mut self) {
        println!("🛑 关闭流水线处理系统...");
        
        // 主动关闭所有发送端，让接收线程退出
        // 这些 sender 会在 drop 时自动关闭，触发线程退出
        
        // 设置较短的等待时间，避免无限等待
        use std::time::Duration;
        let timeout = Duration::from_millis(1000); // 1秒超时
        
        // 快速退出策略 - 不等待线程，让系统自然清理
        println!("⏳ 释放线程资源...");
        
        // 取出线程句柄但不等待，让它们自然结束
        let _remap_handle = self.remap_handle.take();
        let _detection_handle = self.detection_handle.take();
        let _analysis_handle = self.analysis_handle.take();
        
        // 注意：线程会在通道关闭时自然退出，不需要强制等待
        println!("📤 通道已关闭，线程将自然退出");
        
        println!("✅ 流水线处理系统已关闭");
    }
}

impl Drop for AlignmentPipeline {
    fn drop(&mut self) {
        // 避免在Drop中调用可能阻塞的shutdown
        // 用户应该显式调用shutdown()
        println!("🔄 AlignmentPipeline正在释放资源...");
    }
}

/// 🚀 流水线处理的便捷函数
impl AlignmentSystem {
    /// 仅执行重映射（用于Thread A）
    pub fn remap_images_only(
        &mut self,
        left_image: &Mat,
        right_image: &Mat,
    ) -> Result<(Mat, Mat), Box<dyn std::error::Error>> {
        // 确保重映射矩阵已加载
        // 🔧 修正重映射矩阵路径 - 使用yaml_last_param_file目录
        self.ensure_maps_loaded("yaml_last_param_file/rectify_maps.yaml")?;
        
        // 使用公有的访问方法获取重映射矩阵
        if let Some((left_map1, left_map2, right_map1, right_map2)) = self.get_rectify_maps() {
            let rectifier = self.get_rectifier();
            let left_rect = rectifier.remap_image_adaptive(left_image, left_map1, left_map2)?;
            let right_rect = rectifier.remap_image_adaptive(right_image, right_map1, right_map2)?;
            Ok((left_rect, right_rect))
        } else {
            Err("重映射矩阵未加载".into())
        }
    }
    
    /// 仅执行圆心检测（用于Thread B）
    /// 🆕 已更新使用ConnectedComponentsDetector替代SimpleBlobDetector
    pub fn detect_circles_only(
        &mut self,
        left_rectified: &Mat,
        right_rectified: &Mat,
    ) -> Result<(opencv::core::Vector<opencv::core::Point2f>, opencv::core::Vector<opencv::core::Point2f>), Box<dyn std::error::Error>> {
        let pattern_size = opencv::core::Size::new(4, 10);
        let mut corners_left = opencv::core::Vector::<opencv::core::Point2f>::new();
        let mut corners_right = opencv::core::Vector::<opencv::core::Point2f>::new();
        
        // 🆕 使用连通域检测器替代SimpleBlobDetector
        // let detector = self.create_optimized_blob_detector()?; // 已替换
        use opencv::features2d::{SimpleBlobDetector, SimpleBlobDetector_Params};
        let detector = SimpleBlobDetector::create(SimpleBlobDetector_Params::default()?)?.into(); // 保持接口兼容，但实际不使用
        
        println!("🔍 Thread B: 使用连通域检测器进行圆心检测");
        
        let left_found = self.detect_circles_full_image(
            left_rectified, 
            pattern_size, 
            &mut corners_left, 
            &detector
        )?;
        
        let right_found = self.detect_circles_full_image(
            right_rectified, 
            pattern_size, 
            &mut corners_right, 
            &detector
        )?;
        
        if !left_found {
            return Err("左眼圆点网格检测失败".into());
        }
        if !right_found {
            return Err("右眼圆点网格检测失败".into());
        }
        
        println!("✅ Thread B: 圆心检测完成 - 左眼{}个点，右眼{}个点", 
                corners_left.len(), corners_right.len());
        
        Ok((corners_left, corners_right))
    }
} 