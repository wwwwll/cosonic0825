//! 对比两个检测函数的差异
//! 
//! 测试get_image_points_and_obj_points_pairs vs detect_and_get_points_from_paths

use std::fs;
use opencv::{core::Size, imgcodecs, prelude::*};
use merging_image_lib::modules::calibration_circles::*;

fn main() {
    println!("🔍 对比两个检测函数的差异");
    println!("============================");
    
    // 创建标定器
    let mut calibrator = match Calibrator::new(
        Size::new(2448, 2048),
        15.0, // CIRCLE_DIAMETER
        25.0, // CENTER_DISTANCE  
        Size::new(4, 10), // PATTERN_COLS, PATTERN_ROWS
        1.0,  // ERROR_THRESHOLD
    ) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 创建标定器失败: {}", e);
            return;
        }
    };
    
    println!("✅ 标定器创建成功");
    
    // 测试1: 使用get_image_points_and_obj_points_pairs (成功的函数)
    println!("\n📋 测试1: get_image_points_and_obj_points_pairs (已知成功)");
    test_get_image_points_and_obj_points_pairs(&mut calibrator);
    
    // 测试2: 使用detect_and_get_points_from_paths (失败的函数)
    println!("\n📋 测试2: detect_and_get_points_from_paths (当前失败)");
    test_detect_and_get_points_from_paths(&mut calibrator);
    
    // 测试3: 手动单张图像对比
    println!("\n📋 测试3: 手动单张图像对比");
    test_single_image_comparison(&mut calibrator);
    
    // 新增测试4: 测试转换后的PNG文件
    println!("\n📋 测试4: 测试转换后的PNG文件 (关键测试)");
    test_converted_png_images(&mut calibrator);
}

fn test_get_image_points_and_obj_points_pairs(calibrator: &mut Calibrator) {
    let test_folder = r"C:\Users\Y000010\MVS\Data\point_5_4";
    
    match calibrator.get_image_points_and_obj_points_pairs(test_folder, CameraType::Left) {
        Ok((obj_points, img_points)) => {
            println!("✅ get_image_points_and_obj_points_pairs 成功");
            println!("   - 成功处理图像数: {}", img_points.len());
        },
        Err(e) => {
            println!("❌ get_image_points_and_obj_points_pairs 失败: {}", e);
        }
    }
}

fn test_detect_and_get_points_from_paths(calibrator: &mut Calibrator) {
    // 生成采集图像的路径
    let captured_folder = r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755064325";
    let mut left_paths = Vec::new();
    
    for i in 1..=10 {
        let path = format!("{}\\calib_left_{:02}.png", captured_folder, i);
        left_paths.push(path);
    }
    
    match calibrator.detect_and_get_points_from_paths(&left_paths, CameraType::Left) {
        Ok((obj_points, img_points)) => {
            println!("✅ detect_and_get_points_from_paths 成功");
            println!("   - 成功处理图像数: {}", img_points.len());
        },
        Err(e) => {
            println!("❌ detect_and_get_points_from_paths 失败: {}", e);
        }
    }
}

fn test_single_image_comparison(calibrator: &mut Calibrator) {
    // 测试一张已知成功的BMP图像
    let bmp_path = r"C:\Users\Y000010\MVS\Data\point_5_4\l_0.bmp";
    // 测试一张PNG图像
    let png_path = r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755064325\calib_left_01.png";
    
    println!("🧪 测试BMP图像: {}", bmp_path);
    test_single_image(calibrator, bmp_path);
    
    println!("🧪 测试PNG图像: {}", png_path);
    test_single_image(calibrator, png_path);
}

fn test_single_image(calibrator: &mut Calibrator, image_path: &str) {
    // 检查文件是否存在
    if !std::path::Path::new(image_path).exists() {
        println!("   ❌ 文件不存在: {}", image_path);
        return;
    }
    
    // 读取图像
    let img = match imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR) {
        Ok(img) if !img.empty() => {
            println!("   ✅ 图像读取成功: {}x{}", img.cols(), img.rows());
            img
        },
        _ => {
            println!("   ❌ 无法读取图像");
            return;
        }
    };
    
    // 测试检测
    match calibrator.find_asymmetric_circles_grid_points(&img, true) {
        Ok(centers) => {
            let expected = (4 * 10) as usize;
            println!("   ✅ 检测成功: 找到 {} 个点 (预期 {} 个)", centers.len(), expected);
            
            if centers.len() == expected {
                println!("   🎯 完全匹配！");
            } else {
                println!("   ⚠️  数量不匹配");
            }
        },
        Err(e) => {
            println!("   ❌ 检测失败: {}", e);
        }
    }
    
    // 测试quick_detect_calibration_pattern
    let quick_result = calibrator.quick_detect_calibration_pattern(&img);
    println!("   quick_detect_calibration_pattern: {}", 
        if quick_result { "✅ 成功" } else { "❌ 失败" });
}

fn test_converted_png_images(calibrator: &mut Calibrator) {
    println!("🎯 这是关键测试：使用相同内容但不同格式的图像");
    println!("   - 原始BMP: 已知成功");
    println!("   - 转换PNG: 测试是否成功");
    
    // 测试转换后的PNG文件
    let converted_folder = r"C:\Users\Y000010\MVS\Data\point_5_4\png";
    let mut converted_paths = Vec::new();
    
    for i in 0..9 {
        let path = format!("{}\\l_{}.png", converted_folder, i);
        converted_paths.push(path);
    }
    
    println!("\n🔍 测试1: 使用detect_and_get_points_from_paths处理转换后的PNG");
    match calibrator.detect_and_get_points_from_paths(&converted_paths, CameraType::Left) {
        Ok((obj_points, img_points)) => {
            println!("✅ 转换后的PNG文件检测成功！");
            println!("   - 成功处理图像数: {}", img_points.len());
            println!("🎯 结论: 问题不是图像内容，而是图像质量/格式");
        },
        Err(e) => {
            println!("❌ 转换后的PNG文件检测失败: {}", e);
            println!("🔍 进一步分析: 逐个测试转换后的PNG文件");
            
            // 逐个测试前3张转换后的PNG
            for (i, path) in converted_paths.iter().take(3).enumerate() {
                println!("\n📷 测试转换PNG #{}: {}", i, path);
                test_single_image(calibrator, path);
            }
        }
    }
    
    println!("\n🔍 测试2: 直接对比原始BMP vs 转换PNG");
    let bmp_path = r"C:\Users\Y000010\MVS\Data\point_5_4\l_0.bmp";
    let png_path = r"C:\Users\Y000010\MVS\Data\point_5_4\png\l_0.png";
    
    println!("📊 相同内容的图像对比:");
    println!("🖼️  原始BMP: {}", bmp_path);
    test_single_image(calibrator, bmp_path);
    
    println!("🖼️  转换PNG: {}", png_path);
    test_single_image(calibrator, png_path);
    
    // 分析结论
    println!("\n📋 分析结论:");
    println!("   如果转换PNG成功 → 问题是实时采集的图像质量");
    println!("   如果转换PNG失败 → 问题是PNG格式处理或OpenCV读取");
} 