// simple_alignment_test.rs - 简化的合像检测测试程序
// 专门用于快速诊断圆点检测问题

use std::time::Instant;
use opencv::{core, imgcodecs, prelude::*};
use merging_image_lib::modules::alignment::AlignmentSystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动简化合像检测测试");
    println!("{}", "=".repeat(50));
    
    // 1. 确定文件路径
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    let src_tauri_dir = exe_dir.parent().unwrap().parent().unwrap();
    
    println!("📍 项目目录: {:?}", src_tauri_dir);
    
    // 2. 检查必需文件
    let img_left = src_tauri_dir.join("src/tests/data/benchmark/left_calibration.bmp");
    let img_right = src_tauri_dir.join("src/tests/data/benchmark/right_calibration.bmp");
            // 🔧 修正重映射矩阵路径 - 使用yaml_last_param_file目录
        let rectify_maps = src_tauri_dir.join("yaml_last_param_file/rectify_maps.yaml");
    
    println!("\n📁 检查文件存在性:");
    println!("   左图: {:?} - {}", img_left, if img_left.exists() { "✓" } else { "❌" });
    println!("   右图: {:?} - {}", img_right, if img_right.exists() { "✓" } else { "❌" });
    println!("   重映射: {:?} - {}", rectify_maps, if rectify_maps.exists() { "✓" } else { "❌" });
    
    if !img_left.exists() || !img_right.exists() {
        return Err("测试图像文件不存在".into());
    }
    
    // 3. 加载图像
    println!("\n📷 加载测试图像...");
    let left_img = imgcodecs::imread(img_left.to_str().unwrap(), imgcodecs::IMREAD_GRAYSCALE)?;
    let right_img = imgcodecs::imread(img_right.to_str().unwrap(), imgcodecs::IMREAD_GRAYSCALE)?;
    
    if left_img.empty() || right_img.empty() {
        return Err("图像加载失败".into());
    }
    
    let img_size = left_img.size()?;
    println!("✓ 图像加载成功: {}×{}", img_size.width, img_size.height);
    
    // 4. 检查图像质量
    println!("\n📊 图像质量检查:");
    let mut min_val = 0.0;
    let mut max_val = 0.0;
    core::min_max_loc(&left_img, Some(&mut min_val), Some(&mut max_val), None, None, &core::no_array())?;
    println!("   左图灰度范围: [{:.0}, {:.0}]", min_val, max_val);
    
    core::min_max_loc(&right_img, Some(&mut min_val), Some(&mut max_val), None, None, &core::no_array())?;
    println!("   右图灰度范围: [{:.0}, {:.0}]", min_val, max_val);
    
    // 5. 初始化检测系统
    println!("\n🔧 初始化检测系统...");
            // 🔧 修正参数文件路径 - 使用yaml_last_param_file目录
        // 旧路径 (注释掉):
        // let left_params = src_tauri_dir.join("left_camera_params.yaml");
        // let right_params = src_tauri_dir.join("right_camera_params.yaml");
        // let stereo_params = src_tauri_dir.join("stereo_params.yaml");
        // let rectify_params = src_tauri_dir.join("rectify_params.yaml");
        
        let left_params = src_tauri_dir.join("yaml_last_param_file/left_camera_params.yaml");
        let right_params = src_tauri_dir.join("yaml_last_param_file/right_camera_params.yaml");
        let stereo_params = src_tauri_dir.join("yaml_last_param_file/stereo_params.yaml");
        let rectify_params = src_tauri_dir.join("yaml_last_param_file/rectify_params.yaml");
    
    // 检查标定参数文件
    let param_files = [
        ("左相机", &left_params),
        ("右相机", &right_params), 
        ("双目", &stereo_params),
        ("校正", &rectify_params),
    ];
    
    println!("📋 检查标定参数文件:");
    for (name, file) in &param_files {
        println!("   {}: {} - {}", name, file.file_name().unwrap().to_str().unwrap(), 
                if file.exists() { "✓" } else { "❌" });
        if !file.exists() {
            return Err(format!("{}参数文件不存在: {:?}", name, file).into());
        }
    }
    
    // 创建检测系统
    let mut alignment_system = if rectify_maps.exists() {
        println!("使用预加载模式创建系统...");
        AlignmentSystem::new_with_preload(
            img_size,
            left_params.to_str().unwrap(),
            right_params.to_str().unwrap(),
            stereo_params.to_str().unwrap(),
            rectify_params.to_str().unwrap(),
            rectify_maps.to_str().unwrap(),
        )?
    } else {
        println!("使用普通模式创建系统...");
        AlignmentSystem::new(
            img_size,
            left_params.to_str().unwrap(),
            right_params.to_str().unwrap(),
            stereo_params.to_str().unwrap(),
            rectify_params.to_str().unwrap(),
        )?
    };
    
    println!("✓ 检测系统初始化完成");
    
    // 6. 执行圆点检测测试
    println!("\n🔍 执行圆点检测测试...");
    println!("{}", "-".repeat(40));
    
    let start_time = Instant::now();
    let result = alignment_system.detect_circles_grid(
        &left_img,
        &right_img,
        rectify_maps.to_str().unwrap(),
    );
    let elapsed = start_time.elapsed();
    
    match result {
        Ok((left_corners, right_corners)) => {
            println!("🎉 圆点检测成功!");
            println!("   左眼: {} 个圆点", left_corners.len());
            println!("   右眼: {} 个圆点", right_corners.len());
            println!("   耗时: {:.1} ms", elapsed.as_millis());
            
            if left_corners.len() == 40 && right_corners.len() == 40 {
                println!("✓ 圆点数量正确");
                
                // 简单的姿态检测测试
                println!("\n🎯 快速姿态检测测试:");
                let left_pose = alignment_system.check_left_eye_pose(&left_corners)?;
                let right_pose = alignment_system.check_right_eye_pose(&right_corners)?;
                
                println!("   左眼姿态: roll={:.2}°, pitch={:.2}°, yaw={:.2}°, 通过={}", 
                        left_pose.roll, left_pose.pitch, left_pose.yaw, left_pose.pass);
                println!("   右眼姿态: roll={:.2}°, pitch={:.2}°, yaw={:.2}°, 通过={}", 
                        right_pose.roll, right_pose.pitch, right_pose.yaw, right_pose.pass);
                
                // 如果姿态都通过，测试合像
                if left_pose.pass && right_pose.pass {
                    let alignment = alignment_system.check_dual_eye_alignment(&left_corners, &right_corners, true)?;
                    println!("   合像检测: RMS={:.2}px, 通过={}", alignment.rms, alignment.pass);
                    println!("✅ 完整检测流程成功!");
                } else {
                    println!("⚠️ 姿态检测未完全通过，但圆点检测正常");
                }
                
            } else {
                println!("⚠️ 圆点数量异常 (期望40个)");
            }
        }
        Err(e) => {
            println!("❌ 圆点检测失败: {}", e);
            println!("\n🔍 可能的解决方案:");
            println!("   1. 检查图像中是否包含标定板");
            println!("   2. 检查图像质量和对比度");
            println!("   3. 检查重映射参数是否正确");
            println!("   4. 尝试调整SimpleBlobDetector参数");
            
            return Err(format!("检测失败: {}", e).into());
        }
    }
    
    println!("\n🎉 测试完成!");
    println!("如果检测失败，请检查:");
    println!("   - 图像文件是否正确");
    println!("   - 标定参数文件是否完整");
    println!("   - 重映射文件是否存在");
    
    Ok(())
} 