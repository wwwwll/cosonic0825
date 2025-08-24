use opencv::{
    calib3d,
    core::{Mat, Point2f, Point3d, Vector, norm, NORM_L2, Size},
    objdetect::{self, ArucoDetector},
    prelude::*,
};
use crate::modules::param_io::*;
use std::f64::consts::PI;

// 合像检测结果
#[derive(Debug)]
pub struct MergingResult {
    pub single_ok: bool,
    pub single_rotation_deg: f64,
    pub fusion_ok: bool,
    pub dx_mm: f64,
    pub dy_mm: f64,
    pub dz_mm: f64,
    pub drot_deg: f64,
    pub center_ok: bool,      // 中轴偏移是否通过
    pub theta_x_deg: f64,     // 水平偏移角(度)
    pub theta_y_deg: f64,     // 垂直偏移角(度)
    pub theta_radial_deg: f64, // 径向总偏移角(度)
}

// 误差阈值配置
pub struct MergingThreshold {
    pub rotation_deg: f64,    // 旋转角度误差阈值(度)
    pub x_px: f64,           // x方向误差阈值(像素)
    pub y_px: f64,           // y方向误差阈值(像素)
    pub z_diopter: f64,      // z方向误差阈值(屈光度)
    pub center_deg: f64,     // 中轴偏移角度阈值(度)
}

impl Default for MergingThreshold {
    fn default() -> Self {
        Self {
            rotation_deg: 0.10,  // ±0.10°
            x_px: 5.0,          // ≈5像素
            y_px: 5.0,          // ≈5像素
            z_diopter: 0.50,    // ±0.50屈光度
            center_deg: 0.50,   // ±0.50°(30′)
        }
    }
}

pub struct MergingChecker {
    detector: ArucoDetector,
    threshold: MergingThreshold,
    camera_matrix: Mat,       // 相机内参矩阵
    image_width: i32,         // 图像宽度(像素)
    image_height: i32,        // 图像高度(像素)
    baseline: f64,            // 双目基线长度(meters)
    ppd: f64,                // 每度对应的像素数(Pixels Per Degree)
    tag_size: f64,           // AprilTag实际尺寸(meters)
    object_points: Vector<Point3d>, // 标准AprilTag四个角点的3D坐标(meters)
}

impl MergingChecker {
    /// 创建合像检测器实例
    pub fn new(
        camera_matrix: Mat,
        image_size: (i32, i32),
        baseline: f64,        // 输入mm，内部转换为meters
        tag_size: f64,       // 输入mm，内部转换为meters
        threshold: Option<MergingThreshold>,
    ) -> Result<Self, opencv::Error> {
        // 转换单位：mm -> meters
        let baseline_m = baseline / 1000.0;
        let tag_size_m = tag_size / 1000.0;

        let dict = objdetect::get_predefined_dictionary(
            objdetect::PredefinedDictionaryType::DICT_APRILTAG_36h11
        )?;
        let params = objdetect::DetectorParameters::default()?;
        let refine_params = objdetect::RefineParameters::new_def()?;
        let detector = ArucoDetector::new(&dict, &params, refine_params)?;

        // 计算PPD
        let fx = *camera_matrix.at::<f64>(0)?;
        let fov_h = 2.0 * (image_size.0 as f64 / (2.0 * fx)).atan() * 180.0 / PI;
        let ppd = image_size.0 as f64 / fov_h;

        // 构建标准AprilTag 3D坐标点(meters)
        let half_size = tag_size_m / 2.0;
        let mut object_points = Vector::new();
        object_points.push(Point3d::new(-half_size, -half_size, 0.0));
        object_points.push(Point3d::new(half_size, -half_size, 0.0));
        object_points.push(Point3d::new(half_size, half_size, 0.0));
        object_points.push(Point3d::new(-half_size, half_size, 0.0));

        Ok(Self {
            detector,
            threshold: threshold.unwrap_or_default(),
            camera_matrix,
            image_width: image_size.0,
            image_height: image_size.1,
            baseline: baseline_m,
            ppd,
            tag_size: tag_size_m,
            object_points,
        })
    }

