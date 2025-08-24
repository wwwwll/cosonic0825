// 限定使用 opencv 4.10.0
// 仅支持格式为 tag35h11 的 AprilTag Grid

use opencv::{
    calib3d,
    core::{Mat, Point2f, Point3f, Size, Vector, Rect, Ptr},
    objdetect::{self, ArucoDetector, DetectorParameters, PredefinedDictionaryType, RefineParameters},
    prelude::*,
    aruco,
};
use std::collections::HashMap;
use crate::modules::param_io::*;

pub struct Calibrator {
    image_size: Size,                 // Size::new(width pixel i32, height pixel i32) image pixel size
    tag_size: f32,                    // AprilTag实际尺寸(mm)
    tag_separation: f32,              // tag间距(mm)
    grid_size: Size,                  // Size::new(width number i32, height number i32) number of markers in x and y directions
    detector: ArucoDetector,
    error_threshold: f64,             // 重投影误差阈值
    board: Ptr<objdetect::Board>,    // AprilTag board
}

impl Calibrator {
    /// 创建标定器实例
    pub fn new(
        image_size: Size, // image pixel size width & height
        tag_size: f32, // 输入mm，内部转换为meters
        tag_separation: f32, // 输入mm，内部转换为meters
        grid_size: Size, // cols markers number, rows markers number
        error_threshold: f64,
    ) -> Result<Self, opencv::Error> {
        let dict = objdetect::get_predefined_dictionary(
            PredefinedDictionaryType::DICT_APRILTAG_36h11
        )?;
        let params = DetectorParameters::default()?;
        let refine_params = RefineParameters::new_def()?;
        let detector = ArucoDetector::new(&dict, &params, refine_params)?;

        let ids_mat = opencv::core::Mat::from_slice(&[
             2,  3,  4,  5,  6,
             9, 10, 11, 12, 13,
            16, 17, 18, 19, 20,
            23, 24, 25, 26, 27,
            30, 31, 32, 33, 34,
        ])?;

        // 转换单位：mm -> meters
        let tag_size_m = tag_size / 1000.0;
        let tag_separation_m = tag_separation / 1000.0;

        // 创建board
        let grid_board = objdetect::GridBoard::new(
            grid_size,  // cols, rows marker number
            tag_size_m, // meters
            tag_separation_m,  // meters
            //tag_size,
            //tag_separation,
            &dict,
            &ids_mat,  // tag ID
        )?;

        let grid_board_ptr = Ptr::new(grid_board);
        let board: Ptr<objdetect::Board> = grid_board_ptr.into();
        
        Ok(Self {
            image_size,
            tag_size,
            tag_separation,
            grid_size,
            detector,
            error_threshold,
            board,
        })
    }

    /// 3.2.1-3.2.2 AprilTag角点检测
    pub fn detect_corners(&self, image: &Mat) -> Result<(Vector<Vector<Point2f>>, Mat), opencv::Error> {
        let mut corners = Vector::<Vector<Point2f>>::new();
        let mut ids = Mat::default();
        let mut rejected = Vector::<Vector<Point2f>>::new();
        
        self.detector.detect_markers(image, &mut corners, &mut ids, &mut rejected)?;
        Ok((corners, ids))
    }

