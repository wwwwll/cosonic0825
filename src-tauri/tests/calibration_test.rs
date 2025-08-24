// 限定使用 opencv 4.10.0
// 仅支持格式为 tag35h11 的 AprilTag Grid
// tag顺序已写死，见calibration.rs中的ids_mat

use opencv::{
    core::{FileStorage, FileStorage_WRITE, FileStorage_READ, FileNode, Mat, Point2f, Point3f, Size, Vector},
    imgcodecs,
    prelude::*,
};
use std::fs;
use std::path::Path;

use crate::modules::{
    calibration::{Calibrator, MonoCamera, MonoCalibResult, StereoCalibResult},
    rectification::Rectifier,
    param_io::*,
};

// 根据Test.md更新测试参数
const TAG_SIZE: f64 = 26.0;  // mm
const TAG_SPACING: f64 = 16.0;  // mm
const GRID_ROWS: i32 = 5;
const GRID_COLS: i32 = 7;
const IMAGE_WIDTH: i32 = 2448;  // TODO: 需要确认
const IMAGE_HEIGHT: i32 = 2048;  // TODO: 需要确认

#[test]
fn test_calibration_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    // 创建标定器实例
    let calibrator = Calibrator::new(
        Size::new(IMAGE_WIDTH, IMAGE_HEIGHT),
        TAG_SIZE as f32,        // 输入mm，内部会转换为meters
        TAG_SPACING as f32,     // 输入mm，内部会转换为meters
        Size::new(GRID_COLS, GRID_ROWS),  // 注意顺序：cols, rows
        1.0,  // 重投影误差阈值
    )?;

    // 读取标定图像
    let left_images = load_calibration_images("left")?;
    let right_images = load_calibration_images("right")?;

    // 检测角点
    let mut left_all_corners = Vector::<Vector<Point2f>>::new();  // 明确指定类型
    let mut right_all_corners = Vector::<Vector<Point2f>>::new();
    let mut left_all_ids = Vector::<i32>::new();
    let mut right_all_ids = Vector::<i32>::new();
    let mut counter = Vector::<i32>::new();
    
    println!("\n开始处理标定图像...");
    for (idx, (left_img, right_img)) in left_images.iter().zip(right_images.iter()).enumerate() {
        let (left_corners, left_ids) = calibrator.detect_corners(left_img)?;
        let (right_corners, right_ids) = calibrator.detect_corners(right_img)?;
        println!("Frame {}: 检测到的AprilTag数量 -> 左图: {}, 右图: {}", 
                 idx, left_corners.len(), right_corners.len());

        // 将Mat格式的ids转换为Vector<i32>
        let mut left_ids_vec = Vector::<i32>::new();
        let mut right_ids_vec = Vector::<i32>::new();
        for i in 0..left_ids.rows() {
            left_ids_vec.push(*left_ids.at::<i32>(i)?);
        }
        for i in 0..right_ids.rows() {
            right_ids_vec.push(*right_ids.at::<i32>(i)?);
        }

        // 添加到all_corners和all_ids中
        left_all_corners.push(left_corners);
        right_all_corners.push(right_corners);
        for id in left_ids_vec.iter() {
            left_all_ids.push(id.clone());  // 使用clone()而不是解引用
        }
        for id in right_ids_vec.iter() {
            right_all_ids.push(id.clone());  // 使用clone()而不是解引用
        }
        counter.push(left_corners.len() as i32);

        // 打印一些调试信息
        if idx == 0 {
            for i in 0..4 {  // 只打印第一个tag的四个角点
                let left_corner = left_corners.get(0)?.get(i)?;
                println!("    左图角点 [0,{}] = ({:.1}, {:.1}) px", i, left_corner.x, left_corner.y);
            }
        }

        println!("    Frame {} 处理完成: {} 左图点, {} 右图点", 
            idx, 
            left_corners.len() * 4,  // 每个tag有4个角点
            right_corners.len() * 4
        );
    }

    // 在循环结束后，使用累积的数据调用一次match_image_points_multi_frame
    let (obj_points_list, left_corners_list, right_corners_list) = 
        calibrator.match_image_points_multi_frame(&left_all_corners, &left_all_ids, &right_all_corners, &right_all_ids)?;

    println!("\n标定数据汇总:");
    println!("- 有效帧数: {}", counter.len());
    println!("- 总角点数: {} (左), {} (右)", 
        left_all_corners.len() * 4,  // 每个tag有4个角点
        right_all_corners.len() * 4
    );
    
    if counter.is_empty() {
        return Err("没有足够的有效标定帧".into());
    }

    // 单目标定
    let left_calib = match calibrator.calibrate_mono(&left_all_corners, &left_all_ids, &counter)? {
        MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
            println!("Left camera calibration error: {}", error);
            Some((camera_matrix, dist_coeffs))
        }
        MonoCalibResult::NeedRecalibration(error) => {
            println!("Left camera needs recalibration, error: {}", error);
            None
        }
    };

    let right_calib = match calibrator.calibrate_mono(&right_all_corners, &right_all_ids, &counter)? {
        MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
            println!("Right camera calibration error: {}", error);
            Some((camera_matrix, dist_coeffs))
        }
        MonoCalibResult::NeedRecalibration(error) => {
            println!("Right camera needs recalibration, error: {}", error);
            None
        }
    };

    // 确保单目标定成功
    let (left_camera_matrix, left_dist_coeffs) = left_calib.ok_or("Left camera calibration failed")?;
    let (right_camera_matrix, right_dist_coeffs) = right_calib.ok_or("Right camera calibration failed")?;

    let left_camera = MonoCamera {
        camera_matrix: left_camera_matrix.clone(),
        dist_coeffs: left_dist_coeffs.clone(),
    };
    let right_camera = MonoCamera {
        camera_matrix: right_camera_matrix.clone(),
        dist_coeffs: right_dist_coeffs.clone(),
    };

    // 生成左右相机的obj/img点用于双目标定
    let mut obj_points_list = Vector::new();
    let mut left_corners_list = Vector::new();
    let mut right_corners_list = Vector::new();

    for (idx, (left_img, right_img)) in left_images.iter().zip(right_images.iter()).enumerate() {
        let (left_corners, left_ids) = calibrator.detect_corners(left_img)?;
        let (right_corners, right_ids) = calibrator.detect_corners(right_img)?;

        let (frame_obj_points, frame_corners_left, frame_corners_right) = 
            calibrator.match_image_points_multi_frame(&left_corners, &left_ids, &right_corners, &right_ids)?;

        obj_points_list.push(frame_obj_points);
        left_corners_list.push(frame_corners_left);
        right_corners_list.push(frame_corners_right);
    }

    // 双目标定
    let stereo_calib = match calibrator.calibrate_stereo(
        &obj_points_list,
        &left_corners_list,
        &right_corners_list,
        &left_camera,
        &right_camera,
    )? {
        StereoCalibResult::Success { r, t, error } => {
            println!("Stereo calibration error: {}", error);
            Some((r, t))
        }
        StereoCalibResult::NeedRecalibration(error) => {
            println!("Stereo calibration needs recalibration, error: {}", error);
            None
        }
    };

    // 确保双目标定成功
    let (r, t) = stereo_calib.ok_or("Stereo calibration failed")?;

    // 计算立体校正映射
    let rectify_maps = calibrator.compute_stereo_rectify(
        &left_camera,
        &right_camera,
        &r,
        &t,
    )?;

    // 计算重映射矩阵
    let (left_map1, left_map2) = calibrator.compute_undistort_maps(
        &left_camera_matrix,
        &left_dist_coeffs,
        &rectify_maps.r1,
        &rectify_maps.p1,
    )?;

    let (right_map1, right_map2) = calibrator.compute_undistort_maps(
        &right_camera_matrix,
        &right_dist_coeffs,
        &rectify_maps.r2,
        &rectify_maps.p2,
    )?;

    // 验证参数合理性
    let fx_left = *left_camera_matrix.at::<f64>(0)?;
    let fx_right = *right_camera_matrix.at::<f64>(0)?;
    let tx = *t.at::<f64>(0)?;
    
    assert!(fx_left > 0.0);  // fx应该为正
    assert!(fx_right > 0.0);
    assert!(tx > 0.0);  // 基线应该为正

    // 保存标定结果
    let params_dir = Path::new("D:/rust_projects/merging_image/src-tauri/src/tests/data/params");
    fs::create_dir_all(params_dir)?;

    // 保存左相机参数
    let left_camera_params = CameraParams {
        camera_matrix: mat_to_vec2d_f64(&left_camera_matrix),
        dist_coeffs: mat_to_vec_f64(&left_dist_coeffs),
    };
    save_camera_params(params_dir.join("left_camera.yaml"), &left_camera_params)?;

    // 保存右相机参数
    let right_camera_params = CameraParams {
        camera_matrix: mat_to_vec2d_f64(&right_camera_matrix),
        dist_coeffs: mat_to_vec_f64(&right_dist_coeffs),
    };
    save_camera_params(params_dir.join("right_camera.yaml"), &right_camera_params)?;

    // 保存双目参数
    let stereo_params = StereoParams {
        r: mat_to_vec2d_f64(&r),
        t: mat_to_vec_f64(&t),
    };
    save_stereo_params(params_dir.join("stereo.yaml"), &stereo_params)?;

    // 保存校正参数
    let rectify_params = RectifyParams {
        r1: mat_to_vec2d_f64(&rectify_maps.r1),
        r2: mat_to_vec2d_f64(&rectify_maps.r2),
        p1: mat_to_vec2d_f64(&rectify_maps.p1),
        p2: mat_to_vec2d_f64(&rectify_maps.p2),
        q: mat_to_vec2d_f64(&rectify_maps.q),
    };
    save_rectify_params(params_dir.join("rectify.yaml"), &rectify_params)?;

    // 保存重映射矩阵
    let maps = RectifyMaps {
        left_map1: mat_to_vec2d_f32(&left_map1),
        left_map2: mat_to_vec2d_f32(&left_map2),
        right_map1: mat_to_vec2d_f32(&right_map1),
        right_map2: mat_to_vec2d_f32(&right_map2),
    };
    save_rectify_maps(params_dir.join("maps.yaml"), &maps)?;

    Ok(())
}

