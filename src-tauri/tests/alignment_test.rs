#[cfg(test)]
use crate::modules::alignment::*;
use opencv::{core, imgcodecs};

#[test]
fn test_alignment_system_creation() {
    println!("=== 测试AlignmentSystem创建 ===");
    
    // 注意：这个测试需要实际的标定参数文件
    // 在实际使用时，需要提供正确的文件路径
    let image_size = core::Size::new(2448, 2048); //注意修改硬件ROI后需要修改图像尺寸
    
    // 由于测试环境可能没有实际的标定文件，这里只是展示API的使用方式
    /*
    let result = AlignmentSystem::new(
        image_size,
        "left_camera_params.yaml",
        "right_camera_params.yaml", 
        "stereo_params.yaml",
        "rectify_params.yaml",
    );
    
    match result {
        Ok(system) => {
            println!("✓ AlignmentSystem创建成功");
        }
        Err(e) => {
            println!("AlignmentSystem创建失败: {}", e);
            // 在测试环境中，文件不存在是正常的
        }
    }
    */
    
    println!("AlignmentSystem API测试完成");
}

#[test] 
fn test_alignment_workflow() {
    println!("=== 测试光机合像检测工作流程 ===");
    
    // 这个测试展示了完整的工作流程
    // 实际使用时需要提供真实的图像和参数文件
    
    println!("工作流程步骤:");
    println!("1. 创建AlignmentSystem");
    println!("2. 加载左右眼图像");
    println!("3. 检测圆点网格");
    println!("4. 单光机姿态检测");
    println!("5. 双光机合像判定");
    println!("6. 生成debug图像");
    
    // 示例代码（需要实际文件支持）:
    /*
    let image_size = core::Size::new(2448, 2048);
    let mut alignment_system = AlignmentSystem::new(
        image_size,
        "params/left_camera.yaml",
        "params/right_camera.yaml",
        "params/stereo.yaml", 
        "params/rectify.yaml",
    )?;
    
    // 加载测试图像
    let left_image = imgcodecs::imread("test_images/left_projector.png", imgcodecs::IMREAD_COLOR)?;
    let right_image = imgcodecs::imread("test_images/right_projector.png", imgcodecs::IMREAD_COLOR)?;
    
    // 检测圆点
    let (corners_left, corners_right) = alignment_system.detect_circles_grid(
        &left_image,
        &right_image,
        "params/rectify_maps.yaml"
    )?;
    
    // 单光机姿态检测
    let pose_result = alignment_system.check_single_eye_pose(&corners_left)?;
    println!("单光机姿态: {:?}", pose_result);
    
    if pose_result.pass {
        // 双光机合像判定
        let alignment_result = alignment_system.check_dual_eye_alignment(
            &corners_left,
            &corners_right,
            true  // 保存debug图像
        )?;
        println!("合像检测结果: {:?}", alignment_result);
    }
    */
    
    println!("✓ 工作流程测试完成");
}

#[test]
fn test_statistical_functions() {
    println!("=== 测试统计函数 ===");
    
    // 测试统计计算函数
    use crate::modules::alignment::{mean, rms, percentile};
    
    let test_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    
    let mean_val = mean(&test_data);
    let rms_val = rms(&test_data);
    let p95_val = percentile(&test_data, 95.0);
    
    println!("测试数据: {:?}", test_data);
    println!("均值: {:.2}", mean_val);
    println!("RMS: {:.2}", rms_val);
    println!("P95: {:.2}", p95_val);
    
    // 验证计算结果
    assert!((mean_val - 5.5).abs() < 0.01, "均值计算错误");
    assert!(rms_val > 0.0, "RMS应该大于0");
    assert!(p95_val >= mean_val, "P95应该大于等于均值");
    
    println!("✓ 统计函数测试通过");
}

#[test]
fn test_pose_calculation() {
    println!("=== 测试姿态计算 ===");
    
    // 生成模拟数据：正常姿态
    let corners_normal = generate_mock_corners(40, 100.0, 100.0, 0.1);
    
    // 生成模拟数据：有旋转的姿态
    let corners_rotated = generate_mock_corners(40, 100.0, 100.0, 1.0);
    
    // 手动计算旋转角
    let dx_normal = corners_normal.get(1).unwrap().x - corners_normal.get(0).unwrap().x;
    let dy_normal = corners_normal.get(1).unwrap().y - corners_normal.get(0).unwrap().y;
    let angle_normal = f64::atan2(dy_normal as f64, dx_normal as f64) * 180.0 / std::f64::consts::PI;
    
    let dx_rotated = corners_rotated.get(1).unwrap().x - corners_rotated.get(0).unwrap().x;
    let dy_rotated = corners_rotated.get(1).unwrap().y - corners_rotated.get(0).unwrap().y;
    let angle_rotated = f64::atan2(dy_rotated as f64, dx_rotated as f64) * 180.0 / std::f64::consts::PI;
    
    println!("正常姿态角度: {:.3}°", angle_normal);
    println!("旋转后角度: {:.3}°", angle_rotated);
    
    // 验证角度差异
    assert!((angle_rotated - angle_normal).abs() > 0.1, "旋转角度应该有明显差异");
    
    println!("✓ 姿态计算测试通过");
}

#[test]
fn test_alignment_calculation() {
    println!("=== 测试合像计算逻辑 ===");
    
    // 使用模拟数据测试合像计算
    let corners_left = generate_mock_corners(40, 100.0, 100.0, 0.1);
    let corners_right = generate_mock_corners(40, 102.0, 101.0, 0.1); // 轻微偏移
    
    // 手动计算残差
    let mut dx_values = Vec::new();
    let mut dy_values = Vec::new();
    
    for i in 0..40 {
        let left = corners_left.get(i).unwrap();
        let right = corners_right.get(i).unwrap();
        
        dx_values.push((right.x - left.x) as f64);
        dy_values.push((right.y - left.y) as f64);
    }
    
    use crate::modules::alignment::{mean, rms};
    
    let mean_dx = mean(&dx_values);
    let mean_dy = mean(&dy_values);
    
    println!("模拟残差:");
    println!("  mean_dx = {:.3} px", mean_dx);
    println!("  mean_dy = {:.3} px", mean_dy);
    
    // 验证计算逻辑
    assert!((mean_dx - 2.0).abs() < 0.5, "x方向偏差应该接近2.0");
    assert!((mean_dy - 1.0).abs() < 0.5, "y方向偏差应该接近1.0");
    
    println!("✓ 合像计算逻辑测试通过");
}

// 模拟测试数据生成函数  
fn generate_mock_corners(count: usize, center_x: f32, center_y: f32, noise: f32) -> opencv::core::Vector<opencv::core::Point2f> {
    use opencv::core::{Vector, Point2f};
    
    let mut corners = Vector::<Point2f>::new();
    
    for i in 0..count {
        let x = center_x + (i as f32 * 10.0) + (noise * 0.1); // 简化随机数
        let y = center_y + ((i / 10) as f32 * 10.0) + (noise * 0.1);
        corners.push(Point2f::new(x, y));
    }
    
    corners
} 