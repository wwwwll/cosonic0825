/**
 * @file simple_camera_manager_test.rs
 * @brief SimpleCameraManager 功能测试程序
 * 
 * 测试新的简化相机管理器的所有功能：
 * 1. 基础生命周期 (new -> start -> stop)
 * 2. 图像采集 (capture_and_process)
 * 3. 保存功能测试
 * 4. 连续采集性能测试
 * 5. 错误处理测试
 * 
 * @version 1.0
 * @date 2025-01-15
 */

use std::{thread, time::Duration};

// 导入我们的SimpleCameraManager
use merging_image_lib::camera_manager::{SimpleCameraManager, CameraError};

fn main() {
    println!("=== SimpleCameraManager 功能测试程序 ===");
    println!();
    
    // 运行所有测试
    if let Err(e) = run_all_tests() {
        eprintln!("❌ 测试失败: {}", e);
        std::process::exit(1);
    }
    
    println!();
    println!("🎉 所有测试通过！SimpleCameraManager 工作正常");
    
    // 等待用户输入
    println!("\n按Enter键退出...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}

fn run_all_tests() -> Result<(), Box<dyn std::error::Error>> {
    // 测试1: 基础生命周期
    test_basic_lifecycle()?;
    
    // 测试2: 错误处理
    test_error_handling()?;
    
    // 测试3: 图像采集
    test_image_capture()?;
    
    // 测试4: 保存功能
    test_save_functionality()?;
    
    // 测试5: 连续采集性能
    test_continuous_capture_performance()?;
    
    Ok(())
}

/// 测试1: 基础生命周期
fn test_basic_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试1: 基础生命周期");
    
    // 创建管理器
    println!("   创建SimpleCameraManager...");
    let manager = SimpleCameraManager::new()?;
    assert!(!manager.is_running(), "初始状态应该是未运行");
    println!("   ✅ 创建成功，初始状态正确");
    
    // 启动相机
    println!("   启动相机...");
    manager.start()?;
    assert!(manager.is_running(), "启动后状态应该是运行中");
    println!("   ✅ 启动成功，状态正确");
    
    // 测试重复启动
    println!("   测试重复启动...");
    match manager.start() {
        Err(CameraError::AlreadyStarted) => println!("   ✅ 正确检测到重复启动"),
        _ => return Err("应该返回AlreadyStarted错误".into()),
    }
    
    // 停止相机
    println!("   停止相机...");
    manager.stop()?;
    assert!(!manager.is_running(), "停止后状态应该是未运行");
    println!("   ✅ 停止成功，状态正确");
    
    println!("✅ 测试1通过: 基础生命周期正常");
    println!();
    
    Ok(())
}

/// 测试2: 错误处理
fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试2: 错误处理");
    
    let manager = SimpleCameraManager::new()?;
    
    // 测试未启动状态下的采集
    println!("   测试未启动状态下的采集...");
    match manager.capture_and_process(false) {
        Err(CameraError::NotStarted) => println!("   ✅ 正确检测到相机未启动"),
        _ => return Err("应该返回NotStarted错误".into()),
    }

    manager.start()?;
    manager.stop()?;
    
    println!("✅ 测试2通过: 错误处理正常");
    println!();
    
    Ok(())
}

/// 测试3: 图像采集
fn test_image_capture() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试3: 图像采集");
    
    let manager = SimpleCameraManager::new()?;
    manager.start()?;
    
    // 等待相机稳定
    println!("   等待相机稳定...");
    thread::sleep(Duration::from_millis(1000));
    
    // 测试不保存模式
    println!("   测试不保存模式...");
    let (left, right) = manager.capture_and_process(false)?;
    
    assert!(!left.is_empty(), "左图像数据不应为空");
    assert!(!right.is_empty(), "右图像数据不应为空");
    println!("   ✅ 不保存模式成功 (Left: {} bytes, Right: {} bytes)", left.len(), right.len());
    
    // 验证数据大小合理性
    let expected_size = manager.get_frame_buffer_size() as usize;
    assert!(left.len() <= expected_size, "左图像大小应该合理");
    assert!(right.len() <= expected_size, "右图像大小应该合理");
    println!("   ✅ 图像大小验证通过");
    
    manager.stop()?;
    println!("✅ 测试3通过: 图像采集正常");
    println!();
    
    Ok(())
}

