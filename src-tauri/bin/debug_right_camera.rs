// 诊断右相机标定失败问题
// 检查右相机图像质量、检测精度和可能的相机移动

use merging_image_lib::modules::calibration_circles::{Calibrator, CameraType};
use opencv::core::Size;
use opencv::prelude::MatTraitConst;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 右相机标定失败诊断");
    println!("=================================");

    // 创建标定器
    let mut calibrator = Calibrator::new(
        Size::new(2448, 2048),
        15.0,
        25.0,
        Size::new(4, 10),
        1.0,
    )?;

    let test_folder = r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755064325";
    
    // 检查右相机图像
    println!("📊 检查右相机图像质量:");
    check_right_camera_images(&mut calibrator, test_folder)?;
    
    // 对比左右相机检测结果
    println!("\n📊 对比左右相机检测结果:");
    compare_left_right_detection(&mut calibrator, test_folder)?;

    Ok(())
}

fn check_right_camera_images(calibrator: &mut Calibrator, folder: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut success_count = 0;
    let mut total_count = 0;

    for i in 1..=10 {
        let right_path = format!("{}\\calib_right_{:02}.png", folder, i);
        
        if !std::path::Path::new(&right_path).exists() {
            continue;
        }

        total_count += 1;
        let filename = format!("calib_right_{:02}.png", i);
        print!("📷 检查 {}: ", filename);

        match opencv::imgcodecs::imread(&right_path, opencv::imgcodecs::IMREAD_COLOR) {
            Ok(image) => {
                if image.empty() {
                    println!("❌ 图像为空");
                    continue;
                }

                // 检查图像基本信息
                let size_info = format!("{}x{}", image.cols(), image.rows());
                
                // 检测圆点
                match calibrator.quick_detect_calibration_pattern(&image) {
                    true => {
                        println!("✅ 成功 ({})", size_info);
                        success_count += 1;
                    }
                    false => {
                        println!("❌ 检测失败 ({})", size_info);
                    }
                }
            }
            Err(e) => {
                println!("❌ 读取错误: {}", e);
            }
        }
    }

    println!("📊 右相机图像检测结果: {}/{} ({:.1}%)", 
             success_count, total_count, 
             (success_count as f32 / total_count as f32) * 100.0);

    if success_count < total_count {
        println!("⚠️  右相机图像存在检测问题，这可能是标定失败的原因");
    }

    Ok(())
}

fn compare_left_right_detection(calibrator: &mut Calibrator, folder: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 检查前3张图像的详细检测结果
    for i in 1..=3 {
        let left_path = format!("{}\\calib_left_{:02}.png", folder, i);
        let right_path = format!("{}\\calib_right_{:02}.png", folder, i);
        
        if !std::path::Path::new(&left_path).exists() || !std::path::Path::new(&right_path).exists() {
            continue;
        }

        println!("\n🔍 详细检查第{}组图像:", i);
        
        // 检查左图
        match opencv::imgcodecs::imread(&left_path, opencv::imgcodecs::IMREAD_COLOR) {
            Ok(left_img) => {
                let left_result = calibrator.quick_detect_calibration_pattern(&left_img);
                println!("   左图: {} ({}x{})", 
                        if left_result { "✅ 成功" } else { "❌ 失败" },
                        left_img.cols(), left_img.rows());
            }
            Err(_) => println!("   左图: ❌ 读取失败"),
        }

        // 检查右图  
        match opencv::imgcodecs::imread(&right_path, opencv::imgcodecs::IMREAD_COLOR) {
            Ok(right_img) => {
                let right_result = calibrator.quick_detect_calibration_pattern(&right_img);
                println!("   右图: {} ({}x{})", 
                        if right_result { "✅ 成功" } else { "❌ 失败" },
                        right_img.cols(), right_img.rows());
            }
            Err(_) => println!("   右图: ❌ 读取失败"),
        }
    }

    println!("\n💡 诊断建议:");
    println!("1. 如果右相机图像检测成功但标定失败 → 可能是相机移动导致");
    println!("2. 如果右相机图像检测失败 → 需要重新采集右相机图像");
    println!("3. 检查相机固定装置是否松动");
    println!("4. 考虑重新采集整套标定图像");

    Ok(())
} 