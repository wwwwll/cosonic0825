//! 测试已保存的标定图像
//! 
//! 使用captures目录下的图像测试完整的标定流程

use std::fs;
use std::path::Path;
use opencv::{core::Size, imgcodecs, prelude::*};
use merging_image_lib::modules::calibration_circles::*;

fn main() {
    println!("🎯 测试本地标定图像 (使用calibration_circles_test.rs相同的图像)");
    println!("============================================================");
    
    // 使用与calibration_circles_test.rs相同的本地图像路径
    let test_image_folder = r"C:\Users\Y000010\MVS\Data\point_5_4";
    println!("📂 使用图像目录: {}", test_image_folder);
    
    // 检查目录是否存在
    if !std::path::Path::new(test_image_folder).exists() {
        println!("❌ 图像目录不存在: {}", test_image_folder);
        println!("💡 请确保目录路径正确，并包含 l_0.bmp~l_8.bmp 和 r_0.bmp~r_8.bmp");
        return;
    }
    
    // 生成图像文件路径（与calibration_circles_test.rs相同的命名规则）
    let (left_images, right_images) = generate_local_image_paths(test_image_folder);
    
    println!("📸 预期图像:");
    println!("   左图: {} 张", left_images.len());
    println!("   右图: {} 张", right_images.len());
    
    // 检查文件是否存在
    let mut existing_left = 0;
    let mut existing_right = 0;
    
    for path in &left_images {
        if std::path::Path::new(path).exists() {
            existing_left += 1;
        }
    }
    
    for path in &right_images {
        if std::path::Path::new(path).exists() {
            existing_right += 1;
        }
    }
    
    println!("📸 实际存在:");
    println!("   左图: {} 张", existing_left);
    println!("   右图: {} 张", existing_right);
    
    if existing_left < 8 || existing_right < 8 {
        println!("⚠️ 图像数量不足，至少需要8组图像进行标定");
        println!("💡 请确保目录包含 l_0.bmp~l_8.bmp 和 r_0.bmp~r_8.bmp");
    }
    
    // 测试单张图像检测
    println!("\n🔍 测试单张图像标定板检测...");
    test_single_image_detection(&left_images, &right_images);
    
    // 测试完整标定流程
    println!("\n🚀 测试完整标定流程...");
    test_full_calibration(&left_images, &right_images);
}

fn generate_local_image_paths(base_dir: &str) -> (Vec<String>, Vec<String>) {
    let mut left_images = Vec::new();
    let mut right_images = Vec::new();
    
    // 生成与calibration_circles_test.rs相同的文件命名规则
    for i in 0..9 {  // l_0.bmp ~ l_8.bmp, r_0.bmp ~ r_8.bmp
        let left_path = format!("{}\\l_{}.bmp", base_dir, i);
        let right_path = format!("{}\\r_{}.bmp", base_dir, i);
        
        left_images.push(left_path);
        right_images.push(right_path);
    }
    
    (left_images, right_images)
}



fn test_single_image_detection(left_images: &[String], right_images: &[String]) {
    if left_images.is_empty() || right_images.is_empty() {
        println!("❌ 没有图像可测试");
        return;
    }
    
    // 测试所有图像，而不只是第一张
    println!("📸 测试所有图像的标定板检测...");
    
    let mut successful_left = 0;
    let mut successful_right = 0;
    
    for (i, (left_path, right_path)) in left_images.iter().zip(right_images.iter()).enumerate() {
        println!("\n📷 测试第{}组图像:", i);
        println!("   左图: {}", left_path);
        println!("   右图: {}", right_path);
        
        let left_image = match imgcodecs::imread(left_path, imgcodecs::IMREAD_GRAYSCALE) {
            Ok(img) if !img.empty() => img,
            _ => {
                println!("   ❌ 无法读取左图");
                continue;
            }
        };
        
        let right_image = match imgcodecs::imread(right_path, imgcodecs::IMREAD_GRAYSCALE) {
            Ok(img) if !img.empty() => img,
            _ => {
                println!("   ❌ 无法读取右图");
                continue;
            }
        };
        
        println!("   ✅ 图像读取成功: 左图{}x{}, 右图{}x{}", 
            left_image.cols(), left_image.rows(),
            right_image.cols(), right_image.rows());
        
        // 使用正确配置测试标定板检测
        let mut calibrator = match Calibrator::new(
            Size::new(left_image.cols(), left_image.rows()),
            15.0, // 正确的圆点直径
            25.0, // 正确的间距
            Size::new(4, 10), // 正确的模式
            1.0,
        ) {
            Ok(c) => c,
            Err(_) => continue,
        };
        
        let left_detected = calibrator.quick_detect_calibration_pattern(&left_image);
        let right_detected = calibrator.quick_detect_calibration_pattern(&right_image);
        
        println!("   检测结果: 左图{}, 右图{}", 
            if left_detected { "✅" } else { "❌" },
            if right_detected { "✅" } else { "❌" }
        );
        
        if left_detected { successful_left += 1; }
        if right_detected { successful_right += 1; }
    }
    
    println!("\n📊 单张图像检测总结:");
    println!("   成功检测的左图: {}/{}", successful_left, left_images.len());
    println!("   成功检测的右图: {}/{}", successful_right, right_images.len());
    
    if successful_left == 0 && successful_right == 0 {
        println!("❌ 所有单张图像检测都失败了");
    } else {
        println!("✅ 部分图像检测成功，这解释了为什么完整流程能成功");
    }
}