/// 测试4: 保存功能
fn test_save_functionality() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试4: 保存功能");
    
    let manager = SimpleCameraManager::new()?;
    manager.start()?;
    
    // 等待相机稳定
    thread::sleep(Duration::from_millis(500));
    
    // 测试保存模式
    println!("   测试保存模式...");
    let (left, right) = manager.capture_and_process(true)?;
    
    assert!(!left.is_empty(), "左图像数据不应为空");
    assert!(!right.is_empty(), "右图像数据不应为空");
    println!("   ✅ 保存模式成功 (Left: {} bytes, Right: {} bytes)", left.len(), right.len());
    
    // 检查文件是否存在
    println!("   检查保存的文件...");
    let captures_dir = std::path::Path::new("captures");
    if captures_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(captures_dir)?
            .filter_map(|entry| entry.ok())
            .collect();
        
        if entries.len() >= 2 {
            println!("   ✅ 文件保存成功，找到 {} 个文件", entries.len());
        } else {
            return Err("保存的文件数量不正确".into());
        }
    } else {
        return Err("captures目录不存在".into());
    }
    
    manager.stop()?;
    println!("✅ 测试4通过: 保存功能正常");
    println!();
    
    Ok(())
}

/// 测试5: 连续采集性能
fn test_continuous_capture_performance() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试5: 连续采集性能");
    
    let manager = SimpleCameraManager::new()?;
    manager.start()?;
    
    // 等待相机稳定
    thread::sleep(Duration::from_millis(500));
    
    // 连续采集测试
    let test_frames = 10;
    let start_time = std::time::Instant::now();
    
    println!("   开始连续采集 {} 帧...", test_frames);
    
    for i in 1..=test_frames {
        let frame_start = std::time::Instant::now();
        
        let (left, right) = manager.capture_and_process(false)?;
        
        let frame_time = frame_start.elapsed();
        
        assert!(!left.is_empty() && !right.is_empty(), "图像数据不应为空");
        
        if i % 3 == 0 {  // 每3帧打印一次
            println!("   第{}帧: {} bytes + {} bytes, 用时 {:.1}ms", 
                     i, left.len(), right.len(), frame_time.as_millis());
        }
        
        // 模拟15fps间隔 (67ms)
        thread::sleep(Duration::from_millis(67));
    }
    
    let total_time = start_time.elapsed();
    let avg_fps = test_frames as f64 / total_time.as_secs_f64();
    
    println!("   ✅ 连续采集完成:");
    println!("      - 总时间: {:.1}秒", total_time.as_secs_f64());
    println!("      - 平均帧率: {:.1} fps", avg_fps);
    println!("      - 目标帧率: 15.0 fps");
    
    // 验证帧率合理性 (允许一定误差)
    if avg_fps >= 10.0 && avg_fps <= 20.0 {
        println!("   ✅ 帧率在合理范围内");
    } else {
        return Err(format!("帧率异常: {:.1} fps", avg_fps).into());
    }
    
    manager.stop()?;
    println!("✅ 测试5通过: 连续采集性能正常");
    println!();
    
    Ok(())
}

/// 辅助函数：清理测试文件
#[allow(dead_code)]
fn cleanup_test_files() -> Result<(), Box<dyn std::error::Error>> {
    let captures_dir = std::path::Path::new("captures");
    if captures_dir.exists() {
        std::fs::remove_dir_all(captures_dir)?;
        println!("🧹 清理测试文件完成");
    }
    Ok(())
} 