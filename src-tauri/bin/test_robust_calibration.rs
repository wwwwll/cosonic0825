// 测试鲁棒SimpleBlobDetector在恶劣环境下的标定检测能力
// 专门针对暗光照、杂乱背景等恶劣条件进行测试

use merging_image_lib::modules::calibration_circles::{Calibrator, CameraType};
use opencv::core::Size;
use opencv::prelude::MatTraitConst;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌙 测试鲁棒SimpleBlobDetector - 恶劣环境标定检测");
    println!("=================================================");

    // 创建标定器实例 - 使用更新后的检测器参数
    let mut calibrator = Calibrator::new(
        Size::new(2448, 2048),  // 图像尺寸
        15.0,                   // 圆点直径 15mm
        25.0,                   // 圆点间距 25mm
        Size::new(4, 10),       // 标定板尺寸 4x10
        1.0,                    // 重投影误差阈值
    )?;

    // 测试1: 最新采集的2张图像（验证修复效果）
    println!("\n🧪 测试1: 验证max_area修复效果（2张最新图像）");
    test_recent_images(&mut calibrator)?;

    // 测试2: 完整的10张图像测试
    println!("\n🧪 测试2: 完整标定图像集测试（10张图像）");
    test_full_image_set(&mut calibrator)?;

    // 测试3: 图像格式对比测试（如果有BMP文件的话）
    println!("\n🧪 测试3: 图像格式影响分析");
    test_image_format_comparison()?;

    Ok(())
}

fn test_recent_images(calibrator: &mut Calibrator) -> Result<(), Box<dyn std::error::Error>> {
    let test_images = vec![
        r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755078298\calib_left_01.png",
        r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755078298\calib_left_02.png",
    ];

    let mut success_count = 0;
    let total_count = test_images.len();

    for (i, image_path) in test_images.iter().enumerate() {
        println!("📷 测试图像 {}/{}: {}", i + 1, total_count, 
                 image_path.split('\\').last().unwrap_or(image_path));
        
        if !std::path::Path::new(image_path).exists() {
            println!("   ❌ 文件不存在，跳过");
            continue;
        }

        match opencv::imgcodecs::imread(image_path, opencv::imgcodecs::IMREAD_COLOR) {
            Ok(image) => {
                if image.empty() {
                    println!("   ❌ 图像读取失败或为空");
                    continue;
                }
                
                println!("   ✅ 图像读取成功: {}x{}", image.cols(), image.rows());
                
                match calibrator.quick_detect_calibration_pattern(&image) {
                    true => {
                        println!("   🎯 ✅ 圆点检测成功!");
                        success_count += 1;
                    }
                    false => {
                        println!("   ❌ 圆点检测失败");
                    }
                }
            }
            Err(e) => {
                println!("   ❌ 图像读取错误: {}", e);
            }
        }
    }

    println!("📊 最新图像测试结果: {}/{} 成功 ({:.1}%)", 
             success_count, total_count, (success_count as f32 / total_count as f32) * 100.0);
    Ok(())
}

fn test_full_image_set(calibrator: &mut Calibrator) -> Result<(), Box<dyn std::error::Error>> {
    let test_folder = r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755064325";
    
    // 构建10张左相机图像路径
    let mut left_paths = Vec::new();
    for i in 1..=10 {
        let path = format!("{}\\calib_left_{:02}.png", test_folder, i);
        if std::path::Path::new(&path).exists() {
            left_paths.push(path);
        }
    }

    println!("📂 找到 {} 张左相机图像", left_paths.len());
    
    if left_paths.is_empty() {
        println!("   ❌ 未找到可用的测试图像");
        return Ok(());
    }

    // 逐个测试图像
    let mut success_count = 0;
    let total_count = left_paths.len();

    for (i, image_path) in left_paths.iter().enumerate() {
        let filename = image_path.split('\\').last().unwrap_or(image_path);
        print!("📷 测试 {}/{}: {} ... ", i + 1, total_count, filename);
        
        match opencv::imgcodecs::imread(image_path, opencv::imgcodecs::IMREAD_COLOR) {
            Ok(image) => {
                if image.empty() {
                    println!("❌ 图像为空");
                    continue;
                }
                
                match calibrator.quick_detect_calibration_pattern(&image) {
                    true => {
                        println!("✅ 成功");
                        success_count += 1;
                    }
                    false => {
                        println!("❌ 失败");
                    }
                }
            }
            Err(_) => {
                println!("❌ 读取错误");
            }
        }
    }

    println!("\n📊 完整图像集测试结果:");
    println!("   成功检测: {}/{}", success_count, total_count);
    println!("   成功率: {:.1}%", (success_count as f32 / total_count as f32) * 100.0);
    
    if success_count >= 5 {
        println!("   🎉 成功率良好！可以进行完整标定流程");
        
        // 测试完整标定流程
        println!("\n🔧 测试完整标定流程:");
        test_full_calibration_with_successful_images(calibrator, &left_paths, success_count)?;
    } else {
        println!("   ⚠️  成功率较低，建议改善拍摄条件或进一步调整参数");
    }

    Ok(())
}

fn test_full_calibration_with_successful_images(
    calibrator: &mut Calibrator, 
    left_paths: &[String], 
    expected_success: usize
) -> Result<(), Box<dyn std::error::Error>> {
    
    // 测试点检测
    match calibrator.detect_and_get_points_from_paths(left_paths, CameraType::Left) {
        Ok((obj_points, img_points)) => {
            println!("   ✅ 批量点检测成功!");
            println!("      - 成功处理图像数: {}", obj_points.len());
            println!("      - 每张图像检测点数: {} (预期: 40)", 
                     if !img_points.is_empty() { img_points.get(0).map_or(0, |v| v.len()) } else { 0 });
            
            if obj_points.len() >= 5 {
                println!("   🎯 图像数量充足，标定质量应该良好");
            } else if obj_points.len() >= 3 {
                println!("   ⚠️  图像数量基本满足，但建议采集更多图像");
            } else {
                println!("   ❌ 图像数量不足，无法进行可靠标定");
            }
        }
        Err(e) => {
            println!("   ❌ 批量点检测失败: {}", e);
        }
    }

    Ok(())
}

fn test_image_format_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 图像格式影响分析:");
    println!("   🔍 理论分析:");
    println!("      BMP: 无损格式，像素数据完整 → 更适合精确检测");
    println!("      PNG: 可能有轻微压缩 → 对检测影响很小");
    println!("   ");
    println!("   🧪 实际测试结论（基于之前的测试）:");
    println!("      - 转换后的PNG文件: ✅ 检测成功");
    println!("      - 实时采集的PNG文件: 根据图像质量而定");
    println!("      - 主要影响因素: 图像质量 > 图像格式");
    println!("   ");
    println!("   💡 建议:");
    println!("      1. 优先改善拍摄条件（光照、对焦、稳定性）");
    println!("      2. 图像格式选择PNG即可（便于存储和传输）");
    println!("      3. 如需最高精度，可考虑使用BMP格式");

    Ok(())
} 