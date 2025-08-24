//! 标定工作流程硬件测试 - 真实相机标定验证
//! 
//! 这是一个需要真实硬件的完整标定流程测试程序
//! 
//! ## ⚠️ 硬件要求
//! 
//! - 海康双目相机已连接
//! - 10×4 Asymmetric Circles 标定板
//! - 充足的光照条件
//! 
//! ## 🎯 测试目标
//! 
//! 1. 真实相机启动和图像采集
//! 2. 标定板检测和验证
//! 3. 完整的标定算法流程
//! 4. 标定结果验证和保存
//! 
//! ## 🚀 运行方式
//! 
//! ```bash
//! # 确保相机连接后运行
//! cargo run --bin calibration_hardware_test
//! ```

use merging_image_lib::modules::calibration_workflow::*;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() {
    println!("🎯 标定工作流程硬件测试 - 真实相机验证");
    println!("=========================================================");
    println!("⚠️  本测试需要连接海康双目相机和标定板");
    println!("📋 请确保：");
    println!("   1. 双目相机已正确连接");
    println!("   2. 准备好 10×4 Asymmetric Circles 标定板");
    println!("   3. 光照条件良好");
    println!("=========================================================\n");

    // 询问用户是否继续
    print!("是否继续硬件测试? (y/N): ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    
    if !input.trim().to_lowercase().starts_with('y') {
        println!("❌ 用户取消测试");
        return;
    }

    // 执行硬件测试
    if let Err(e) = run_hardware_calibration_test() {
        println!("❌ 硬件测试失败: {}", e);
        std::process::exit(1);
    } else {
        println!("🎉 硬件测试完成！");
        std::process::exit(0);
    }
}

fn run_hardware_calibration_test() -> Result<(), String> {
    println!("🧪 开始硬件标定测试...\n");

    // 步骤1: 初始化标定工作流程
    println!("📋 步骤1: 初始化标定工作流程");
    let mut workflow = CalibrationWorkflow::new()
        .map_err(|e| format!("标定工作流程初始化失败: {}", e))?;
    
    println!("✅ 标定工作流程初始化成功");
    println!("   初始状态: {:?}\n", workflow.get_status());

    // 步骤2: 启动标定会话
    println!("📋 步骤2: 启动标定会话");
    workflow.start_calibration()
        .map_err(|e| format!("启动标定会话失败: {}", e))?;
    
    println!("✅ 标定会话启动成功");
    println!("   当前状态: {:?}", workflow.get_status());
    println!("   相机已启动，开始15fps连续采集\n");

    // 步骤3: 自动化图像采集
    println!("📋 步骤3: 自动化图像采集");
    println!("📸 系统将自动拍摄图像，请根据提示调整标定板位置");
    println!("⏱️  每次拍摄前有3秒倒计时，如需停止请按 Ctrl+C");
    
    let target_count = 15;
    let min_valid_count = 10;
    let mut successful_captures = 0;
    let mut attempt_count = 0;

    loop {
        attempt_count += 1;
        let current_images = workflow.get_captured_images();
        let valid_count = current_images.iter().filter(|img| img.has_calibration_pattern).count();
        
        // 检查是否已满足条件
        if current_images.len() >= target_count && valid_count >= min_valid_count {
            println!("\n✅ 采集完成！总共{}组图像，其中{}组有效", current_images.len(), valid_count);
            break;
        }
        
        println!("\n🎯 准备拍摄第 {} 组图像 (当前: {}/{}，有效: {})", 
                 attempt_count, current_images.len(), target_count, valid_count);
        println!("📝 请：");
        println!("   1. 将标定板放置在相机视野内");
        println!("   2. 确保左右相机都能清楚看到标定板");
        println!("   3. 避免反光和阴影");
        
        println!("⏱️  5秒后自动拍摄，请确保标定板完全静止！");
        for i in (1..=5).rev() {
            if i <= 2 {
                print!("   🚨 {}...", i); // 最后2秒用红色警告
            } else {
                print!("   {}...", i);
            }
            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }
        println!(" 📸 拍摄！");

        // 拍摄图像
        match workflow.capture_calibration_pair() {
            Ok(image_pair) => {
                println!("✅ 第 {} 组图像采集成功:", attempt_count);
                println!("   - 图像对ID: {}", image_pair.pair_id);
                println!("   - 左图路径: {}", image_pair.left_image_path);
                println!("   - 右图路径: {}", image_pair.right_image_path);
                println!("   - 检测到标定板: {}", 
                    if image_pair.has_calibration_pattern { "是" } else { "否" });
                println!("   - 采集时间: {}", image_pair.capture_timestamp);
                
                if image_pair.has_calibration_pattern {
                    successful_captures += 1;
                    println!("   🎉 有效标定图像 +1 (总计: {})", successful_captures);
                } else {
                    println!("   ⚠️  未检测到标定板，建议重新拍摄");
                }
            }
            Err(e) => {
                println!("❌ 第 {} 组图像采集失败: {}", attempt_count, e);
                print!("是否继续? (y/N): ");
                io::stdout().flush().unwrap();
                let mut continue_input = String::new();
                io::stdin().read_line(&mut continue_input).unwrap();
                
                if !continue_input.trim().to_lowercase().starts_with('y') {
                    return Err("用户中止测试".to_string());
                }
            }
        }

        // 显示当前状态
        let current_images = workflow.get_captured_images();
        let valid_count = current_images.iter().filter(|img| img.has_calibration_pattern).count();
        println!("   📊 当前进度: 已采集 {} 组图像，其中 {} 组有效", current_images.len(), valid_count);
    }

    // 检查是否有足够的有效图像
    let captured_images = workflow.get_captured_images();
    let valid_images: Vec<_> = captured_images.iter()
        .filter(|img| img.has_calibration_pattern)
        .collect();
    
    println!("\n📊 图像采集总结:");
    println!("   - 总采集图像: {} 组", captured_images.len());
    println!("   - 有效标定图像: {} 组", valid_images.len());
    
    if valid_images.len() < 10 {
        return Err(format!(
            "有效标定图像不足: {}/10，请重新采集更多图像", 
            valid_images.len()
        ));
    }

    // 步骤4: 执行标定算法
    println!("\n📋 步骤4: 执行标定算法");
    println!("🔄 开始标定计算，这可能需要几十秒时间...");
    
    let start_time = std::time::Instant::now();
    
    match workflow.run_calibration() {
        Ok(result) => {
            let duration = start_time.elapsed();
            println!("✅ 标定算法执行成功！");
            println!("   ⏱️  耗时: {:?}", duration);
            println!("   📊 标定结果:");
            println!("      - 成功: {}", result.success);
            println!("      - 左相机RMS误差: {:.4}", result.left_rms_error);
            println!("      - 右相机RMS误差: {:.4}", result.right_rms_error);
            println!("      - 双目RMS误差: {:.4}", result.stereo_rms_error);
            println!("      - 误差阈值: {:.4}", result.error_threshold);
            println!("      - 标定时间: {}", result.calibration_time);
            
            if let Some(error_msg) = &result.error_message {
                println!("      ⚠️  警告信息: {}", error_msg);
            }
            
            // 验证标定质量
            if result.left_rms_error < result.error_threshold && 
               result.right_rms_error < result.error_threshold &&
               result.stereo_rms_error < result.error_threshold {
                println!("   🎉 标定质量良好，误差在阈值范围内！");
            } else {
                println!("   ⚠️  标定误差较大，建议重新标定或检查图像质量");
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("❌ 标定算法执行失败 (耗时: {:?}): {}", duration, e);
            return Err(format!("标定失败: {}", e));
        }
    }

    // 步骤5: 验证最终状态
    println!("\n📋 步骤5: 验证最终状态");
    let final_status = workflow.get_status();
    println!("   最终状态: {:?}", final_status);
    
    match final_status {
        CalibrationStatus::Completed => {
            println!("✅ 标定流程完成！");
            println!("   📁 标定参数已保存到 yaml_last_param_file/ 目录");
            println!("   🖼️  标定图像已保存到 captures/ 目录");
        }
        CalibrationStatus::Failed(msg) => {
            return Err(format!("标定最终状态为失败: {}", msg));
        }
        _ => {
            return Err(format!("标定未正常完成，状态: {:?}", final_status));
        }
    }

    println!("\n🎉 硬件标定测试全部通过！");
    Ok(())
} 