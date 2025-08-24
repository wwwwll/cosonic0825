use opencv::{
    core::{FileStorage, FileStorage_READ, Mat, Size},
    imgcodecs,
    imgproc,
    prelude::*,
};
use std::path::Path;

use crate::modules::merging_check::{MergingChecker, MergingThreshold};
use crate::modules::param_io::*;

const TAG_SIZE: f64 = 24.0;  // mm

#[test]
fn test_merging_check() -> Result<(), Box<dyn std::error::Error>> {
    // 从标定结果中获取相机参数
    let (camera_matrix, baseline) = load_merging_params()?;

    // 创建合像检测器实例
    let checker = MergingChecker::new(
        camera_matrix,
        (1123, 794),  // 使用元组而不是Size
        baseline,
        TAG_SIZE,
        None,  // 使用默认阈值
    )?;

    // 测试基本功能
    test_basic_detection(&checker)?;

    // 测试已知偏移
    test_known_offset(&checker)?;

    Ok(())
}

fn test_basic_detection(checker: &MergingChecker) -> Result<(), Box<dyn std::error::Error>> {
    // 读取测试图像（已经过校正的图像）
    let left_image = load_test_image("left")?;
    let right_image = load_test_image("right")?;

    // 检测角点
    let left_corners = checker.detect_corners(&left_image)?;
    let right_corners = checker.detect_corners(&right_image)?;

    // 确保检测到角点
    assert!(!left_corners.is_empty());
    assert!(!right_corners.is_empty());

    // 单光机判定
    let (single_ok, single_rot) = checker.check_single_projector(&left_corners)?;
    println!("Single projector rotation: {:.3}°", single_rot);

    // 双光机合像判定
    let result = checker.check_stereo_fusion(&left_corners, &right_corners)?;
    println!("Fusion result: {:#?}", result);

    Ok(())
}

fn test_known_offset(checker: &MergingChecker) -> Result<(), Box<dyn std::error::Error>> {
    // 读取基准图像
    let base_image = load_test_image("left")?;
    
    // 创建已知偏移的图像
    let mut offset_image = base_image.clone();
    
    // 平移5像素
    let matrix = Mat::from_slice_2d(&[
        [1.0, 0.0, 5.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0f64],
    ])?;
    
    imgproc::warp_affine(
        &base_image,
        &mut offset_image,
        &matrix,
        base_image.size()?,
        imgproc::INTER_LINEAR,
        opencv::core::BORDER_CONSTANT,
        opencv::core::Scalar::default(),
    )?;

    // 检测角点
    let base_corners = checker.detect_corners(&base_image)?;
    let offset_corners = checker.detect_corners(&offset_image)?;

    // 合像判定
    let result = checker.check_stereo_fusion(&base_corners, &offset_corners)?;
    
    // 验证偏移检测结果
    assert!(result.dx_mm > 0.0);  // 应该检测到正向x偏移
    assert!(result.dy_mm.abs() < 0.1);  // y方向应该几乎没有偏移
    
    println!("Offset test result: {:#?}", result);

    Ok(())
}

fn load_merging_params() -> Result<(Mat, f64), Box<dyn std::error::Error>> {
    let params_dir = Path::new("src-tauri/src/tests/data/params");
    
    // 加载左相机参数
    let left_params = load_camera_params(params_dir.join("left_camera.yaml"))?;
    let camera_matrix = vec2d_to_mat_f64(&left_params.camera_matrix)?;
    
    // 加载双目参数获取基线
    let stereo_params = load_stereo_params(params_dir.join("stereo.yaml"))?;
    let t = vec_to_mat_f64(&stereo_params.t)?;
    let baseline = *t.at::<f64>(0)?;  // 取x方向分量作为基线

    Ok((camera_matrix, baseline))
}

fn load_test_image(name: &str) -> Result<Mat, Box<dyn std::error::Error>> {
    let test_data_dir = Path::new("src-tauri/src/tests/data");
    let img_path = test_data_dir.join(format!("merging_check_{}.jpg", name));
    
    let img = imgcodecs::imread(
        img_path.to_str().unwrap(),
        imgcodecs::IMREAD_COLOR,
    )?;
    
    if img.empty() {
        return Err(format!("Failed to load image: {}", img_path.display()).into());
    }
    
    Ok(img)
} 