fn test_full_calibration(left_images: &[String], right_images: &[String]) {
    let min_images = left_images.len().min(right_images.len());
    if min_images < 8 {
        println!("❌ 图像数量不足 ({}/8)，跳过完整标定测试", min_images);
        return;
    }
    
    println!("🔬 开始完整标定流程测试...");
    
    // 使用正确的配置（来自calibration_circles_test.rs）
    let mut calibrator = match Calibrator::new(
        Size::new(2448, 2048),
        15.0, // circle_diameter (正确值)
        25.0, // center_distance
        Size::new(4, 10), // pattern_size (正确值：4列10行)
        1.0,  // error_threshold (与测试文件保持一致)
    ) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ 创建标定器失败: {}", e);
            return;
        }
    };
    
    // 使用前8组图像进行测试
    let test_count = 8.min(min_images);
    let left_test_images: Vec<String> = left_images.iter().take(test_count).cloned().collect();
    let right_test_images: Vec<String> = right_images.iter().take(test_count).cloned().collect();
    
    println!("📊 使用 {} 组图像进行标定", test_count);
    
    // Step 1: 检测左相机特征点 (使用与calibration_circles_test.rs相同的函数)
    println!("🔍 Step 1: 检测左相机特征点...");
    let (left_obj_points, left_img_points) = match calibrator.get_image_points_and_obj_points_pairs(
        r"C:\Users\Y000010\MVS\Data\point_5_4",
        CameraType::Left,
    ) {
        Ok(points) => {
            println!("✅ 左相机特征点检测成功");
            println!("   - 成功处理图像数: {}", points.1.len());
            points
        },
        Err(e) => {
            println!("❌ 左相机特征点检测失败: {}", e);
            return;
        }
    };
    
    println!("🔍 Step 2: 检测右相机特征点...");
    let (right_obj_points, right_img_points) = match calibrator.get_image_points_and_obj_points_pairs(
        r"C:\Users\Y000010\MVS\Data\point_5_4",
        CameraType::Right,
    ) {
        Ok(points) => {
            println!("✅ 右相机特征点检测成功");
            println!("   - 成功处理图像数: {}", points.1.len());
            points
        },
        Err(e) => {
            println!("❌ 右相机特征点检测失败: {}", e);
            return;
        }
    };
    
    // Step 2: 左相机标定
    println!("📷 Step 3: 左相机单目标定...");
    let left_result = match calibrator.calibrate_mono(&left_obj_points, &left_img_points) {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 左相机标定失败: {}", e);
            return;
        }
    };
    
    let (left_camera, left_error) = match left_result {
        MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
            println!("✅ 左相机标定成功，RMS误差: {:.4}", error);
            (MonoCamera { camera_matrix, dist_coeffs }, error)
        },
        MonoCalibResult::NeedRecalibration(error) => {
            println!("❌ 左相机标定失败，RMS误差过大: {:.4}", error);
            return;
        }
    };
    
    // Step 3: 右相机标定
    println!("📷 Step 4: 右相机单目标定...");
    let right_result = match calibrator.calibrate_mono(&right_obj_points, &right_img_points) {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 右相机标定失败: {}", e);
            return;
        }
    };
    
    let (right_camera, right_error) = match right_result {
        MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
            println!("✅ 右相机标定成功，RMS误差: {:.4}", error);
            (MonoCamera { camera_matrix, dist_coeffs }, error)
        },
        MonoCalibResult::NeedRecalibration(error) => {
            println!("❌ 右相机标定失败，RMS误差过大: {:.4}", error);
            return;
        }
    };
    
    // Step 4: 双目标定
    println!("👁️‍🗨️ Step 5: 双目标定...");
    let stereo_result = match calibrator.calibrate_stereo(
        &left_obj_points, &left_img_points, &right_img_points,
        &left_camera, &right_camera
    ) {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 双目标定失败: {}", e);
            return;
        }
    };
    
    let (r, t, stereo_error) = match stereo_result {
        StereoCalibResult::Success { r, t, error } => {
            println!("✅ 双目标定成功，RMS误差: {:.4}", error);
            (r, t, error)
        },
        StereoCalibResult::NeedRecalibration(error) => {
            println!("❌ 双目标定失败，RMS误差过大: {:.4}", error);
            return;
        }
    };
    
    println!("\n🎉 完整标定流程测试成功！");
    println!("📊 标定结果:");
    println!("   左相机RMS误差: {:.4}", left_error);
    println!("   右相机RMS误差: {:.4}", right_error);
    println!("   双目RMS误差: {:.4}", stereo_error);
    
    // 可选：保存标定结果
    println!("\n💾 标定参数已计算完成，可以保存到yaml文件");
} 