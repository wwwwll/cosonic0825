use opencv::core::{Mat, Size};
use opencv::prelude::{MatTrait, MatTraitConst};
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CameraParams {
    pub camera_matrix: Vec<Vec<f64>>,  // 3x3
    pub dist_coeffs: Vec<f64>,         // 1x5
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StereoParams {
    pub r: Vec<Vec<f64>>,  // 3x3 rotation matrix
    pub t: Vec<f64>,       // 3x1 translation vector
    //pub e: Vec<Vec<f64>>,  // 3x3 essential matrix
    //pub f: Vec<Vec<f64>>,  // 3x3 fundamental matrix
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RectifyParams {
    pub r1: Vec<Vec<f64>>,  // 3x3 rectification transform for camera 1
    pub r2: Vec<Vec<f64>>,  // 3x3 rectification transform for camera 2
    pub p1: Vec<Vec<f64>>,  // 3x4 projection matrix for camera 1
    pub p2: Vec<Vec<f64>>,  // 3x4 projection matrix for camera 2
    pub q: Vec<Vec<f64>>,   // 4x4 disparity-to-depth mapping matrix
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RectifyLeftRightMaps {
    pub left_map1: Vec<Vec<f32>>,   // x-mapping for left camera
    pub left_map2: Vec<Vec<f32>>,   // y-mapping for left camera
    pub right_map1: Vec<Vec<f32>>,  // x-mapping for right camera
    pub right_map2: Vec<Vec<f32>>,  // y-mapping for right camera
}

// --- Mat <-> Vec 转换工具 ---
pub fn mat_to_vec2d_f64(mat: &Mat) -> Vec<Vec<f64>> {
    let rows = mat.rows();
    let cols = mat.cols();
    let mut result = vec![vec![0.0; cols as usize]; rows as usize];
    for i in 0..rows {
        for j in 0..cols {
            result[i as usize][j as usize] = *mat.at_2d::<f64>(i, j).unwrap();
        }
    }
    result
}

pub fn mat_to_vec2d_f32(mat: &Mat) -> Vec<Vec<f32>> {
    let rows = mat.rows();
    let cols = mat.cols();
    let mut result = vec![vec![0.0; cols as usize]; rows as usize];
    for i in 0..rows {
        for j in 0..cols {
            result[i as usize][j as usize] = *mat.at_2d::<f32>(i, j).unwrap();
        }
    }
    result
}

pub fn mat_to_vec_f64(mat: &Mat) -> Vec<f64> {
    let total = mat.total() as usize;
    let mut result = vec![0.0; total];
    for i in 0..total {
        result[i] = *mat.at::<f64>(i as i32).unwrap();
    }
    result
}

pub fn vec2d_to_mat_f64(data: &[Vec<f64>]) -> Result<Mat, opencv::Error> {
    let rows = data.len();
    let cols = data[0].len();
    let mut mat = Mat::new_rows_cols_with_default(
        rows as i32, 
        cols as i32,
        opencv::core::CV_64F,
        opencv::core::Scalar::default(),
    )?;
    
    for i in 0..rows {
        for j in 0..cols {
            *mat.at_2d_mut::<f64>(i as i32, j as i32)? = data[i][j];
        }
    }
    Ok(mat)
}

pub fn vec2d_to_mat_f32(data: &[Vec<f32>]) -> Result<Mat, opencv::Error> {
    let rows = data.len();
    let cols = data[0].len();
    let mut mat = Mat::new_rows_cols_with_default(
        rows as i32, 
        cols as i32,
        opencv::core::CV_32F,
        opencv::core::Scalar::default(),
    )?;
    
    for i in 0..rows {
        for j in 0..cols {
            *mat.at_2d_mut::<f32>(i as i32, j as i32)? = data[i][j];
        }
    }
    Ok(mat)
}

pub fn vec_to_mat_f64(data: &[f64]) -> Result<Mat, opencv::Error> {
    let mut mat = Mat::new_rows_cols_with_default(
        data.len() as i32,
        1,
        opencv::core::CV_64F,
        opencv::core::Scalar::default(),
    )?;
    
    for (i, &value) in data.iter().enumerate() {
        *mat.at_mut::<f64>(i as i32)? = value;
    }
    Ok(mat)
}

// --- YAML 保存/加载函数 ---
pub fn save_camera_params<P: AsRef<Path>>(path: P, params: &CameraParams) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(params)?;
    fs::write(path, yaml)?;
    Ok(())
}

pub fn load_camera_params<P: AsRef<Path>>(path: P) -> Result<CameraParams, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(path)?;
    let params = serde_yaml::from_str(&yaml)?;
    Ok(params)
}

pub fn save_stereo_params<P: AsRef<Path>>(path: P, params: &StereoParams) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(params)?;
    fs::write(path, yaml)?;
    Ok(())
}

pub fn load_stereo_params<P: AsRef<Path>>(path: P) -> Result<StereoParams, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(path)?;
    let params = serde_yaml::from_str(&yaml)?;
    Ok(params)
}

pub fn save_rectify_params<P: AsRef<Path>>(path: P, params: &RectifyParams) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(params)?;
    fs::write(path, yaml)?;
    Ok(())
}

pub fn load_rectify_params<P: AsRef<Path>>(path: P) -> Result<RectifyParams, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(path)?;
    let params = serde_yaml::from_str(&yaml)?;
    Ok(params)
}

pub fn save_rectify_maps<P: AsRef<Path>>(path: P, maps: &RectifyLeftRightMaps) -> Result<(), Box<dyn std::error::Error>> {
    let yaml = serde_yaml::to_string(maps)?;
    fs::write(path, yaml)?;
    Ok(())
}

pub fn load_rectify_maps<P: AsRef<Path>>(path: P) -> Result<RectifyLeftRightMaps, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(path)?;
    let maps = serde_yaml::from_str(&yaml)?;
    Ok(maps)
}

// --- 图像文件保存/加载函数 ---

/// 保存图像缓冲区到文件
pub fn save_image_buffer_to_file(buffer: &[u8], file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::write(file_path, buffer)?;
    println!("📷 图像已保存: {}", file_path);
    Ok(())
}

/// 保存双目图像对
pub fn save_stereo_image_buffers(
    left_buffer: &[u8], 
    right_buffer: &[u8], 
    left_path: &str, 
    right_path: &str
) -> Result<(), Box<dyn std::error::Error>> {
    save_image_buffer_to_file(left_buffer, left_path)?;
    save_image_buffer_to_file(right_buffer, right_path)?;
    println!("📷 双目图像对已保存: {} | {}", left_path, right_path);
    Ok(())
}

/// 验证图像文件是否存在和有效
pub fn validate_image_file<P: AsRef<Path>>(path: P) -> Result<bool, Box<dyn std::error::Error>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(false);
    }
    
    // 检查文件大小（简单验证）
    let metadata = fs::metadata(path)?;
    Ok(metadata.len() > 100) // 假设有效图像至少100字节
}

/// 验证双目图像对是否都存在和有效
pub fn validate_stereo_image_pair(left_path: &str, right_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let left_valid = validate_image_file(left_path)?;
    let right_valid = validate_image_file(right_path)?;
    Ok(left_valid && right_valid)
}