fn generate_object_points_single_frame() -> Result<Vector<Point3f>, Box<dyn std::error::Error>> {
    let mut points = Vector::new();
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLS {
            let x = col as f32 * (TAG_SIZE + TAG_SPACING) as f32;
            let y = row as f32 * (TAG_SIZE + TAG_SPACING) as f32;
            points.push(Point3f::new(x, y, 0.0));
        }
    }
    Ok(points)
}

fn load_calibration_images(prefix: &str) -> Result<Vec<Mat>, Box<dyn std::error::Error>> {
    let mut images = Vec::new();
    let test_data_dir = Path::new("D:/rust_projects/merging_image/src-tauri/src/tests/data");
    
    // 加载5组标定图像（编号0-4）
    for i in 0..5 {
        let img_path = test_data_dir.join(format!("{}_{}.bmp", prefix, i));
        let img = imgcodecs::imread(
            img_path.to_str().unwrap(),
            imgcodecs::IMREAD_COLOR,
        )?;
        if img.empty() {
            return Err(format!("Failed to load image: {}", img_path.display()).into());
        }
        images.push(img);
    }
    
    Ok(images)
}

/*
fn save_calibration_results(
    left_camera_matrix: &Mat,
    left_dist_coeffs: &Mat,
    right_camera_matrix: &Mat,
    right_dist_coeffs: &Mat,
    r: &Mat,
    t: &Mat,
    left_map1: &Mat,
    left_map2: &Mat,
    right_map1: &Mat,
    right_map2: &Mat,
) -> Result<(), Box<dyn std::error::Error>> {
    let params_dir = Path::new("src-tauri/src/tests/data/params");
    fs::create_dir_all(params_dir)?;

    // 保存相机参数
    let mut fs = FileStorage::new(
        params_dir.join("camera_params.xml").to_str().unwrap(),
        FileStorage_WRITE,
        "",
    )?;
    
    fs.write("left_camera_matrix", left_camera_matrix)?;
    fs.write("left_dist_coeffs", left_dist_coeffs)?;
    fs.write("right_camera_matrix", right_camera_matrix)?;
    fs.write("right_dist_coeffs", right_dist_coeffs)?;
    fs.write("r", r)?;
    fs.write("t", t)?;
    fs.release()?;

    // 保存重映射矩阵
    let mut fs = FileStorage::new(
        params_dir.join("rectify_maps.xml").to_str().unwrap(),
        FileStorage_WRITE,
        "",
    )?;
    
    fs.write("left_map1", left_map1)?;
    fs.write("left_map2", left_map2)?;
    fs.write("right_map1", right_map1)?;
    fs.write("right_map2", right_map2)?;
    fs.release()?;

    Ok(())
}
*/