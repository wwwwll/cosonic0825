use opencv::{
    core::{Mat, Size},
    imgcodecs,
    prelude::*,
};
use std::path::Path;
use crate::modules::{
    rectification::Rectifier,
    param_io::*,
};

#[test]
fn test_rectification() -> Result<(), Box<dyn std::error::Error>> {
    // 创建校正器实例
    let rectifier = Rectifier::new(Size::new(2880, 2160))?;

    // 读取测试图像
    let left_image = load_test_image("left")?;
    let right_image = load_test_image("right")?;

    // 从标定测试中获取重映射矩阵
    let maps = load_rectify_maps(Path::new("D:/rust_projects/merging_image/src-tauri/src/tests/data/params/maps.yaml"))?;
    let left_map1 = vec2d_to_mat_f32(&maps.left_map1)?;
    let left_map2 = vec2d_to_mat_f32(&maps.left_map2)?;
    let right_map1 = vec2d_to_mat_f32(&maps.right_map1)?;
    let right_map2 = vec2d_to_mat_f32(&maps.right_map2)?;

    // 应用重映射
    let left_rectified = rectifier.remap_image(&left_image, &left_map1, &left_map2)?;
    let right_rectified = rectifier.remap_image(&right_image, &right_map1, &right_map2)?;

    // 验证校正结果
    assert!(!left_rectified.empty());
    assert!(!right_rectified.empty());
    assert_eq!(left_rectified.size()?, right_rectified.size()?);

    Ok(())
}

fn load_test_image(prefix: &str) -> Result<Mat, Box<dyn std::error::Error>> {
    let test_data_dir = Path::new("D:/rust_projects/merging_image/src-tauri/src/tests/data");
    let img_path = test_data_dir.join(format!("calibration_{}_{}.jpg", prefix, 0));  // 使用第一组标定图像
    
    let img = imgcodecs::imread(
        img_path.to_str().unwrap(),
        imgcodecs::IMREAD_COLOR,
    )?;
    
    if img.empty() {
        return Err(format!("Failed to load image: {}", img_path.display()).into());
    }
    
    Ok(img)
} 