    /// 3.2.3 单目标定
    pub fn calibrate_mono(
        &self,
        all_corners: &Vector<Vector<Point2f>>,
        all_ids: &Vector<i32>,
        counter: &Vector<i32>,
    ) -> Result<MonoCalibResult, opencv::Error> {
        let mut camera_matrix = Mat::zeros(3, 3, opencv::core::CV_64F)?.to_mat()?;
        unsafe {
            *camera_matrix.at_mut::<f64>(0)? = self.image_size.width as f64; // 初始焦距估计
            *camera_matrix.at_mut::<f64>(4)? = self.image_size.width as f64;
            *camera_matrix.at_mut::<f64>(2)? = self.image_size.width as f64 / 2.0; // 主点
            *camera_matrix.at_mut::<f64>(5)? = self.image_size.height as f64 / 2.0;
            *camera_matrix.at_mut::<f64>(8)? = 1.0;
        }

        let mut dist_coeffs = Mat::zeros(5, 1, opencv::core::CV_64F)?.to_mat()?;
        let mut rvecs = Vector::<Mat>::new();
        let mut tvecs = Vector::<Mat>::new();

        // 使用aruco::calibrate_camera_aruco进行标定，无法使用matchImagePoints和solvePnP代替单目标定
        let error = aruco::calibrate_camera_aruco(
            all_corners,
            all_ids,
            counter,
            &self.board,
            self.image_size,
            &mut camera_matrix,
            &mut dist_coeffs,
            &mut rvecs,
            &mut tvecs,
            calib3d::CALIB_FIX_K3 | calib3d::CALIB_FIX_PRINCIPAL_POINT | calib3d::CALIB_USE_INTRINSIC_GUESS /*| calib3d::CALIB_RATIONAL_MODEL*/,
            opencv::core::TermCriteria::new(
                opencv::core::TermCriteria_COUNT + opencv::core::TermCriteria_EPS,
                30,
                1e-6
            )?,
        )?;

        println!("Camera Matrix:\n{:?}", camera_matrix);
        println!("Distortion Coefficients: {:?}", dist_coeffs);

        // 判断重投影误差
        if error > self.error_threshold {
            Ok(MonoCalibResult::NeedRecalibration(error))
        } else {
            Ok(MonoCalibResult::Success {
                camera_matrix,
                dist_coeffs,
                error,
            })
        }
    }

    /// 3.2.4 生成左右相机的obj/img点
    pub fn match_image_points_multi_frame(
        &self,
        left_corners: &Vector<Vector<Point2f>>,
        left_ids: &Vector<i32>,
        right_corners: &Vector<Vector<Point2f>>,
        right_ids: &Vector<i32>,
    ) -> Result<(Vector<Vector<Point3f>>, Vector<Vector<Point2f>>, Vector<Vector<Point2f>>), opencv::Error> {
        let mut obj_points = Vector::<Vector<Point3f>>::new();
        let mut left_img_points = Vector::<Vector<Point2f>>::new();
        let mut right_img_points = Vector::<Vector<Point2f>>::new();

        // 创建右图ID到索引的映射，避免O(n)的查找
        let mut right_id_to_idx = HashMap::new();
        for (idx, id) in (0..right_ids.len()).map(|i| (i, right_ids.get(i))) {
            if let Ok(id) = id {
                right_id_to_idx.insert(id, idx);
            }
        }

        // 按左图的顺序遍历，保持顺序一致性
        for i in 0..left_ids.len() {
            let left_id = left_ids.get(i)?;
            if let Some(&j) = right_id_to_idx.get(&left_id) {
                // 获取这个tag的3D点
                let mut frame_obj_points = Vector::<Point3f>::new();
                let mut frame_img_points = Vector::<Point2f>::new();
                
                // 调用board的match_image_points
                self.board.match_image_points(
                    &left_corners.get(i)?,
                    &left_ids,
                    &mut frame_obj_points,
                    &mut frame_img_points,
                )?;

                // 验证点数量一致性
                if frame_obj_points.len() == frame_img_points.len() 
                   && frame_img_points.len() == left_corners.get(i)?.len()
                   && left_corners.get(i)?.len() == right_corners.get(j)?.len() {
                    // 添加对应的点
                    obj_points.push(frame_obj_points);
                    left_img_points.push(left_corners.get(i)?);
                    right_img_points.push(right_corners.get(j)?);
                } else {
                    println!("Warning: Skipping frame due to inconsistent point counts");
                }
            }
        }

        // 最终验证所有帧的点数量一致
        for i in 0..obj_points.len() {
            if obj_points.get(i)?.len() != left_img_points.get(i)?.len() 
               || left_img_points.get(i)?.len() != right_img_points.get(i)?.len() {
                return Err(opencv::Error::new(
                    opencv::core::StsError,
                    "Point counts mismatch between object and image points".to_string()
                ));
            }
        }

        Ok((obj_points, left_img_points, right_img_points))
    }