    /// 3.4.1 AprilTag角点检测
    pub fn detect_corners(&self, image: &Mat) -> Result<Vector<Vector<Point2f>>, opencv::Error> {
        let mut corners = Vector::<Vector<Point2f>>::new();
        let mut ids = Mat::default();
        let mut rejected = Vector::<Vector<Point2f>>::new();
        
        self.detector.detect_markers(image, &mut corners, &mut ids, &mut rejected)?;
        Ok(corners)
    }

    /// 3.4.2 单光机判定
    pub fn check_single_projector(
        &self,
        corners: &Vector<Vector<Point2f>>,
    ) -> Result<(bool, f64), opencv::Error> {
        if corners.is_empty() {
            return Ok((false, 0.0));
        }

        // 使用solvePnP计算旋转角度
        let (_, rotation) = self.calculate_pose(&corners.get(0)?)?;
        let rotation_deg = rotation.z * 180.0 / PI;  // 取z轴旋转角度
        
        // 判断是否在误差允许区间内
        let is_ok = rotation_deg.abs() <= self.threshold.rotation_deg;
        
        Ok((is_ok, rotation_deg))
    }

    /// 3.4.3 双光机合像判定
    pub fn check_stereo_fusion(
        &self,
        left_corners: &Vector<Vector<Point2f>>,
        right_corners: &Vector<Vector<Point2f>>,
    ) -> Result<MergingResult, opencv::Error> {
        if left_corners.is_empty() || right_corners.is_empty() {
            return Ok(MergingResult {
                single_ok: false,
                single_rotation_deg: 0.0,
                fusion_ok: false,
                dx_mm: 0.0,
                dy_mm: 0.0,
                dz_mm: 0.0,
                drot_deg: 0.0,
                center_ok: false,
                theta_x_deg: 0.0,
                theta_y_deg: 0.0,
                theta_radial_deg: 0.0,
            });
        }

        // 获取左右图像中AprilTag的中心点
        let left_corners = left_corners.get(0)?;
        let right_corners = right_corners.get(0)?;
        let left_center = self.calculate_tag_center(&left_corners);
        let right_center = self.calculate_tag_center(&right_corners);

        // 计算x/y方向像素偏差
        let dx_px = (right_center.x - left_center.x) as f64;
        let dy_px = (right_center.y - left_center.y) as f64;

        // 计算深度(mm)
        let depth = self.calculate_depth(left_center.x as f64, right_center.x as f64);
        
        // 计算x/y方向物理偏差(mm)
        let fx = *self.camera_matrix.at::<f64>(0)?;
        let fy = *self.camera_matrix.at::<f64>(4)?;  // 3x3矩阵中(1,1)位置对应索引4
        let dx_mm = dx_px * depth / fx;
        let dy_mm = dy_px * depth / fy;

        // 计算旋转角度偏差
        let (_, left_rot) = self.calculate_pose(&left_corners)?;
        let (_, right_rot) = self.calculate_pose(&right_corners)?;
        let drot_deg = (right_rot.z - left_rot.z) * 180.0 / PI;

        // 判断各项指标是否在误差允许区间内
        let x_ok = dx_px.abs() <= self.threshold.x_px;
        let y_ok = dy_px.abs() <= self.threshold.y_px;
        
        // 计算深度差对应的屈光度
        let dz_diopter = 1000.0 / depth; // 转换为屈光度(D = 1/m = 1000/mm)
        let z_ok = dz_diopter.abs() <= self.threshold.z_diopter;
        
        let rot_ok = drot_deg.abs() <= self.threshold.rotation_deg;

        // 计算中轴偏移角度
        let cx = *self.camera_matrix.at::<f64>(2)?;  // 主点x坐标
        let cy = *self.camera_matrix.at::<f64>(5)?;  // 主点y坐标

        let du_l = left_center.x as f64 - cx;
        let dv_l = left_center.y as f64 - cy;
        let du_r = right_center.x as f64 - cx;
        let dv_r = right_center.y as f64 - cy;

        let theta_x = ((du_l / fx + du_r / fx) * 0.5) * 180.0 / PI;
        let theta_y = ((dv_l / fy + dv_r / fy) * 0.5) * 180.0 / PI;
        let theta_radial = (theta_x * theta_x + theta_y * theta_y).sqrt();

        let center_ok = theta_radial <= self.threshold.center_deg;

        // 获取单光机判定结果
        let mut left_corners_vec = Vector::new();
        left_corners_vec.push(left_corners.clone());
        let (single_ok, single_rotation_deg) = self.check_single_projector(&left_corners_vec)?;

        Ok(MergingResult {
            single_ok,
            single_rotation_deg,
            fusion_ok: x_ok && y_ok && z_ok && rot_ok && center_ok,
            dx_mm,
            dy_mm,
            dz_mm: depth,
            drot_deg,
            center_ok,
            theta_x_deg: theta_x,
            theta_y_deg: theta_y,
            theta_radial_deg: theta_radial,
        })
    }

