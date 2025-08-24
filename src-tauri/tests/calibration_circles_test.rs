#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::calibration_circles::*;
    use crate::modules::param_io::*;
    use opencv::core::Size;
    use opencv::prelude::*;

    const TEST_IMAGE_FOLDER: &str = r"C:\Users\Y000010\MVS\Data\point_5_4";
    const PATTERN_COLS: i32 = 4;  // 
    const PATTERN_ROWS: i32 = 10; // 
    const CENTER_DISTANCE: f32 = 25.0; // 25mm
    const CIRCLE_DIAMETER: f32 = 15.0; // 15mm
    const ERROR_THRESHOLD: f64 = 1.0; // 重投影误差阈值

    #[test]
    fn test_calibration_circles_full_pipeline() {
        println!("=== 开始测试 Asymmetric Circles Grid 标定流程 ===");
        
        // 步骤1: 创建标定器实例
        println!("\n步骤1: 创建标定器实例");
        let image_size = Size::new(2448, 2048); // 根据实际图像大小调整
        let pattern_size = Size::new(PATTERN_COLS, PATTERN_ROWS);
        
        let mut calibrator = Calibrator::new(
            image_size,
            CIRCLE_DIAMETER,
            CENTER_DISTANCE,
            pattern_size,
            ERROR_THRESHOLD,
        ).expect("Failed to create calibrator");
        
        println!("✓ 标定器创建成功");
        println!("  - 图像尺寸: {}x{}", image_size.width, image_size.height);
        println!("  - 标定板: {}行{}列", pattern_size.height, pattern_size.width);
        println!("  - 圆心距离: {}mm", CENTER_DISTANCE);
        println!("  - 圆点直径: {}mm", CIRCLE_DIAMETER);

        // 步骤2: 测试世界坐标生成
        println!("\n步骤2: 测试世界坐标生成");
        let world_points = calibrator.generate_asymmetric_circle_grid_world_points()
            .expect("Failed to generate world points");
        
        println!("✓ 世界坐标生成成功");
        println!("  - 总点数: {}", world_points.len());
        println!("  - 预期点数: {}", PATTERN_COLS * PATTERN_ROWS);
        assert_eq!(world_points.len(), (PATTERN_COLS * PATTERN_ROWS) as usize);
        
        // 打印前几个点的坐标
        for i in 0..std::cmp::min(5, world_points.len()) {
            let point = world_points.get(i).expect("Failed to get point");
            println!("  - 点{}: ({:.2}, {:.2}, {:.2})", i, point.x, point.y, point.z);
        }

        // 步骤3: 读取左相机图像并检测圆点
        println!("\n步骤3: 读取左相机图像并检测圆点");
        let (left_obj_points, left_img_points) = calibrator
            .get_image_points_and_obj_points_pairs(TEST_IMAGE_FOLDER, CameraType::Left)
            .expect("Failed to process left camera images");
        
        println!("✓ 左相机图像处理完成");
        println!("  - 成功处理图像数: {}", left_img_points.len());
        
        if left_img_points.is_empty() {
            panic!("❌ 左相机未检测到任何有效图像，请检查图像路径和文件名");
        }

        // 步骤4: 读取右相机图像并检测圆点
        println!("\n步骤4: 读取右相机图像并检测圆点");
        let (right_obj_points, right_img_points) = calibrator
            .get_image_points_and_obj_points_pairs(TEST_IMAGE_FOLDER, CameraType::Right)
            .expect("Failed to process right camera images");
        
        println!("✓ 右相机图像处理完成");
        println!("  - 成功处理图像数: {}", right_img_points.len());
        
        if right_img_points.is_empty() {
            panic!("❌ 右相机未检测到任何有效图像，请检查图像路径和文件名");
        }

        // 步骤5: 左相机单目标定
        println!("\n步骤5: 左相机单目标定");
        let left_calib_result = calibrator
            .calibrate_mono(&left_obj_points, &left_img_points)
            .expect("Failed to calibrate left camera");
        
        let (left_camera_matrix, left_dist_coeffs, left_error) = match left_calib_result {
            MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
                println!("✓ 左相机标定成功");
                println!("  - 重投影误差: {:.6}", error);
                (camera_matrix, dist_coeffs, error)
            }
            MonoCalibResult::NeedRecalibration(error) => {
                println!("⚠ 左相机标定精度不足，重投影误差: {:.6}", error);
                panic!("Left camera calibration error too high");
            }
        };

        // 步骤6: 右相机单目标定
        println!("\n步骤6: 右相机单目标定");
        let right_calib_result = calibrator
            .calibrate_mono(&right_obj_points, &right_img_points)
            .expect("Failed to calibrate right camera");
        
        let (right_camera_matrix, right_dist_coeffs, right_error) = match right_calib_result {
            MonoCalibResult::Success { camera_matrix, dist_coeffs, error } => {
                println!("✓ 右相机标定成功");
                println!("  - 重投影误差: {:.6}", error);
                (camera_matrix, dist_coeffs, error)
            }
            MonoCalibResult::NeedRecalibration(error) => {
                println!("⚠ 右相机标定精度不足，重投影误差: {:.6}", error);
                panic!("Right camera calibration error too high");
            }
        };

        // 步骤7: 双目标定
        println!("\n步骤7: 双目标定");
        let left_camera = MonoCamera {
            camera_matrix: left_camera_matrix.clone(),
            dist_coeffs: left_dist_coeffs.clone(),
        };
        let right_camera = MonoCamera {
            camera_matrix: right_camera_matrix.clone(),
            dist_coeffs: right_dist_coeffs.clone(),
        };

        let stereo_result = calibrator
            .calibrate_stereo(
                &left_obj_points, // 使用左相机的obj_points
                &left_img_points,
                &right_img_points,
                &left_camera,
                &right_camera,
            )
            .expect("Failed to perform stereo calibration");

        let (rotation_matrix, translation_vector, stereo_error) = match stereo_result {
            StereoCalibResult::Success { r, t, error } => {
                println!("✓ 双目标定成功");
                println!("  - 重投影误差: {:.6}", error);
                (r, t, error)
            }
            StereoCalibResult::NeedRecalibration(error) => {
                println!("⚠ 双目标定精度不足，重投影误差: {:.6}", error);
                panic!("Stereo calibration error too high");
            }
        };

        // 步骤8: 计算立体校正映射
        println!("\n步骤8: 计算立体校正映射");
        let rectify_maps = calibrator
            .compute_stereo_rectify(
                &left_camera,
                &right_camera,
                &rotation_matrix,
                &translation_vector,
            )
            .expect("Failed to compute stereo rectification");

        println!("✓ 立体校正映射计算成功");

        // 步骤9: 计算左右相机的重映射矩阵
        println!("\n步骤9: 计算重映射矩阵");
        let (left_map1, left_map2) = calibrator
            .compute_undistort_maps(
                &left_camera.camera_matrix,
                &left_camera.dist_coeffs,
                &rectify_maps.r1,
                &rectify_maps.p1,
            )
            .expect("Failed to compute left camera undistort maps");

        let (right_map1, right_map2) = calibrator
            .compute_undistort_maps(
                &right_camera.camera_matrix,
                &right_camera.dist_coeffs,
                &rectify_maps.r2,
                &rectify_maps.p2,
            )
            .expect("Failed to compute right camera undistort maps");

        println!("✓ 重映射矩阵计算成功");

        // 步骤10: 保存标定结果
        println!("\n步骤10: 保存标定结果");
        
        // 保存左相机参数
        let left_params = CameraParams {
            camera_matrix: mat_to_vec2d_f64(&left_camera_matrix),
            dist_coeffs: mat_to_vec_f64(&left_dist_coeffs),
        };
        save_camera_params("left_camera_params.yaml", &left_params)
            .expect("Failed to save left camera parameters");

        // 保存右相机参数
        let right_params = CameraParams {
            camera_matrix: mat_to_vec2d_f64(&right_camera_matrix),
            dist_coeffs: mat_to_vec_f64(&right_dist_coeffs),
        };
        save_camera_params("right_camera_params.yaml", &right_params)
            .expect("Failed to save right camera parameters");

        // 保存双目标定参数
        let stereo_params = StereoParams {
            r: mat_to_vec2d_f64(&rotation_matrix),
            t: mat_to_vec_f64(&translation_vector),
        };
        save_stereo_params("stereo_params.yaml", &stereo_params)
            .expect("Failed to save stereo parameters");

        // 保存校正参数
        let rectify_params = RectifyParams {
            r1: mat_to_vec2d_f64(&rectify_maps.r1),
            r2: mat_to_vec2d_f64(&rectify_maps.r2),
            p1: mat_to_vec2d_f64(&rectify_maps.p1),
            p2: mat_to_vec2d_f64(&rectify_maps.p2),
            q: mat_to_vec2d_f64(&rectify_maps.q),
        };
        save_rectify_params("rectify_params.yaml", &rectify_params)
            .expect("Failed to save rectification parameters");

        // 保存重映射矩阵
        let remap_maps = RectifyLeftRightMaps {
            left_map1: mat_to_vec2d_f32(&left_map1),
            left_map2: mat_to_vec2d_f32(&left_map2),
            right_map1: mat_to_vec2d_f32(&right_map1),
            right_map2: mat_to_vec2d_f32(&right_map2),
        };
        save_rectify_maps("rectify_maps.yaml", &remap_maps)
            .expect("Failed to save rectification maps");

        println!("✓ 标定结果保存成功");
        println!("  - left_camera_params.yaml");
        println!("  - right_camera_params.yaml");
        println!("  - stereo_params.yaml");
        println!("  - rectify_params.yaml");
        println!("  - rectify_maps.yaml");

        // 步骤11: 打印测试总结
        println!("\n=== 测试总结 ===");
        println!("✓ 所有测试步骤都成功完成");
        println!("  - 左相机处理图像数: {}", left_img_points.len());
        println!("  - 右相机处理图像数: {}", right_img_points.len());
        println!("  - 左相机重投影误差: {:.6}", left_error);
        println!("  - 右相机重投影误差: {:.6}", right_error);
        println!("  - 双目标定重投影误差: {:.6}", stereo_error);
        
        // 验证误差是否在可接受范围内
        assert!(left_error <= ERROR_THRESHOLD, "左相机重投影误差过大");
        assert!(right_error <= ERROR_THRESHOLD, "右相机重投影误差过大");
        assert!(stereo_error <= ERROR_THRESHOLD, "双目标定重投影误差过大");
        
        println!("=== 所有测试通过 ✓ ===");
    }

    #[test]
    fn test_single_image_detection() {
        println!("=== 测试单张图像圆点检测 ===");
        
        let image_size = Size::new(1440, 1080);
        let pattern_size = Size::new(PATTERN_COLS, PATTERN_ROWS);
        
        let mut calibrator = Calibrator::new(
            image_size,
            CIRCLE_DIAMETER,
            CENTER_DISTANCE,
            pattern_size,
            ERROR_THRESHOLD,
        ).expect("Failed to create calibrator");

        // 测试读取第一张左相机图像
        let test_image_path = format!(r"{}\l_0.bmp", TEST_IMAGE_FOLDER);
        let img = opencv::imgcodecs::imread(&test_image_path, opencv::imgcodecs::IMREAD_COLOR)
            .expect("Failed to read test image");
        
        if img.empty() {
            panic!("测试图像为空，请检查路径: {}", test_image_path);
        }

        println!("✓ 成功读取测试图像: {}", test_image_path);
        println!("  - 图像尺寸: {}x{}x{}", img.cols(), img.rows(), img.channels());

        // 检测圆点
        match calibrator.find_asymmetric_circles_grid_points(&img, true) {
            Ok(centers) => {
                println!("✓ 圆点检测成功");
                println!("  - 检测到圆点数: {}", centers.len());
                println!("  - 预期圆点数: {}", PATTERN_COLS * PATTERN_ROWS);
                
                // 打印前几个圆点坐标
                for i in 0..std::cmp::min(5, centers.len()) {
                    let point = centers.get(i).expect("Failed to get center point");
                    println!("  - 圆点{}: ({:.2}, {:.2})", i, point.x, point.y);
                }
                
                if centers.len() == (PATTERN_COLS * PATTERN_ROWS) as usize {
                    println!("✓ 检测到的圆点数量正确");
                } else {
                    println!("⚠ 检测到的圆点数量不匹配");
                }
            }
            Err(e) => {
                println!("❌ 圆点检测失败: {:?}", e);
                panic!("Circle detection failed");
            }
        }
    }
} 