    /// 3.2.3 双目相机标定
    pub fn calibrate_stereo(
        &self,
        obj_points: &Vector<Vector<Point3f>>,
        left_points: &Vector<Vector<Point2f>>,
        right_points: &Vector<Vector<Point2f>>,
        left_camera: &MonoCamera,
        right_camera: &MonoCamera,
    ) -> Result<StereoCalibResult, opencv::Error> {
        let mut r = Mat::default(); // rotation matirx
        let mut t = Mat::default(); // translation vector
        let mut e = Mat::default(); // essential matrix, 暂时不用
        let mut f = Mat::default(); // fundamental matrix，暂时不用

        // stereo_calibrate 需要可变输入/输出矩阵，先复制一份可变矩阵
        let mut k1 = left_camera.camera_matrix.clone();
        let mut d1 = left_camera.dist_coeffs.clone();
        let mut k2 = right_camera.camera_matrix.clone();
        let mut d2 = right_camera.dist_coeffs.clone();

        // 执行标定并获取重投影误差
        let error = calib3d::stereo_calibrate(
            obj_points,
            left_points,
            right_points,
            &mut k1,
            &mut d1,
            &mut k2,
            &mut d2,
            self.image_size,
            &mut r,
            &mut t,
            &mut e, /*暂时不用 */
            &mut f, /*暂时不用 */
            calib3d::CALIB_FIX_INTRINSIC,
            opencv::core::TermCriteria::default()?,
        )?;

        // 判断重投影误差
        if error > self.error_threshold {
            Ok(StereoCalibResult::NeedRecalibration(error))
        } else {
            Ok(StereoCalibResult::Success { r, t, error })
        }
    }

    /// 3.2.4 计算立体校正映射
    pub fn compute_stereo_rectify(
        &self,
        left_camera: &MonoCamera,
        right_camera: &MonoCamera,
        r: &Mat,
        t: &Mat,
    ) -> Result<RectifyMaps, opencv::Error> {
        let mut r1 = Mat::default();
        let mut r2 = Mat::default();
        let mut p1 = Mat::default();
        let mut p2 = Mat::default();
        let mut q = Mat::default();

        let mut roi1 = Rect::default();
        let mut roi2 = Rect::default();

        calib3d::stereo_rectify(
            &left_camera.camera_matrix,
            &left_camera.dist_coeffs,
            &right_camera.camera_matrix,
            &right_camera.dist_coeffs,
            self.image_size,
            r,
            t,
            &mut r1,
            &mut r2,
            &mut p1,
            &mut p2,
            &mut q,
            calib3d::CALIB_ZERO_DISPARITY,
            -1.0,
            self.image_size,
            &mut roi1,
            &mut roi2,
        )?;

        Ok(RectifyMaps { r1, r2, p1, p2, q })
    }

    /// 3.2.5 计算重映射矩阵
    pub fn compute_undistort_maps(
        &self,
        camera_matrix: &Mat,
        dist_coeffs: &Mat,
        r: &Mat,
        p: &Mat,
    ) -> Result<(Mat, Mat), opencv::Error> {
        let mut map1 = Mat::default();
        let mut map2 = Mat::default();

        calib3d::init_undistort_rectify_map(
            camera_matrix,
            dist_coeffs,
            r,
            p,
            self.image_size,
            opencv::core::CV_32FC1,
            &mut map1,
            &mut map2,
        )?;

        Ok((map1, map2))
    }
}

// 数据结构定义
pub struct MonoCamera {
    pub camera_matrix: Mat,
    pub dist_coeffs: Mat,
}

pub enum MonoCalibResult {
    Success {
        camera_matrix: Mat,
        dist_coeffs: Mat,
        error: f64,
    },
    NeedRecalibration(f64),
}

pub enum StereoCalibResult {
    Success {
        r: Mat,
        t: Mat,
        error: f64,
    },
    NeedRecalibration(f64),
}

pub struct RectifyMaps {
    pub r1: Mat,
    pub r2: Mat,
    pub p1: Mat,
    pub p2: Mat,
    pub q: Mat,
}