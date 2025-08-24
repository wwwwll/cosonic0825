//! 标定板规格测试程序
//! 
//! 测试不同的标定板参数组合，找到正确的配置

use merging_image_lib::modules::calibration_circles::Calibrator;
use opencv::{core::Size, imgcodecs, prelude::*};

fn main() {
    println!("🎯 标定板规格测试程序");
    println!("======================");
    
    // 读取测试图像
    let test_image_path = "test_image_2448x2048.png";
    let image = match imgcodecs::imread(test_image_path, imgcodecs::IMREAD_GRAYSCALE) {
        Ok(img) if !img.empty() => {
            println!("✅ 成功读取测试图像: {}", test_image_path);
            println!("   图像尺寸: {}x{}", img.cols(), img.rows());
            img
        },
        _ => {
            println!("❌ 无法读取测试图像: {}", test_image_path);
            println!("💡 请先运行 camera_diagnosis 生成测试图像");
            return;
        }
    };
    
    println!("\n🔍 测试不同的标定板规格...\n");
    
    // 常见的标定板配置
    let test_configs = vec![
        // (pattern_size, circle_diameter, center_distance, description)
        (Size::new(10, 4), 5.0, 25.0, "10x4, 5mm圆, 25mm间距 (默认)"),
        (Size::new(4, 10), 5.0, 25.0, "4x10, 5mm圆, 25mm间距 (行列互换)"),
        (Size::new(7, 7), 5.0, 25.0, "7x7, 5mm圆, 25mm间距"),
        (Size::new(9, 6), 5.0, 25.0, "9x6, 5mm圆, 25mm间距"),
        (Size::new(11, 8), 5.0, 25.0, "11x8, 5mm圆, 25mm间距"),
        
        // 不同圆点直径
        (Size::new(10, 4), 3.0, 25.0, "10x4, 3mm圆, 25mm间距"),
        (Size::new(10, 4), 7.0, 25.0, "10x4, 7mm圆, 25mm间距"),
        (Size::new(10, 4), 10.0, 25.0, "10x4, 10mm圆, 25mm间距"),
        
        // 不同间距
        (Size::new(10, 4), 5.0, 15.0, "10x4, 5mm圆, 15mm间距"),
        (Size::new(10, 4), 5.0, 20.0, "10x4, 5mm圆, 20mm间距"),
        (Size::new(10, 4), 5.0, 30.0, "10x4, 5mm圆, 30mm间距"),
        (Size::new(10, 4), 5.0, 35.0, "10x4, 5mm圆, 35mm间距"),
        
        // 常见OpenCV标定板
        (Size::new(9, 6), 3.0, 15.0, "OpenCV样例 9x6"),
        (Size::new(7, 5), 4.0, 20.0, "OpenCV样例 7x5"),
    ];
    
    let mut successful_configs = Vec::new();
    
    for (i, (pattern_size, circle_diameter, center_distance, description)) in test_configs.iter().enumerate() {
        println!("🧪 测试 {}: {}", i + 1, description);
        
        let mut calibrator = match Calibrator::new(
            Size::new(image.cols(), image.rows()),
            *circle_diameter,
            *center_distance,
            *pattern_size,
            2.0, // error_threshold
        ) {
            Ok(c) => c,
            Err(e) => {
                println!("   ❌ 创建标定器失败: {}", e);
                continue;
            }
        };
        
        let detected = calibrator.quick_detect_calibration_pattern(&image);
        
        if detected {
            println!("   ✅ 检测成功！");
            successful_configs.push((pattern_size.clone(), *circle_diameter, *center_distance, description.clone()));
        } else {
            println!("   ❌ 检测失败");
        }
    }
    
    // 总结结果
    println!("\n📊 测试结果总结:");
    println!("================");
    
    if successful_configs.is_empty() {
        println!("❌ 没有找到匹配的标定板配置");
        println!("\n💡 可能的原因:");
        println!("   1. 图像中没有标定板");
        println!("   2. 标定板类型不是 asymmetric circles");
        println!("   3. 图像质量问题（模糊、过暗、过亮）");
        println!("   4. 标定板规格不在测试范围内");
        println!("\n🔧 建议:");
        println!("   1. 检查保存的测试图像 test_image_2448x2048.png");
        println!("   2. 确认标定板类型和规格");
        println!("   3. 调整相机曝光和增益设置");
        println!("   4. 确保标定板完全在视野内");
    } else {
        println!("✅ 找到 {} 个匹配的配置:", successful_configs.len());
        for (i, (pattern_size, circle_diameter, center_distance, description)) in successful_configs.iter().enumerate() {
            println!("   {}. {}", i + 1, description);
            println!("      Pattern: {}x{}, Circle: {}mm, Distance: {}mm", 
                pattern_size.width, pattern_size.height, circle_diameter, center_distance);
        }
        
        if let Some((pattern_size, circle_diameter, center_distance, _)) = successful_configs.first() {
            println!("\n🎯 建议使用第一个配置:");
            println!("   circle_diameter: {}mm", circle_diameter);
            println!("   center_distance: {}mm", center_distance);
            println!("   pattern_size: Size::new({}, {})", pattern_size.width, pattern_size.height);
        }
    }
} 