    /// 计算深度(mm)
    fn calculate_depth(&self, u_l: f64, u_r: f64) -> f64 {
        let fx = unsafe { *self.camera_matrix.at::<f64>(0).unwrap_unchecked() };
        let depth_m = fx * self.baseline / (u_l - u_r);
        depth_m * 1000.0  // 转换回mm返回，因为MergingResult中的dz_mm期望单位是mm
    }

    /// 使用solvePnP计算位姿
    fn calculate_pose(&self, corners: &Vector<Point2f>) -> Result<(Point3d, Point3d), opencv::Error> {
        let mut rvec = Mat::default();
        let mut tvec = Mat::default();
        let dist_coeffs = Mat::default(); // 假设图像已经校正，不需要畸变系数

        calib3d::solve_pnp(
            &self.object_points,
            corners,
            &self.camera_matrix,
            &dist_coeffs,
            &mut rvec,
            &mut tvec,
            false,
            calib3d::SOLVEPNP_IPPE,
        )?;

        // 将旋转向量转换为欧拉角
        let mut rotation_mat = Mat::default();
        calib3d::rodrigues(&rvec, &mut rotation_mat, &mut Mat::default())?;
        
        // 从旋转矩阵计算欧拉角
        let euler = self.rotation_matrix_to_euler_angles(&rotation_mat)?;
        
        Ok((
            Point3d::new(
                *tvec.at::<f64>(0)?,
                *tvec.at::<f64>(1)?,
                *tvec.at::<f64>(2)?
            ),
            euler
        ))
    }

    /// 将旋转矩阵转换为欧拉角
    fn rotation_matrix_to_euler_angles(&self, r: &Mat) -> Result<Point3d, opencv::Error> {
        let sy = (r.at::<f64>(0)?.powi(2) + r.at::<f64>(3)?.powi(2)).sqrt();
        
        let singular = sy < 1e-6;
        
        let (x, y, z) = if !singular {
            (
                (r.at::<f64>(7)? / r.at::<f64>(8)?).atan(),
                (-r.at::<f64>(6)?).atan2(sy),
                (r.at::<f64>(3)? / r.at::<f64>(0)?).atan(),
            )
        } else {
            (
                0.0,
                (-r.at::<f64>(6)?).atan2(sy),
                0.0,
            )
        };
        
        Ok(Point3d::new(x, y, z))
    }

    /// 计算AprilTag中心点
    fn calculate_tag_center(&self, corners: &Vector<Point2f>) -> Point2f {
        let mut x = 0.0;
        let mut y = 0.0;
        for i in 0..corners.len() {
            let corner = corners.get(i).unwrap();
            x += corner.x as f64;
            y += corner.y as f64;
        }
        Point2f::new(
            (x / corners.len() as f64) as f32,
            (y / corners.len() as f64) as f32
        )
    }
}
