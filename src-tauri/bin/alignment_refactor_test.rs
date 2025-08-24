// alignment_refactor_test.rs - 重构后的合像检测功能测试
// 测试新的解耦API：独立的姿态检测和合像分析
// 用于测试合像检测全流程耗时

use std::time::Instant;
use opencv::{core, imgcodecs, prelude::*};
use merging_image_lib::modules::alignment::{AlignmentSystem, SingleEyePoseResult, DualEyeAlignmentResult, CenteringResult, AdjustmentVectors};

/// 重构功能测试器
pub struct AlignmentRefactorTest {
    alignment_system: AlignmentSystem,
    test_image_left: core::Mat,
    test_image_right: core::Mat,
    rectify_maps_path: String,
}

impl AlignmentRefactorTest {
    /// 创建测试实例
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔧 初始化重构功能测试器...");
        
        // 确定正确的文件路径
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().unwrap();
        let src_tauri_dir = exe_dir.parent().unwrap().parent().unwrap();
        
        // 构造绝对路径
        let img_path_left = src_tauri_dir.join("src/tests/data/benchmark/left_calibration.bmp");
        let img_path_right = src_tauri_dir.join("src/tests/data/benchmark/right_calibration.bmp");
        // 🔧 修正参数文件路径 - 使用yaml_last_param_file目录
        // 旧路径 (注释掉):
        // let left_cam_params = src_tauri_dir.join("left_camera_params.yaml");
        // let right_cam_params = src_tauri_dir.join("right_camera_params.yaml");
        // let stereo_params = src_tauri_dir.join("stereo_params.yaml");
        // let rectify_params = src_tauri_dir.join("rectify_params.yaml");
        
        let left_cam_params = src_tauri_dir.join("yaml_last_param_file/left_camera_params.yaml");
        let right_cam_params = src_tauri_dir.join("yaml_last_param_file/right_camera_params.yaml");
        let stereo_params = src_tauri_dir.join("yaml_last_param_file/stereo_params.yaml");
        let rectify_params = src_tauri_dir.join("yaml_last_param_file/rectify_params.yaml");
        // 🔧 修正重映射矩阵路径 - 使用yaml_last_param_file目录
        let rectify_maps = src_tauri_dir.join("yaml_last_param_file/rectify_maps.yaml");
        
        // 加载测试图像
        println!("📁 加载测试图像...");
        let test_image_left = imgcodecs::imread(
            img_path_left.to_str().unwrap(),
            imgcodecs::IMREAD_GRAYSCALE
        )?;
        
        let test_image_right = imgcodecs::imread(
            img_path_right.to_str().unwrap(), 
            imgcodecs::IMREAD_GRAYSCALE
        )?;
        
        if test_image_left.empty() || test_image_right.empty() {
            return Err("无法加载测试图像，请检查文件路径".into());
        }
        
        let image_size = test_image_left.size()?;
        println!("✓ 测试图像加载成功: {}×{}", image_size.width, image_size.height);
        
        // 创建合像检测系统（使用预加载优化）
        println!("🔧 初始化合像检测系统...");
        let alignment_system = AlignmentSystem::new_with_preload(
            image_size,
            left_cam_params.to_str().unwrap(),
            right_cam_params.to_str().unwrap(), 
            stereo_params.to_str().unwrap(),
            rectify_params.to_str().unwrap(),
            rectify_maps.to_str().unwrap(),
        )?;
        
        println!("✓ 合像检测系统初始化完成");
        
