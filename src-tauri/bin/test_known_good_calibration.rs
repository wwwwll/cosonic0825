// 使用已知良好的标定图像测试我们的标定算法
// 验证算法本身是否正常工作

use merging_image_lib::modules::calibration_circles::{Calibrator, CameraType};
use opencv::core::Size;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试已知良好标定图像");
    println!("=====================================");
    println!("📁 使用图像: C:\\Users\\Y000010\\MVS\\Data\\point_5_4");
    println!("🎯 目标: 验证标定算法是否能重现低重投影误差");
    println!("=====================================\n");

    // 测试1: 使用BMP文件
    println!("📋 测试1: 使用原始BMP文件");
    test_calibration_with_bmps()?;

    // 测试2: 使用PNG文件
    println!("\n📋 测试2: 使用转换后的PNG文件");
    test_calibration_with_pngs()?;

    Ok(())
}

fn test_calibration_with_bmps() -> Result<(), Box<dyn std::error::Error>> {
    let bmp_folder = r"C:\Users\Y000010\MVS\Data\point_5_4";
    
    // 创建标定器 - 使用与workflow相同的参数
    let mut calibrator = Calibrator::new(
        Size::new(2448, 2048),  // 假设相同的图像尺寸
        15.0,                   // 圆点直径
        25.0,                   // 圆点间距
        Size::new(4, 10),       // 标定板尺寸
        1.0,                    // 重投影误差阈值
    )?;

    // 构建BMP文件路径 (l_0.bmp到l_8.bmp)
    let mut left_paths = Vec::new();
    let mut right_paths = Vec::new();
    
    for i in 0..9 {
        let left_path = format!("{}\\l_{}.bmp", bmp_folder, i);
        let right_path = format!("{}\\r_{}.bmp", bmp_folder, i);
        
        if std::path::Path::new(&left_path).exists() && std::path::Path::new(&right_path).exists() {
            left_paths.push(left_path);
            right_paths.push(right_path);
        }
    }

    println!("📊 找到 {} 组BMP图像", left_paths.len());
    
    if left_paths.len() < 5 {
        println!("⚠️  BMP图像数量不足，跳过测试");
        return Ok(());
    }

    // 执行标定
    run_full_calibration_test(&mut calibrator, &left_paths, &right_paths, "BMP")?;
    
    Ok(())
}

fn test_calibration_with_pngs() -> Result<(), Box<dyn std::error::Error>> {
    let png_folder = r"C:\Users\Y000010\MVS\Data\point_5_4\png";
    
    // 创建标定器
    let mut calibrator = Calibrator::new(
        Size::new(2448, 2048),
        15.0,
        25.0,
        Size::new(4, 10),
        1.0,
    )?;

    // 构建PNG文件路径 (l_0.png到l_8.png)
    let mut left_paths = Vec::new();
    let mut right_paths = Vec::new();
    
    for i in 0..9 {
        let left_path = format!("{}\\l_{}.png", png_folder, i);
        let right_path = format!("{}\\r_{}.png", png_folder, i);
        
        if std::path::Path::new(&left_path).exists() && std::path::Path::new(&right_path).exists() {
            left_paths.push(left_path);
            right_paths.push(right_path);
        }
    }

    println!("📊 找到 {} 组PNG图像", left_paths.len());
    
    if left_paths.len() < 5 {
        println!("⚠️  PNG图像数量不足，跳过测试");
        return Ok(());
    }

    // 执行标定
    run_full_calibration_test(&mut calibrator, &left_paths, &right_paths, "PNG")?;
    
    Ok(())
}

fn run_full_calibration_test(
    calibrator: &mut Calibrator,
    left_paths: &[String],
    right_paths: &[String],
    format_name: &str
) -> Result<(), Box<dyn std::error::Error>> {
    
    println!("🚀 开始{}格式完整标定测试", format_name);

    // Step 1: 左相机点检测
    println!("📷 Step 1: 左相机点检测...");
    let (left_obj_points, left_img_points) = calibrator.detect_and_get_points_from_paths(left_paths, CameraType::Left)?;
    println!("✅ 左相机点检测成功: {} 组图像", left_obj_points.len());

    // Step 2: 右相机点检测
    println!("📷 Step 2: 右相机点检测...");
    let (right_obj_points, right_img_points) = calibrator.detect_and_get_points_from_paths(right_paths, CameraType::Right)?;
    println!("✅ 右相机点检测成功: {} 组图像", right_obj_points.len());

    // Step 3: 左相机单目标定
    println!("📷 Step 3: 左相机单目标定...");
    match calibrator.calibrate_mono(&left_obj_points, &left_img_points) {
        Ok(left_result) => {
            match left_result {
                merging_image_lib::modules::calibration_circles::MonoCalibResult::Success { camera_matrix: _, dist_coeffs: _, error } => {
                    println!("✅ 左相机标定成功，RMS误差: {:.4}", error);
                }
                merging_image_lib::modules::calibration_circles::MonoCalibResult::NeedRecalibration(error) => {
                    println!("❌ 左相机标定失败，RMS误差: {:.4}", error);
                    return Ok(());
                }
            }
        }
        Err(e) => {
            println!("❌ 左相机标定错误: {}", e);
            return Ok(());
        }
    }

    // Step 4: 右相机单目标定
    println!("📷 Step 4: 右相机单目标定...");
    match calibrator.calibrate_mono(&right_obj_points, &right_img_points) {
        Ok(right_result) => {
            match right_result {
                merging_image_lib::modules::calibration_circles::MonoCalibResult::Success { camera_matrix: _, dist_coeffs: _, error } => {
                    println!("✅ 右相机标定成功，RMS误差: {:.4}", error);
                    println!("🎉 {}格式标定测试完成！", format_name);
                }
                merging_image_lib::modules::calibration_circles::MonoCalibResult::NeedRecalibration(error) => {
                    println!("❌ 右相机标定失败，RMS误差: {:.4}", error);
                }
            }
        }
        Err(e) => {
            println!("❌ 右相机标定错误: {}", e);
        }
    }

    Ok(())
} 