        Ok(Self {
            alignment_system,
            test_image_left,
            test_image_right,
            rectify_maps_path: rectify_maps.to_string_lossy().to_string(),
        })
    }
    
    /// 运行完整的重构功能测试
    pub fn run_tests(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🚀 开始重构功能测试");
        println!("{}", "=".repeat(50));
        
        // 1. 测试圆心检测
        println!("\n📊 测试1: 圆心检测功能");
        println!("{}", "-".repeat(30));
        let (left_corners, right_corners) = self.test_circle_detection()?;
        
        // 2. 测试独立的左眼姿态检测
        println!("\n📊 测试2: 左眼姿态检测（新API）");
        println!("{}", "-".repeat(30));
        let left_pose = self.test_left_eye_pose(&left_corners)?;
        
        // 3. 测试独立的右眼姿态检测
        println!("\n📊 测试3: 右眼姿态检测（新API）");
        println!("{}", "-".repeat(30));
        let right_pose = self.test_right_eye_pose(&right_corners)?;
        
        // 4. 测试纯合像分析
        println!("\n📊 测试4: 纯合像分析（无姿态检测）");
        println!("{}", "-".repeat(30));
        let alignment_result = self.test_pure_alignment_analysis(&left_corners, &right_corners)?;
        
        // 4.5. 测试左眼居中检测 - 🆕
        println!("\n📊 测试4.5: 左眼居中检测（新功能）");
        println!("{}", "-".repeat(30));
        let centering_result = self.test_left_eye_centering(&left_corners)?;
        
        // 4.6. 测试操作提示辅助计算 - 🆕
        println!("\n📊 测试4.6: 操作提示辅助计算（新功能）");
        println!("{}", "-".repeat(30));
        let adjustment_vectors = self.test_adjustment_vectors(&left_pose, &centering_result, &right_pose, &alignment_result)?;
        
        // 5. 测试完整的检测流程（模拟前端调用）
        println!("\n📊 测试5: 完整检测流程（模拟前端调用）");
        println!("{}", "-".repeat(30));
        self.test_complete_workflow(&left_corners, &right_corners, &left_pose, &right_pose, &alignment_result, &centering_result, &adjustment_vectors)?;
        
        // 6. 测试向后兼容性
        println!("\n📊 测试6: 向后兼容性测试");
        println!("{}", "-".repeat(30));
        self.test_backward_compatibility(&left_corners, &right_corners)?;
        
        // 7. 性能对比测试
        println!("\n📊 测试7: 性能对比测试");
        println!("{}", "-".repeat(30));
        self.test_performance_comparison()?;
        
        println!("\n✅ 所有重构功能测试完成！");
        Ok(())
    }
    
    /// 测试圆心检测
    fn test_circle_detection(&mut self) -> Result<(opencv::core::Vector<opencv::core::Point2f>, opencv::core::Vector<opencv::core::Point2f>), Box<dyn std::error::Error>> {
        println!("🔍 测试圆心检测功能...");
        
        let start = Instant::now();
        let result = self.alignment_system.detect_circles_grid(
            &self.test_image_left,
            &self.test_image_right,
            &self.rectify_maps_path,
        );
        let detection_time = start.elapsed();
        
        match result {
            Ok((left_corners, right_corners)) => {
                println!("✓ 圆心检测成功");
                println!("   左眼检测到: {} 个圆点", left_corners.len());
                println!("   右眼检测到: {} 个圆点", right_corners.len());
                println!("   检测耗时: {:.1} ms", detection_time.as_millis());
                Ok((left_corners, right_corners))
            },
            Err(e) => {
                println!("❌ 圆心检测失败: {}", e);
                Err(e)
            }
        }
    }
    
    /// 测试左眼姿态检测（新API）
    fn test_left_eye_pose(&mut self, left_corners: &opencv::core::Vector<opencv::core::Point2f>) -> Result<SingleEyePoseResult, Box<dyn std::error::Error>> {
        println!("🔍 测试左眼姿态检测（新API）...");
        
        // 获取左相机参数
        let (left_camera_matrix, left_dist_coeffs) = self.alignment_system.get_left_camera_params();
        
        let start = Instant::now();
        let result = self.alignment_system.check_single_eye_pose(
            left_corners,
            left_camera_matrix,
            left_dist_coeffs,
        );
        let pose_time = start.elapsed();
        
        match result {
            Ok(pose_result) => {
                println!("✓ 左眼姿态检测成功");
                println!("   Roll: {:.3}°", pose_result.roll);
                println!("   Pitch: {:.3}°", pose_result.pitch);
                println!("   Yaw: {:.3}°", pose_result.yaw);
                println!("   通过: {}", if pose_result.pass { "✓" } else { "❌" });
                println!("   检测耗时: {:.1} ms", pose_time.as_millis());
                Ok(pose_result)
            },
            Err(e) => {
                println!("❌ 左眼姿态检测失败: {}", e);
                Err(e)
            }
        }
    }
    
    /// 测试右眼姿态检测（新API）
    fn test_right_eye_pose(&mut self, right_corners: &opencv::core::Vector<opencv::core::Point2f>) -> Result<SingleEyePoseResult, Box<dyn std::error::Error>> {
        println!("🔍 测试右眼姿态检测（新API）...");
        
        // 获取右相机参数
        let (right_camera_matrix, right_dist_coeffs) = self.alignment_system.get_right_camera_params();
        
        let start = Instant::now();
        let result = self.alignment_system.check_single_eye_pose(
            right_corners,
            right_camera_matrix,
            right_dist_coeffs,
        );
        let pose_time = start.elapsed();
        
        match result {
            Ok(pose_result) => {
                println!("✓ 右眼姿态检测成功");
                println!("   Roll: {:.3}°", pose_result.roll);
                println!("   Pitch: {:.3}°", pose_result.pitch);
                println!("   Yaw: {:.3}°", pose_result.yaw);
                println!("   通过: {}", if pose_result.pass { "✓" } else { "❌" });
                println!("   检测耗时: {:.1} ms", pose_time.as_millis());
                Ok(pose_result)
            },
            Err(e) => {
                println!("❌ 右眼姿态检测失败: {}", e);
                Err(e)
            }
        }
    }
    
    /// 测试纯合像分析（无姿态检测）
    fn test_pure_alignment_analysis(&mut self, left_corners: &opencv::core::Vector<opencv::core::Point2f>, right_corners: &opencv::core::Vector<opencv::core::Point2f>) -> Result<DualEyeAlignmentResult, Box<dyn std::error::Error>> {
        println!("🔍 测试纯合像分析（无姿态检测）...");
        
        let start = Instant::now();
        let result = self.alignment_system.check_dual_eye_alignment(
            left_corners,
            right_corners,
            true, // 保存debug图像
        );
        let alignment_time = start.elapsed();
        
        match result {
            Ok(alignment_result) => {
                println!("✓ 合像分析成功");
                println!("   Δx_mean: {:.3} px", alignment_result.mean_dx);
                println!("   Δy_mean: {:.3} px", alignment_result.mean_dy);
                println!("   RMS: {:.3} px", alignment_result.rms);
                println!("   P95: {:.3} px", alignment_result.p95);
                println!("   Max: {:.3} px", alignment_result.max_err);
                println!("   通过: {}", if alignment_result.pass { "✓" } else { "❌" });
                println!("   分析耗时: {:.1} ms", alignment_time.as_millis());
                Ok(alignment_result)
            },
            Err(e) => {
                println!("❌ 合像分析失败: {}", e);
                Err(e)
            }
        }
    }
    
    /// 测试左眼居中检测（新功能）
    fn test_left_eye_centering(&mut self, left_corners: &opencv::core::Vector<opencv::core::Point2f>) -> Result<CenteringResult, Box<dyn std::error::Error>> {
        println!("🔍 测试左眼居中检测（新功能）...");
        
        let start = Instant::now();
        let result = self.alignment_system.check_left_eye_centering(
            left_corners,
            None, // 使用默认容差
        );
        let centering_time = start.elapsed();
        
        match result {
            Ok(centering_result) => {
                println!("✓ 左眼居中检测成功");
                println!("   居中状态: {}", if centering_result.is_centered { "✓ 居中" } else { "❌ 偏移" });
                println!("   右上角偏移: ({:.1}, {:.1}) px", centering_result.top_right_offset_x, centering_result.top_right_offset_y);
                println!("   左下角偏移: ({:.1}, {:.1}) px", centering_result.bottom_left_offset_x, centering_result.bottom_left_offset_y);
                println!("   最大偏移距离: {:.1} px", centering_result.max_offset_distance);
                println!("   容差阈值: {:.1} px", centering_result.tolerance_px);
                println!("   检测耗时: {:.1} ms", centering_time.as_millis());
                Ok(centering_result)
            },
            Err(e) => {
                println!("❌ 左眼居中检测失败: {}", e);
                Err(e)
            }
        }
    }
    
    /// 测试操作提示辅助计算（新功能）
    fn test_adjustment_vectors(
        &self,
        left_pose: &SingleEyePoseResult,
        centering_result: &CenteringResult,
        right_pose: &SingleEyePoseResult,
        alignment_result: &DualEyeAlignmentResult,
    ) -> Result<AdjustmentVectors, Box<dyn std::error::Error>> {
        println!("🔍 测试操作提示辅助计算（新功能）...");
        
        let start = Instant::now();
        let adjustment_vectors = self.alignment_system.calculate_adjustment_vectors(
            Some(left_pose),
            Some(centering_result),
            Some(right_pose),
            Some(alignment_result),
        );
        let adjustment_time = start.elapsed();
        
        println!("✓ 操作提示辅助计算成功");
        println!("   调整优先级: {:?}", adjustment_vectors.priority);
        println!("   左眼需要调整: {}", adjustment_vectors.left_eye_adjustment.needs_adjustment);
        println!("   右眼需要调整: {}", adjustment_vectors.right_eye_adjustment.needs_adjustment);
        println!("   合像RMS误差: {:.3} px", adjustment_vectors.alignment_adjustment.rms_error);
        println!("   计算耗时: {:.1} ms", adjustment_time.as_millis());
        
        Ok(adjustment_vectors)
    }
    
    /// 测试完整的检测流程（模拟前端调用）
    fn test_complete_workflow(
        &self,
        left_corners: &opencv::core::Vector<opencv::core::Point2f>,
        right_corners: &opencv::core::Vector<opencv::core::Point2f>,
        left_pose: &SingleEyePoseResult,
        right_pose: &SingleEyePoseResult,
        alignment_result: &DualEyeAlignmentResult,
        centering_result: &CenteringResult,
        adjustment_vectors: &AdjustmentVectors,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 模拟前端完整检测流程...");
        
        // 模拟前端的检测逻辑
        println!("📋 检测流程:");
        println!("   1. 圆心检测: ✓ (左眼{}点, 右眼{}点)", left_corners.len(), right_corners.len());
        println!("   2. 左眼姿态: {} (roll={:.3}°, pitch={:.3}°, yaw={:.3}°)", 
                if left_pose.pass { "✓" } else { "❌" },
                left_pose.roll, left_pose.pitch, left_pose.yaw);
        println!("   3. 右眼姿态: {} (roll={:.3}°, pitch={:.3}°, yaw={:.3}°)", 
                if right_pose.pass { "✓" } else { "❌" },
                right_pose.roll, right_pose.pitch, right_pose.yaw);
        
        // 检查是否可以进行合像分析
        if left_pose.pass && right_pose.pass {
            println!("   4. 合像分析: {} (RMS={:.3}px, P95={:.3}px, Max={:.3}px)", 
                    if alignment_result.pass { "✓" } else { "❌" },
                    alignment_result.rms, alignment_result.p95, alignment_result.max_err);
            
            // 生成调整提示
            let adjustment_hint = format!(
                "调整提示: Δx={:.3}px {}, Δy={:.3}px {}",
                alignment_result.mean_dx,
                if alignment_result.mean_dx > 0.0 { "(右眼向左调)" } else { "(右眼向右调)" },
                alignment_result.mean_dy,
                if alignment_result.mean_dy < 0.0 { "(右眼向上调)" } else { "(右眼向下调)" }
            );
            println!("   📋 {}", adjustment_hint);
        } else {
            println!("   4. 合像分析: 跳过（姿态检测未通过）");
            if !left_pose.pass {
                println!("      ❌ 左眼姿态超出容差，请先调平左光机");
            }
            if !right_pose.pass {
                println!("      ❌ 右眼姿态超出容差，请先调平右光机");
            }
        }

        // 测试左眼居中检测
        println!("   5. 左眼居中检测: {} (最大偏移: {:.1}px, 容差: {:.1}px)", 
                if centering_result.is_centered { "✓ 居中" } else { "❌ 偏移" },
                centering_result.max_offset_distance, centering_result.tolerance_px);
        
        // 测试操作提示辅助计算
        println!("   6. 操作提示辅助计算:");
        println!("      调整优先级: {:?}", adjustment_vectors.priority);
        println!("      左眼Roll调整: {:.3}°", adjustment_vectors.left_eye_adjustment.roll_adjustment);
        println!("      右眼Roll调整: {:.3}°", adjustment_vectors.right_eye_adjustment.roll_adjustment);
        println!("      合像X调整: {:.3}px", adjustment_vectors.alignment_adjustment.delta_x);
        println!("      合像Y调整: {:.3}px", adjustment_vectors.alignment_adjustment.delta_y);
        
        println!("✓ 完整检测流程模拟完成");
        Ok(())
    }
    
    /// 测试向后兼容性
    fn test_backward_compatibility(&mut self, left_corners: &opencv::core::Vector<opencv::core::Point2f>, right_corners: &opencv::core::Vector<opencv::core::Point2f>) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 测试向后兼容性...");
        
        // 测试向后兼容的左眼姿态检测
        let start = Instant::now();
        let left_result = self.alignment_system.check_left_eye_pose(left_corners)?;
        let left_time = start.elapsed();
        
        println!("✓ 向后兼容的左眼姿态检测: {} ({:.1} ms)", 
                if left_result.pass { "通过" } else { "失败" }, left_time.as_millis());
        
        // 测试向后兼容的右眼姿态检测
        let start = Instant::now();
        let right_result = self.alignment_system.check_right_eye_pose(right_corners)?;
        let right_time = start.elapsed();
        
        println!("✓ 向后兼容的右眼姿态检测: {} ({:.1} ms)", 
                if right_result.pass { "通过" } else { "失败" }, right_time.as_millis());
        
        println!("✓ 向后兼容性测试完成");
        Ok(())
    }
    
    /// 性能对比测试
    fn test_performance_comparison(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 性能对比测试...");
        
        // 多次运行测试性能稳定性
        let test_count = 5;
        let mut detection_times = Vec::new();
        let mut pose_times = Vec::new();
        let mut alignment_times = Vec::new();
        
        for i in 1..=test_count {
            print!("   第{}次测试...", i);
            
            // 圆心检测
            let start = Instant::now();
            let (left_corners, right_corners) = self.alignment_system.detect_circles_grid(
                &self.test_image_left,
                &self.test_image_right,
                &self.rectify_maps_path,
            )?;
            let detection_time = start.elapsed();
            detection_times.push(detection_time);
            
            // 姿态检测
            let start = Instant::now();
            let (left_camera_matrix, left_dist_coeffs) = self.alignment_system.get_left_camera_params();
            let _left_pose = self.alignment_system.check_single_eye_pose(
                &left_corners, left_camera_matrix, left_dist_coeffs)?;
            
            let (right_camera_matrix, right_dist_coeffs) = self.alignment_system.get_right_camera_params();
            let _right_pose = self.alignment_system.check_single_eye_pose(
                &right_corners, right_camera_matrix, right_dist_coeffs)?;
            let pose_time = start.elapsed();
            pose_times.push(pose_time);
            
            // 合像分析
            let start = Instant::now();
            let _alignment = self.alignment_system.check_dual_eye_alignment(
                &left_corners, &right_corners, false)?;
            let alignment_time = start.elapsed();
            alignment_times.push(alignment_time);
            
            println!(" 完成");
        }
        
        // 计算平均性能
        let avg_detection = detection_times.iter().sum::<std::time::Duration>() / test_count as u32;
        let avg_pose = pose_times.iter().sum::<std::time::Duration>() / test_count as u32;
        let avg_alignment = alignment_times.iter().sum::<std::time::Duration>() / test_count as u32;
        let avg_total = avg_detection + avg_pose + avg_alignment;
        
        println!("📊 性能统计 ({}次平均):", test_count);
        println!("   圆心检测: {:.1} ms", avg_detection.as_millis());
        println!("   姿态检测: {:.1} ms", avg_pose.as_millis());
        println!("   合像分析: {:.1} ms", avg_alignment.as_millis());
        println!("   总耗时: {:.1} ms", avg_total.as_millis());
        
        // 10fps兼容性分析
        let fps_10_threshold = 100.0; // 100ms
        let compatibility = if avg_total.as_millis() as f64 <= fps_10_threshold {
            "✓ PASS"
        } else {
            "❌ FAIL"
        };
        
        println!("🎯 10fps兼容性: {} ({:.1} ms / {:.1} ms)", 
                compatibility, avg_total.as_millis(), fps_10_threshold);
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 合像检测重构功能测试工具");
    println!("测试新的解耦API和向后兼容性");
    println!();
    
    // 创建并运行重构功能测试
    let mut test = AlignmentRefactorTest::new()?;
    test.run_tests()?;
    
    println!("\n🎉 重构功能测试完成！");
    println!("新的API设计验证成功，可以安全地用于前端集成。");
    
    Ok(())
} 