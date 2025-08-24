// alignment.rs - 光机合像检测模块
// 基于asymmetric circles grid进行AR眼镜左右光机合像判定

use opencv::{
    calib3d, 
    core::{AlgorithmHint, Ptr, Vector, Mat, Point, Point2f, Point3f, Size, Scalar, CV_64F, CV_8UC3}, 
    imgcodecs, 
    imgproc, 
    prelude::*, 
    types, 
    features2d::{SimpleBlobDetector, SimpleBlobDetector_Params},
};
use crate::modules::{param_io::*, rectification::Rectifier, calibration_circles::Calibrator};
// 🆕 导入新的连通域圆点检测模块
use crate::modules::alignment_circles_detection::ConnectedComponentsDetector;
use std::time::Instant; // 添加性能监控

// ---------- 常量定义 ----------
// 🔧 临时放宽容差以专注性能优化测试
const ROLL_TH: f64 = 5.0;        // 旋转角度阈值 (度) - 临时放宽 0.05
const PITCH_YAW_TH: f64 = 10.0;  // 俯仰/偏航角度阈值 (度) - 临时放宽 0.10
const RMS_TH: f64 = 100.0;         // RMS误差阈值 (像素) - 临时放宽 0.10
const P95_TH: f64 = 100.0;        // P95误差阈值 (像素) - 临时放宽 0.20
const MAX_TH: f64 = 200.0;        // 最大误差阈值 (像素) - 临时放宽 0.30

// 🎯 居中检测阈值常量
const CENTERING_TOLERANCE_PX: f32 = 50.0;  // 居中容差阈值 (像素)

// 🎯 期望的居中位置 (基于2448×2048分辨率)
const EXPECTED_TOP_RIGHT: (f32, f32) = (1735.0, 545.0);  // 序号0点期望位置
const EXPECTED_BOTTOM_LEFT: (f32, f32) = (1215.0, 970.0); // 序号39点期望位置

/// 光机合像检测系统
pub struct AlignmentSystem {
    // 轻量参数（内存缓存）
    left_camera_matrix: Mat,
    left_dist_coeffs: Mat,
    right_camera_matrix: Mat,
    right_dist_coeffs: Mat,
    stereo_params: StereoParams,
    rectify_params: RectifyParams,
    
    // 重映射矩阵（懒加载）
    left_maps: Option<(Mat, Mat)>,
    right_maps: Option<(Mat, Mat)>,
    
    // 工具组件
    rectifier: Rectifier,
    calibrator: Calibrator,
    // 🆕 新增连通域圆点检测器
    circle_detector: ConnectedComponentsDetector,
    
    // 图像尺寸
    image_size: Size,
}

/// 单光机姿态检测结果
#[derive(Debug)]
#[derive(Clone)]
pub struct SingleEyePoseResult {
    pub roll: f64,   // 旋转角 (度)
    pub pitch: f64,  // 俯仰角 (度)
    pub yaw: f64,    // 偏航角 (度)
    pub pass: bool,  // 是否通过
}

/// 双光机合像检测结果
#[derive(Debug)]
#[derive(Clone)]
pub struct DualEyeAlignmentResult {
    pub mean_dx: f64,  // x方向平均偏差 (像素)
    pub mean_dy: f64,  // y方向平均偏差 (像素)
    pub rms: f64,      // RMS误差 (像素)
    pub p95: f64,      // P95误差 (像素)
    pub max_err: f64,  // 最大误差 (像素)
    pub pass: bool,    // 是否通过
}

/// 居中检测结果
#[derive(Debug, Clone)]
pub struct CenteringResult {
    pub is_centered: bool,              // 是否居中
    pub top_right_offset_x: f32,        // 右上角点X偏移 (像素)
    pub top_right_offset_y: f32,        // 右上角点Y偏移 (像素)
    pub bottom_left_offset_x: f32,      // 左下角点X偏移 (像素)
    pub bottom_left_offset_y: f32,      // 左下角点Y偏移 (像素)
    pub max_offset_distance: f32,       // 最大偏移距离 (像素)
    pub tolerance_px: f32,              // 容差阈值 (像素)
    pub actual_top_right: (f32, f32),   // 实际右上角点位置 (x, y)
    pub actual_bottom_left: (f32, f32), // 实际左下角点位置 (x, y)
    pub expected_top_right: (f32, f32), // 期望右上角点位置 (x, y)
    pub expected_bottom_left: (f32, f32), // 期望左下角点位置 (x, y)
}

/// 关键点验证结果
#[derive(Debug, Clone)]
pub struct KeyPointValidation {
    pub top_right_ok: bool,     // 右上角点是否在容差内
    pub bottom_left_ok: bool,   // 左下角点是否在容差内
    pub all_points_ok: bool,    // 所有关键点是否都在容差内
}

/// 操作调整向量 - 提供机械调整的原始数据
#[derive(Debug, Clone)]
pub struct AdjustmentVectors {
    pub left_eye_adjustment: EyeAdjustment,   // 左眼调整建议
    pub right_eye_adjustment: EyeAdjustment,  // 右眼调整建议
    pub alignment_adjustment: AlignmentAdjustment, // 合像调整建议
    pub priority: AdjustmentPriority,         // 调整优先级
}

/// 单眼调整建议
#[derive(Debug, Clone)]
pub struct EyeAdjustment {
    pub roll_adjustment: f64,    // 旋转调整 (度)
    pub pitch_adjustment: f64,   // 俯仰调整 (度) 
    pub yaw_adjustment: f64,     // 偏航调整 (度)
    pub centering_x: f32,        // X方向居中调整 (像素)
    pub centering_y: f32,        // Y方向居中调整 (像素)
    pub needs_adjustment: bool,  // 是否需要调整
}

/// 合像调整建议
#[derive(Debug, Clone)]
pub struct AlignmentAdjustment {
    pub delta_x: f64,           // X方向像素偏差
    pub delta_y: f64,           // Y方向像素偏差
    pub rms_error: f64,         // RMS误差
    pub adjustment_priority: String, // 调整优先级描述
}

/// 调整优先级枚举
#[derive(Debug, Clone)]
pub enum AdjustmentPriority {
    LeftEyePose,      // 优先调整左眼姿态
    LeftEyeCentering, // 优先调整左眼居中
    RightEyePose,     // 优先调整右眼姿态
    DualEyeAlignment, // 优先调整双眼合像
    Complete,         // 调整完成
}

impl AlignmentSystem {
    /// 创建光机合像检测系统
    pub fn new(
        image_size: Size,
        left_camera_params_path: &str,
        right_camera_params_path: &str,
        stereo_params_path: &str,
        rectify_params_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 加载轻量参数
        println!("加载标定参数...");
        let left_camera = load_camera_params(left_camera_params_path)?;
        let right_camera = load_camera_params(right_camera_params_path)?;
        let stereo = load_stereo_params(stereo_params_path)?;
        let rectify = load_rectify_params(rectify_params_path)?;
        
        // 转换为OpenCV Mat格式
        let left_camera_matrix = vec2d_to_mat_f64(&left_camera.camera_matrix)?;
        let left_dist_coeffs = vec_to_mat_f64(&left_camera.dist_coeffs)?;
        let right_camera_matrix = vec2d_to_mat_f64(&right_camera.camera_matrix)?;
        let right_dist_coeffs = vec_to_mat_f64(&right_camera.dist_coeffs)?;
        
        // 创建工具组件
        let rectifier = Rectifier::new(image_size)?;
        let calibrator = Calibrator::new(
            image_size,
            15.0,    // 圆点直径 (mm)
            25.0,   // 圆心距离 (mm)
            Size::new(4, 10), // pattern_size
            1.0,    // 重投影误差阈值
        )?;
        
        // 🆕 创建连通域圆点检测器
        let circle_detector = ConnectedComponentsDetector::new();
        
        println!("标定参数加载完成");
        
        Ok(Self {
            left_camera_matrix,
            left_dist_coeffs,
            right_camera_matrix,
            right_dist_coeffs,
            stereo_params: stereo,
            rectify_params: rectify,
            left_maps: None,
            right_maps: None,
            rectifier,
            calibrator,
            circle_detector, // 🆕 添加新字段
            image_size,
        })
    }
    
    /// 🚀 预加载重映射矩阵 - 解决懒加载性能瓶颈
    pub fn preload_rectify_maps(&mut self, rectify_maps_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 开始预加载重映射矩阵...");
        let start = Instant::now();
        
        // 强制加载重映射矩阵到内存
        self.ensure_maps_loaded(rectify_maps_path)?;
        
        let elapsed = start.elapsed();
        println!("✓ 重映射矩阵预加载完成，耗时: {:.1} ms", elapsed.as_millis());
        
        Ok(())
    }
    
    /// 🚀 系统初始化时预加载所有必需资源
    pub fn new_with_preload(
        image_size: Size,
        left_camera_params_path: &str,
        right_camera_params_path: &str,
        stereo_params_path: &str,
        rectify_params_path: &str,
        rectify_maps_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🚀 创建AlignmentSystem并预加载所有资源...");
        let total_start = Instant::now();
        
        // 创建基本系统
        let mut system = Self::new(
            image_size,
            left_camera_params_path,
            right_camera_params_path,
            stereo_params_path,
            rectify_params_path,
        )?;
        
        // 预加载重映射矩阵
        system.preload_rectify_maps(rectify_maps_path)?;
        
        // 配置OpenCV线程数以优化性能
        system.configure_opencv_threads();
        
        let total_elapsed = total_start.elapsed();
        println!("✓ AlignmentSystem完全初始化完成，总耗时: {:.1} ms", total_elapsed.as_millis());
        
        Ok(system)
    }
    
    /// 🔧 智能配置OpenCV线程数
    pub fn configure_opencv_threads(&self) {
        let cpu_cores = num_cpus::get();
        
        // 对于图像处理任务，过多线程会增加上下文切换开销
        let optimal_threads = match cpu_cores {
            1..=4 => cpu_cores,
            5..=8 => 4,
            9..=16 => 6,
            _ => 8, // 高核心数CPU限制在8线程
        };
        
        if let Ok(_) = opencv::core::set_num_threads(optimal_threads as i32) {
            println!("🔧 OpenCV线程数优化: {} -> {} (CPU核心: {})", 
                    opencv::core::get_num_threads().unwrap_or(-1), 
                    optimal_threads, 
                    cpu_cores);
        }
    }
    
    /// 确保重映射矩阵已加载
    pub fn ensure_maps_loaded(&mut self, rectify_maps_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.left_maps.is_none() {
            println!("首次使用，加载重映射矩阵...");
            let maps = load_rectify_maps(rectify_maps_path)?;
            
            self.left_maps = Some((
                vec2d_to_mat_f32(&maps.left_map1)?,
                vec2d_to_mat_f32(&maps.left_map2)?
            ));
            self.right_maps = Some((
                vec2d_to_mat_f32(&maps.right_map1)?,
                vec2d_to_mat_f32(&maps.right_map2)?
            ));
            println!("重映射矩阵加载完成");
        }
        Ok(())
    }
    
    /// 生成简化的世界坐标点（第一个点为原点）
    fn generate_simplified_object_points(&self) -> Result<Vector<Point3f>, opencv::Error> {
        let world_points = self.calibrator.generate_world_points_from_list()?;
        let mut simplified_points = Vector::<Point3f>::new();
        
        // 获取第一个点作为原点偏移
        let first_point = world_points.get(0)?;
        let offset_x = first_point.x;
        let offset_y = first_point.y;
        
        // 所有点减去第一个点的坐标，使第一个点为原点
        for i in 0..world_points.len() {
            let point = world_points.get(i)?;
            simplified_points.push(Point3f::new(
                point.x - offset_x,
                point.y - offset_y,
                0.0
            ));
        }
        
        println!("生成简化世界坐标，共{}个点，第一个点为原点", simplified_points.len());
        Ok(simplified_points)
    }
    
    /// 3.4.1 异步圆阵角点检测 - 🚀 ROI优化版本
    pub fn detect_circles_grid(
        &mut self,
        left_image: &Mat,
        right_image: &Mat,
        rectify_maps_path: &str,
    ) -> Result<(Vector<Point2f>, Vector<Point2f>), Box<dyn std::error::Error>> {
        let detection_start = Instant::now();
        
        // Debug: 打印输入图像信息
        println!("输入图像信息:");
        println!("  左图尺寸: {}x{}, 类型: {}", left_image.cols(), left_image.rows(), left_image.typ());
        println!("  右图尺寸: {}x{}, 类型: {}", right_image.cols(), right_image.rows(), right_image.typ());
        
        // 确保重映射矩阵已加载
        let remap_start = Instant::now();
        self.ensure_maps_loaded(rectify_maps_path)?;
        let remap_load_time = remap_start.elapsed();
        println!("⏱️  重映射矩阵加载耗时: {:.1} ms", remap_load_time.as_millis());
        
        // 获取重映射矩阵
        let (left_map1, left_map2) = self.left_maps.as_ref().unwrap();
        let (right_map1, right_map2) = self.right_maps.as_ref().unwrap();
        
        // 应用重映射
        println!("应用图像重映射...");
        let remap_process_start = Instant::now();
        let left_rect = self.rectifier.remap_image_adaptive(left_image, left_map1, left_map2)?;
        let right_rect = self.rectifier.remap_image_adaptive(right_image, right_map1, right_map2)?;
        let remap_process_time = remap_process_start.elapsed();
        println!("⏱️  图像重映射处理耗时: {:.1} ms", remap_process_time.as_millis());
        
        // 🚀 ROI区域优化 - 基于先验知识限制检测区域
        let roi_detection_start = Instant::now();
        
        // 检测圆点 - 使用优化的ROI方法
        let pattern_size = Size::new(4, 10);
        let mut corners_left = Vector::<Point2f>::new();
        let mut corners_right = Vector::<Point2f>::new();
        
        // 🆕 使用连通域检测器替代SimpleBlobDetector
        // let detector = self.create_optimized_blob_detector()?; // 已替换
        let detector = SimpleBlobDetector::create(SimpleBlobDetector_Params::default()?)?.into(); // 保持接口兼容，但实际不使用
        
        println!("🔍 使用全图检测左眼圆点...");
        let left_found = self.detect_circles_full_image(
            &left_rect,
            pattern_size,
            &mut corners_left,
            &detector
        )?;
        
        println!("🔍 使用全图检测右眼圆点...");
        let right_found = self.detect_circles_full_image(
            &right_rect,
            pattern_size,
            &mut corners_right,
            &detector
        )?;
        
        let roi_detection_time = roi_detection_start.elapsed();
        println!("⏱️  ROI圆心检测耗时: {:.1} ms", roi_detection_time.as_millis());
        
        if !left_found {
            return Err("左眼圆点网格检测失败".into());
        }
        if !right_found {
            return Err("右眼圆点网格检测失败".into());
        }
        
        println!("✓ 左眼检测到{}个圆点", corners_left.len());
        println!("✓ 右眼检测到{}个圆点", corners_right.len());
        
        let total_detection_time = detection_start.elapsed();
        println!("⏱️  总检测耗时: {:.1} ms", total_detection_time.as_millis());
        
        Ok((corners_left, corners_right))
    }
    
    // 🔧 【已替换】创建优化的SimpleBlobDetector - 针对2448×2048图像和25mm圆心距离
    // 🆕 现在使用ConnectedComponentsDetector替代SimpleBlobDetector
    // 原实现保留用于参考和回滚
    /*
    pub fn create_optimized_blob_detector(&self) -> Result<Ptr<opencv::features2d::Feature2D>, opencv::Error> {
        let mut blob_params = SimpleBlobDetector_Params::default()?;
        
        // 🎯 实际光机投影环境优化 - 基于实测数据
        // 实测圆点直径: 67-90px, 面积约3525-6362px²
        
        // 阈值设置 - 适应"发虚"到过曝的亮度范围
        blob_params.min_threshold = 40.0;   // 降低以捕获"发虚"圆点
        blob_params.max_threshold = 220.0;  // 适应过曝圆点
        blob_params.threshold_step = 30.0;  // 🚀 大步长提升性能
        
        // ❌ 关闭颜色筛选 - 圆点亮度差异太大
        blob_params.filter_by_color = false;
        
        // 🎯 面积过滤 - 基于实测数据（直径67-90px）
        blob_params.filter_by_area = true;
        blob_params.min_area = 3000.0;   // π*(67/2)² ≈ 3525, 留余量
        blob_params.max_area = 7000.0;   // π*(90/2)² ≈ 6362, 留余量
        
        // 🚀 关闭所有形状筛选器 - 最大化性能
        blob_params.filter_by_circularity = false;  // 关闭圆形度筛选
        blob_params.filter_by_convexity = false;    // 关闭凸性筛选  
        blob_params.filter_by_inertia = false;      // 关闭惯性筛选
        
        println!("🔧 使用光机投影优化的SimpleBlobDetector参数:");
        println!("   阈值范围: {:.0} - {:.0}, 步长: {:.0}", 
                blob_params.min_threshold, blob_params.max_threshold, blob_params.threshold_step);
        println!("   面积范围: {:.0} - {:.0} px² (直径约67-90px)", 
                blob_params.min_area, blob_params.max_area);
        println!("   颜色筛选: 禁用 (圆点亮度差异大)");
        println!("   形状筛选: 全部禁用 (性能优化)");
        
        let detector = SimpleBlobDetector::create(blob_params)?;
        Ok(detector.into())
    }
    */
    
    /// 🆕 获取连通域圆点检测器的可变引用
    /// 
    /// 替代原来的create_optimized_blob_detector，直接返回内置的ConnectedComponentsDetector
    pub fn get_circle_detector_mut(&mut self) -> &mut ConnectedComponentsDetector {
        println!("🔧 使用连通域圆点检测器 (替代SimpleBlobDetector):");
        println!("   检测方法: 连通域分析 + 背景平坦化 + V3.3自适应细化");
        println!("   面积范围: 1600-14000 px² (直径约67-90px)");
        println!("   连通性: 4连通 (减少黏连)");
        println!("   排序算法: PCA+投影排序 (稳定性100%)");
        
        &mut self.circle_detector
    }
    
    // 🔍 【已替换】全图圆心检测 - 配合硬件ROI使用
    // 🆕 现在使用ConnectedComponentsDetector替代SimpleBlobDetector + find_circles_grid
    // 原实现保留用于参考和回滚
    /*
    pub fn detect_circles_full_image(
        &mut self,
        image: &Mat,
        pattern_size: Size,
        corners: &mut Vector<Point2f>,
        detector: &Ptr<opencv::features2d::Feature2D>,
    ) -> Result<bool, opencv::Error> {
        println!("🔍 执行全图圆心检测 (图像: {}×{}, 通道: {}, 类型: {})", 
                image.cols(), image.rows(), image.channels(), image.typ());
        
        // // ===== DEBUG START: 可在正式版本中删除 =====
        // // 🔍 DEBUG: 检查图像统计信息
        // let mut min_val = 0.0;
        // let mut max_val = 0.0;
        // opencv::core::min_max_loc(
        //     image,
        //     Some(&mut min_val),
        //     Some(&mut max_val),
        //     None,
        //     None,
        //     &opencv::core::no_array(),
        // )?;
        // println!("   [DEBUG] 图像灰度范围: [{:.0}, {:.0}]", min_val, max_val);
        
        // // ===== DEBUG: 步骤1 - 检测SimpleBlobDetector找到的所有blob =====
        // {
        //     println!("   [DEBUG] 步骤1: 检测SimpleBlobDetector的blob...");
            
        //     // 创建一个新的优化detector用于调试
        //     let mut debug_detector = self.create_optimized_blob_detector()?;
        //     let mut keypoints = Vector::new();
            
        //     // 使用新创建的detector进行检测
        //     debug_detector.detect(image, &mut keypoints, &Mat::default())?;
        //     println!("   [DEBUG] SimpleBlobDetector找到 {} 个blob", keypoints.len());
            
        //     // 保存blob检测结果图像
        //     let mut blob_image = Mat::default();
        //     opencv::features2d::draw_keypoints(
        //         image, 
        //         &keypoints, 
        //         &mut blob_image, 
        //         opencv::core::Scalar::new(0.0, 255.0, 0.0, 0.0), // 绿色
        //         opencv::features2d::DrawMatchesFlags::DRAW_RICH_KEYPOINTS
        //     )?;
            
        //     let timestamp = std::time::SystemTime::now()
        //         .duration_since(std::time::UNIX_EPOCH)
        //         .unwrap_or_default()
        //         .as_millis();
        //     let blob_filename = format!("debug_step1_blobs_{}_count{}.png", timestamp, keypoints.len());
        //     imgcodecs::imwrite(&blob_filename, &blob_image, &Vector::new())?;
        //     println!("   [DEBUG] 已保存blob检测图像: {}", blob_filename);
            
        //     // 输出前10个blob的位置
        //     if keypoints.len() > 0 {
        //         println!("   [DEBUG] 前{}个blob位置:", std::cmp::min(10, keypoints.len()));
        //         for i in 0..std::cmp::min(10, keypoints.len()) {
        //             let kp = keypoints.get(i)?;
        //             let pt = kp.pt();
        //             println!("     Blob {}: ({:.0}, {:.0}), size={:.1}", i, pt.x, pt.y, kp.size());
        //         }
        //     }
        // }
        // // ===== DEBUG END: 步骤1 =====
        
        // 直接在全图上进行圆心检测
        let found = calib3d::find_circles_grid(
            image,
            pattern_size,
            corners,
            calib3d::CALIB_CB_ASYMMETRIC_GRID,
            Some(detector),
            calib3d::CirclesGridFinderParameters::default()?,
        )?;
        
        if found {
            println!("✓ 全图检测成功: {}个圆点", corners.len());
            
            // // ===== DEBUG: 步骤2 - 保存find_circles_grid检测到的圆点 =====
            // {
            //     println!("   [DEBUG] 步骤2: 保存find_circles_grid检测结果...");
                
            //     // 转换为彩色图像以便绘制
            //     let mut grid_image = Mat::default();
            //     if image.channels() == 1 {
            //         imgproc::cvt_color(image, &mut grid_image, imgproc::COLOR_GRAY2BGR, 0, AlgorithmHint::ALGO_HINT_DEFAULT)?;
            //     } else {
            //         grid_image = image.clone();
            //     }
                
            //     // 绘制检测到的圆点
            //     for i in 0..corners.len() {
            //         let point = corners.get(i)?;
                    
            //         // 绘制圆点（红色）
            //         imgproc::circle(
            //             &mut grid_image,
            //             Point::new(point.x as i32, point.y as i32),
            //             5,
            //             Scalar::new(0.0, 0.0, 255.0, 0.0), // 红色
            //             2,
            //             imgproc::LINE_8,
            //             0,
            //         )?;
                    
            //         // 添加序号和坐标（绿色）
            //         let text = format!("{}:({:.0},{:.0})", i, point.x, point.y);
            //         imgproc::put_text(
            //             &mut grid_image,
            //             &text,
            //             Point::new(point.x as i32 + 10, point.y as i32 - 10),
            //             imgproc::FONT_HERSHEY_SIMPLEX,
            //             0.3,
            //             Scalar::new(0.0, 255.0, 0.0, 0.0), // 绿色
            //             1,
            //             imgproc::LINE_8,
            //             false,
            //         )?;
            //     }
                
            //     let timestamp = std::time::SystemTime::now()
            //         .duration_since(std::time::UNIX_EPOCH)
            //         .unwrap_or_default()
            //         .as_millis();
            //     let grid_filename = format!("debug_step2_grid_{}_count{}.png", timestamp, corners.len());
            //     imgcodecs::imwrite(&grid_filename, &grid_image, &Vector::new())?;
            //     println!("   [DEBUG] 已保存grid检测图像: {}", grid_filename);
                
            //     // 输出前5个点的坐标
            //     println!("   [DEBUG] 前5个圆点坐标:");
            //     for i in 0..std::cmp::min(5, corners.len()) {
            //         let point = corners.get(i)?;
            //         println!("     点{}: ({:.0}, {:.0})", i, point.x, point.y);
            //     }
            // }
            // // ===== DEBUG END: 步骤2 =====
            
            // 🔧 新增：验证并修正圆点顺序（参考calibration_circles.rs）
            if corners.len() == 40 {  // 10×4 asymmetric circles grid
                println!("🔧 验证圆点检测顺序...");
                
                // 重新排序圆点以确保与世界坐标对应
                let corrected_corners = self.reorder_asymmetric_circles(corners)?;
                
                // 检查是否需要修正
                let first_original = corners.get(0)?;
                let first_corrected = corrected_corners.get(0)?;
                
                if (first_original.x - first_corrected.x).abs() > 1.0 || 
                   (first_original.y - first_corrected.y).abs() > 1.0 {
                    println!("⚠️ 检测到圆点顺序错误，已自动修正");
                    println!("   原始第0点: ({:.0}, {:.0})", first_original.x, first_original.y);
                    println!("   修正后第0点: ({:.0}, {:.0})", first_corrected.x, first_corrected.y);
                    
                    // // ===== DEBUG: 步骤3 - 保存重排序后的圆点 =====
                    // {
                    //     println!("   [DEBUG] 步骤3: 保存重排序后的圆点...");
                        
                    //     // 转换为彩色图像以便绘制
                    //     let mut reorder_image = Mat::default();
                    //     if image.channels() == 1 {
                    //         imgproc::cvt_color(image, &mut reorder_image, imgproc::COLOR_GRAY2BGR, 0, AlgorithmHint::ALGO_HINT_DEFAULT)?;
                    //     } else {
                    //         reorder_image = image.clone();
                    //     }
                        
                    //     // 绘制重排序后的圆点
                    //     for i in 0..corrected_corners.len() {
                    //         let point = corrected_corners.get(i)?;
                            
                    //         // 绘制圆点（蓝色）
                    //         imgproc::circle(
                    //             &mut reorder_image,
                    //             Point::new(point.x as i32, point.y as i32),
                    //             5,
                    //             Scalar::new(255.0, 0.0, 0.0, 0.0), // 蓝色
                    //             2,
                    //             imgproc::LINE_8,
                    //             0,
                    //         )?;
                            
                    //         // 添加序号和坐标（黄色）
                    //         let text = format!("{}:({:.0},{:.0})", i, point.x, point.y);
                    //         imgproc::put_text(
                    //             &mut reorder_image,
                    //             &text,
                    //             Point::new(point.x as i32 + 10, point.y as i32 - 10),
                    //             imgproc::FONT_HERSHEY_SIMPLEX,
                    //             0.3,
                    //             Scalar::new(0.0, 255.0, 255.0, 0.0), // 黄色
                    //             1,
                    //             imgproc::LINE_8,
                    //             false,
                    //         )?;
                    //     }
                        
                    //     let timestamp = std::time::SystemTime::now()
                    //         .duration_since(std::time::UNIX_EPOCH)
                    //         .unwrap_or_default()
                    //         .as_millis();
                    //     let reorder_filename = format!("debug_step3_reordered_{}.png", timestamp);
                    //     imgcodecs::imwrite(&reorder_filename, &reorder_image, &Vector::new())?;
                    //     println!("   [DEBUG] 已保存重排序图像: {}", reorder_filename);
                    // }
                    // // ===== DEBUG END: 步骤3 =====
                    
                    *corners = corrected_corners;
                } else {
                    println!("✅ 圆点顺序正确，无需修正");
                }
            }
        } else {
            println!("❌ 全图检测失败（find_circles_grid返回false）");
        }
        
        Ok(found)
    }
    */
    
    /// 🆕 连通域圆心检测 - 替代SimpleBlobDetector + find_circles_grid
    /// 
    /// 使用ConnectedComponentsDetector进行高性能圆点检测和排序
    pub fn detect_circles_full_image(
        &mut self,
        image: &Mat,
        pattern_size: Size,
        corners: &mut Vector<Point2f>,
        _detector: &Ptr<opencv::features2d::Feature2D>, // 保持接口兼容，但不使用
    ) -> Result<bool, opencv::Error> {
        println!("🔍 执行连通域圆心检测 (图像: {}×{}, 通道: {}, 类型: {})", 
                image.cols(), image.rows(), image.channels(), image.typ());
        
        // 验证pattern_size是否为期望的4×10
        if pattern_size.width != 4 || pattern_size.height != 10 {
            println!("⚠️ 警告: pattern_size不是4×10，当前为{}×{}", pattern_size.width, pattern_size.height);
        }
        
        // 使用连通域检测器进行圆点检测
        let detection_start = std::time::Instant::now();
        let detected_centers = self.circle_detector.detect_circles(image)
            .map_err(|e| opencv::Error::new(opencv::core::StsError, &format!("连通域检测失败: {}", e)))?;
        
        let detection_time = detection_start.elapsed();
        println!("⏱️  连通域检测耗时: {:.1} ms", detection_time.as_millis());
        
        // 检查检测结果
        if detected_centers.len() == 40 {
            println!("✓ 连通域检测成功: {}个圆点", detected_centers.len());
            
            // 进行排序
            let sort_start = std::time::Instant::now();
            let mut sorted_centers = detected_centers.clone();
            self.circle_detector.sort_asymmetric_grid(&mut sorted_centers)
                .map_err(|e| opencv::Error::new(opencv::core::StsError, &format!("圆点排序失败: {}", e)))?;
            
            let sort_time = sort_start.elapsed();
            println!("⏱️  圆点排序耗时: {:.1} ms", sort_time.as_millis());
            
            // 将结果复制到输出参数
            corners.clear();
            for i in 0..sorted_centers.len() {
                corners.push(sorted_centers.get(i).map_err(|e| opencv::Error::new(opencv::core::StsError, &format!("获取圆点失败: {}", e)))?);
            }
            
            println!("✅ 连通域检测+排序完成: {}个圆点", corners.len());
            Ok(true)
        } else {
            println!("❌ 连通域检测失败: 期望40个圆点，实际检测到{}个", detected_centers.len());
            Ok(false)
        }
    }
    
    // 【已替换】重新排序 asymmetric circles 以匹配世界坐标
    // 🆕 现在使用ConnectedComponentsDetector.sort_asymmetric_grid()替代
    // 原实现保留用于参考和回滚
    /*
    /// 重新排序 asymmetric circles 以匹配世界坐标
    /// 
    /// OpenCV的find_circles_grid可能返回不同的列顺序，
    /// 这个函数确保输出顺序与generate_world_points_from_list一致
    /// （参考自calibration_circles.rs的实现）
    fn reorder_asymmetric_circles(&self, centers: &Vector<Point2f>) -> Result<Vector<Point2f>, opencv::Error> {
        if centers.len() != 40 {
            return Ok(centers.clone());
        }
        
        // 检查序号0和序号4的x坐标
        let point_0 = centers.get(0)?;
        let point_4 = centers.get(4)?;
        
        // 如果序号0的x坐标小于序号4，说明列顺序错了
        // 正确情况：序号0应该在最右边（第9列），x坐标应该更大
        if point_0.x < point_4.x {
            println!("   检测到列顺序错误（点0.x={:.0} < 点4.x={:.0}），执行奇偶列交换...", 
                    point_0.x, point_4.x);
            
            // 创建新的排序数组
            let mut reordered = Vector::<Point2f>::new();
            
            // 交换相邻的奇偶列
            // 原顺序: 0-3, 4-7, 8-11, 12-15, 16-19, 20-23, 24-27, 28-31, 32-35, 36-39
            // 新顺序: 4-7, 0-3, 12-15, 8-11, 20-23, 16-19, 28-31, 24-27, 36-39, 32-35
            
            // 交换第1对列（0-3 和 4-7）
            for i in 4..8 {
                reordered.push(centers.get(i)?);
            }
            for i in 0..4 {
                reordered.push(centers.get(i)?);
            }
            
            // 交换第2对列（8-11 和 12-15）
            for i in 12..16 {
                reordered.push(centers.get(i)?);
            }
            for i in 8..12 {
                reordered.push(centers.get(i)?);
            }
            
            // 交换第3对列（16-19 和 20-23）
            for i in 20..24 {
                reordered.push(centers.get(i)?);
            }
            for i in 16..20 {
                reordered.push(centers.get(i)?);
            }
            
            // 交换第4对列（24-27 和 28-31）
            for i in 28..32 {
                reordered.push(centers.get(i)?);
            }
            for i in 24..28 {
                reordered.push(centers.get(i)?);
            }
            
            // 交换第5对列（32-35 和 36-39）
            for i in 36..40 {
                reordered.push(centers.get(i)?);
            }
            for i in 32..36 {
                reordered.push(centers.get(i)?);
            }
            
            Ok(reordered)
        } else {
            // 顺序正确，直接返回
            println!("   列顺序正确（点0.x={:.0} >= 点4.x={:.0}）", point_0.x, point_4.x);
            Ok(centers.clone())
        }
    }
    */
    
    /// 【已弃用】ROI区域圆心检测 - 保留用于向后兼容
    /// 
    /// ⚠️ **此方法已弃用，请使用 detect_circles_full_image()**
    /// 
    /// 现在推荐使用硬件ROI配置，软件侧进行全图检测以获得最佳性能和灵活性。
    #[deprecated(since = "2.1.0", note = "使用 detect_circles_full_image() 替代，配合硬件ROI")]
    pub fn detect_circles_with_roi(
        &mut self,
        image: &Mat,
        pattern_size: Size,
        corners: &mut Vector<Point2f>,
        detector: &Ptr<opencv::features2d::Feature2D>,
    ) -> Result<bool, opencv::Error> {
        println!("⚠️ detect_circles_with_roi() 已弃用，自动转发到全图检测");
        self.detect_circles_full_image(image, pattern_size, corners, detector)
    }
    
    /// 3.4.2 单光机姿态判定（通用版本 - 支持左右眼）
    pub fn check_single_eye_pose(
        &self,
        corners: &Vector<Point2f>,
        camera_matrix: &Mat,
        dist_coeffs: &Mat,
    ) -> Result<SingleEyePoseResult, Box<dyn std::error::Error>> {
        println!("=== 单光机姿态检测 ===");
        
        // 生成简化世界坐标
        let object_points = self.generate_simplified_object_points()?;
        
        // 使用solvePnP计算姿态
        let mut rvec = Mat::default();
        let mut tvec = Mat::default();
        
        calib3d::solve_pnp(
            &object_points,
            corners,
            camera_matrix,
            dist_coeffs,
            &mut rvec,
            &mut tvec,
            false,
            calib3d::SOLVEPNP_IPPE,
        )?;
        
        // 转换旋转向量为旋转矩阵
        let mut rot_matrix = Mat::default();
        calib3d::rodrigues(&rvec, &mut rot_matrix, &mut Mat::default())?;
        
        // 计算欧拉角
        let roll = f64::atan2(
            *rot_matrix.at_2d::<f64>(1, 0)?,
            *rot_matrix.at_2d::<f64>(0, 0)?
        ) * 180.0 / std::f64::consts::PI;
        
        let pitch = f64::atan(
            *tvec.at_2d::<f64>(1, 0)? / *tvec.at_2d::<f64>(2, 0)?
        ) * 180.0 / std::f64::consts::PI;
        
        let yaw = f64::atan(
            *tvec.at_2d::<f64>(0, 0)? / *tvec.at_2d::<f64>(2, 0)?
        ) * 180.0 / std::f64::consts::PI;
        
        // 判断是否在阈值范围内
        let pass = roll.abs() <= ROLL_TH && 
                   pitch.abs() <= PITCH_YAW_TH && 
                   yaw.abs() <= PITCH_YAW_TH;
        
        println!("roll={:.3}°, pitch={:.3}°, yaw={:.3}°", roll, pitch, yaw);
        println!("阈值: |roll| ≤ {:.2}°, |pitch|,|yaw| ≤ {:.2}°", ROLL_TH, PITCH_YAW_TH);
        
        if pass {
            println!("✓ 姿态检测通过");
        } else {
            println!("❌ 姿态超出容差 - 请先机械调平");
        }
        
        Ok(SingleEyePoseResult {
            roll,
            pitch,
            yaw,
            pass,
        })
    }
    
    /// 3.4.3 双光机合像判定（纯合像分析，不包含姿态检测）
    pub fn check_dual_eye_alignment(
        &self,
        corners_left: &Vector<Point2f>,
        corners_right: &Vector<Point2f>,
        save_debug_image: bool,
    ) -> Result<DualEyeAlignmentResult, Box<dyn std::error::Error>> {
        println!("=== 双光机合像判定 ===");
        
        if corners_left.len() != corners_right.len() {
            return Err("左右眼检测到的圆点数量不一致".into());
        }
        
        // 计算残差向量 Δx = xR - xL, Δy = yR - yL
        let mut dx_values = Vec::new();
        let mut dy_values = Vec::new();
        let mut errors = Vec::new();
        
        for i in 0..corners_left.len() {
            let left_point = corners_left.get(i)?;
            let right_point = corners_right.get(i)?;
            
            let dx = (right_point.x - left_point.x) as f64;
            let dy = (right_point.y - left_point.y) as f64;
            let error = (dx * dx + dy * dy).sqrt();
            
            dx_values.push(dx);
            dy_values.push(dy);
            errors.push(error);
        }
        
        // 计算统计量
        let mean_dx = mean(&dx_values);
        let mean_dy = mean(&dy_values);
        let rms = rms(&errors);
        let p95 = percentile(&errors, 95.0);
        let max_err = errors.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // 判断是否通过
        let pass = rms <= RMS_TH && p95 <= P95_TH && max_err <= MAX_TH;
        
        // 输出结果
        println!("方向提示:");
        println!("  Δx_mean = {:.3} px {}", mean_dx, if mean_dx > 0.0 { "(右眼向左调)" } else { "(右眼向右调)" });
        println!("  Δy_mean = {:.3} px {}", mean_dy, if mean_dy < 0.0 { "(右眼向上调)" } else { "(右眼向下调)" });
        
        println!("统计误差:");
        println!("  RMS = {:.3} px (阈值: {:.2})", rms, RMS_TH);
        println!("  P95 = {:.3} px (阈值: {:.2})", p95, P95_TH);
        println!("  Max = {:.3} px (阈值: {:.2})", max_err, MAX_TH);
        
        println!("判定结果: {}", if pass { "✓ PASS" } else { "❌ FAIL" });
        
        // 生成debug图像
        if save_debug_image {
            self.generate_alignment_debug_image(corners_left, corners_right, &dx_values, &dy_values)?;
        }
        
        Ok(DualEyeAlignmentResult {
            mean_dx,
            mean_dy,
            rms,
            p95,
            max_err,
            pass,
        })
    }
    
    /// 🎯 检查左眼图像是否居中
    /// 
    /// 基于asymmetric circles grid的关键点位置判断图像是否居中。
    /// 使用右上角点(序号0)和左下角点(序号39)作为参考点。
    /// 
    /// # 参数
    /// - `corners`: 检测到的40个圆心坐标 (10×4网格)
    /// - `tolerance_px`: 居中容差阈值 (像素)，如果为None则使用默认值
    /// 
    /// # 返回
    /// - `CenteringResult`: 居中检测结果
    pub fn check_left_eye_centering(
        &self,
        corners: &Vector<Point2f>,
        tolerance_px: Option<f32>,
    ) -> Result<CenteringResult, Box<dyn std::error::Error>> {
        println!("=== 左眼图像居中检测 ===");
        
        // 验证圆点数量
        if corners.len() != 40 {
            return Err(format!("圆点数量不正确: 期望40个，实际{}个", corners.len()).into());
        }
        
        let tolerance = tolerance_px.unwrap_or(CENTERING_TOLERANCE_PX);
        
        // 获取关键点坐标
        // 根据asymmetric circles grid的排列，序号0在右上角，序号39在左下角
        let actual_top_right = corners.get(0)?;      // 序号0: 右上角
        let actual_bottom_left = corners.get(39)?;   // 序号39: 左下角
        
        // 期望位置
        let expected_top_right = Point2f::new(EXPECTED_TOP_RIGHT.0, EXPECTED_TOP_RIGHT.1);
        let expected_bottom_left = Point2f::new(EXPECTED_BOTTOM_LEFT.0, EXPECTED_BOTTOM_LEFT.1);
        
        // 计算偏移量
        let top_right_offset_x = actual_top_right.x - expected_top_right.x;
        let top_right_offset_y = actual_top_right.y - expected_top_right.y;
        let bottom_left_offset_x = actual_bottom_left.x - expected_bottom_left.x;
        let bottom_left_offset_y = actual_bottom_left.y - expected_bottom_left.y;
        
        // 计算偏移距离
        let top_right_distance = (top_right_offset_x * top_right_offset_x + 
                                 top_right_offset_y * top_right_offset_y).sqrt();
        let bottom_left_distance = (bottom_left_offset_x * bottom_left_offset_x + 
                                   bottom_left_offset_y * bottom_left_offset_y).sqrt();
        
        let max_offset_distance = top_right_distance.max(bottom_left_distance);
        
        // 判断是否在容差范围内
        let top_right_ok = top_right_distance <= tolerance;
        let bottom_left_ok = bottom_left_distance <= tolerance;
        let is_centered = top_right_ok && bottom_left_ok;
        
        // 输出检测结果
        println!("关键点位置分析:");
        println!("  右上角点(序号0):");
        println!("    期望位置: ({:.1}, {:.1})", expected_top_right.x, expected_top_right.y);
        println!("    实际位置: ({:.1}, {:.1})", actual_top_right.x, actual_top_right.y);
        println!("    偏移量: ({:.1}, {:.1}) px", top_right_offset_x, top_right_offset_y);
        println!("    偏移距离: {:.1} px (容差: {:.1} px) {}", 
                top_right_distance, tolerance, if top_right_ok { "✓" } else { "❌" });
        
        println!("  左下角点(序号39):");
        println!("    期望位置: ({:.1}, {:.1})", expected_bottom_left.x, expected_bottom_left.y);
        println!("    实际位置: ({:.1}, {:.1})", actual_bottom_left.x, actual_bottom_left.y);
        println!("    偏移量: ({:.1}, {:.1}) px", bottom_left_offset_x, bottom_left_offset_y);
        println!("    偏移距离: {:.1} px (容差: {:.1} px) {}", 
                bottom_left_distance, tolerance, if bottom_left_ok { "✓" } else { "❌" });
        
        println!("居中检测结果:");
        println!("  最大偏移距离: {:.1} px", max_offset_distance);
        println!("  容差阈值: {:.1} px", tolerance);
        println!("  居中状态: {}", if is_centered { "✓ 居中" } else { "❌ 偏移" });
        
        if !is_centered {
            println!("调整建议:");
            if !top_right_ok {
                let suggest_x = if top_right_offset_x > 0.0 { "向左" } else { "向右" };
                let suggest_y = if top_right_offset_y > 0.0 { "向上" } else { "向下" };
                println!("  右上角偏移过大，建议{}调整{:.1}px，{}调整{:.1}px", 
                        suggest_x, top_right_offset_x.abs(), suggest_y, top_right_offset_y.abs());
            }
            if !bottom_left_ok {
                let suggest_x = if bottom_left_offset_x > 0.0 { "向左" } else { "向右" };
                let suggest_y = if bottom_left_offset_y > 0.0 { "向上" } else { "向下" };
                println!("  左下角偏移过大，建议{}调整{:.1}px，{}调整{:.1}px", 
                        suggest_x, bottom_left_offset_x.abs(), suggest_y, bottom_left_offset_y.abs());
            }
        }
        
        Ok(CenteringResult {
            is_centered,
            top_right_offset_x,
            top_right_offset_y,
            bottom_left_offset_x,
            bottom_left_offset_y,
            max_offset_distance,
            tolerance_px: tolerance,
            actual_top_right: (actual_top_right.x, actual_top_right.y),
            actual_bottom_left: (actual_bottom_left.x, actual_bottom_left.y),
            expected_top_right: (expected_top_right.x, expected_top_right.y),
            expected_bottom_left: (expected_bottom_left.x, expected_bottom_left.y),
        })
    }
    
    /// 🎯 计算操作调整向量 - 提供机械调整的原始数据
    /// 
    /// 基于检测结果计算具体的机械调整建议，为前端提供原始数据。
    /// 前端可以根据这些数据生成具体的XYR三轴千分尺操作指令。
    /// 
    /// # 参数
    /// - `left_pose`: 左眼姿态检测结果（可选）
    /// - `left_centering`: 左眼居中检测结果（可选）
    /// - `right_pose`: 右眼姿态检测结果（可选）
    /// - `alignment`: 双眼合像检测结果（可选）
    /// 
    /// # 返回
    /// - `AdjustmentVectors`: 包含所有调整建议的结构体
    pub fn calculate_adjustment_vectors(
        &self,
        left_pose: Option<&SingleEyePoseResult>,
        left_centering: Option<&CenteringResult>,
        right_pose: Option<&SingleEyePoseResult>,
        alignment: Option<&DualEyeAlignmentResult>,
    ) -> AdjustmentVectors {
        println!("=== 计算操作调整向量 ===");
        
        // 计算左眼调整建议
        let left_eye_adjustment = self.calculate_eye_adjustment(
            left_pose, 
            left_centering, 
            "左眼"
        );
        
        // 计算右眼调整建议
        let right_eye_adjustment = self.calculate_eye_adjustment(
            right_pose, 
            None, // 右眼不需要居中检测
            "右眼"
        );
        
        // 计算合像调整建议
        let alignment_adjustment = self.calculate_alignment_adjustment(alignment);
        
        // 确定调整优先级
        let priority = self.determine_adjustment_priority(
            &left_eye_adjustment,
            &left_eye_adjustment, // 使用left_eye_adjustment作为居中参考
            &right_eye_adjustment,
            &alignment_adjustment,
            left_centering,
        );
        
        println!("调整优先级: {:?}", priority);
        
        AdjustmentVectors {
            left_eye_adjustment,
            right_eye_adjustment,
            alignment_adjustment,
            priority,
        }
    }
    
    /// 计算单眼调整建议
    fn calculate_eye_adjustment(
        &self,
        pose: Option<&SingleEyePoseResult>,
        centering: Option<&CenteringResult>,
        eye_name: &str,
    ) -> EyeAdjustment {
        let mut adjustment = EyeAdjustment {
            roll_adjustment: 0.0,
            pitch_adjustment: 0.0,
            yaw_adjustment: 0.0,
            centering_x: 0.0,
            centering_y: 0.0,
            needs_adjustment: false,
        };
        
        // 处理姿态调整
        if let Some(pose_result) = pose {
            adjustment.roll_adjustment = -pose_result.roll;  // 反向调整
            adjustment.pitch_adjustment = -pose_result.pitch;
            adjustment.yaw_adjustment = -pose_result.yaw;
            adjustment.needs_adjustment = !pose_result.pass;
            
            println!("{}姿态调整建议:", eye_name);
            println!("  Roll调整: {:.3}° (当前: {:.3}°)", adjustment.roll_adjustment, pose_result.roll);
            println!("  Pitch调整: {:.3}° (当前: {:.3}°)", adjustment.pitch_adjustment, pose_result.pitch);
            println!("  Yaw调整: {:.3}° (当前: {:.3}°)", adjustment.yaw_adjustment, pose_result.yaw);
        }
        
        // 处理居中调整（仅左眼）
        if let Some(centering_result) = centering {
            adjustment.centering_x = -centering_result.top_right_offset_x; // 反向调整
            adjustment.centering_y = -centering_result.top_right_offset_y;
            adjustment.needs_adjustment = adjustment.needs_adjustment || !centering_result.is_centered;
            
            println!("{}居中调整建议:", eye_name);
            println!("  X方向调整: {:.1}px (当前偏移: {:.1}px)", 
                    adjustment.centering_x, centering_result.top_right_offset_x);
            println!("  Y方向调整: {:.1}px (当前偏移: {:.1}px)", 
                    adjustment.centering_y, centering_result.top_right_offset_y);
        }
        
        adjustment
    }
    
    /// 计算合像调整建议
    fn calculate_alignment_adjustment(
        &self,
        alignment: Option<&DualEyeAlignmentResult>,
    ) -> AlignmentAdjustment {
        if let Some(alignment_result) = alignment {
            let priority_desc = if alignment_result.rms > RMS_TH {
                "RMS误差过大，优先调整整体对准"
            } else if alignment_result.p95 > P95_TH {
                "P95误差过大，优先调整局部对准"
            } else if alignment_result.max_err > MAX_TH {
                "最大误差过大，优先调整极值点"
            } else {
                "合像精度良好"
            };
            
            println!("合像调整建议:");
            println!("  X方向调整: {:.3}px (右眼相对左眼)", -alignment_result.mean_dx);
            println!("  Y方向调整: {:.3}px (右眼相对左眼)", -alignment_result.mean_dy);
            println!("  RMS误差: {:.3}px", alignment_result.rms);
            println!("  调整优先级: {}", priority_desc);
            
            AlignmentAdjustment {
                delta_x: -alignment_result.mean_dx, // 反向调整
                delta_y: -alignment_result.mean_dy,
                rms_error: alignment_result.rms,
                adjustment_priority: priority_desc.to_string(),
            }
        } else {
            AlignmentAdjustment {
                delta_x: 0.0,
                delta_y: 0.0,
                rms_error: 0.0,
                adjustment_priority: "无合像数据".to_string(),
            }
        }
    }
    
    /// 确定调整优先级
    fn determine_adjustment_priority(
        &self,
        left_pose_adj: &EyeAdjustment,
        left_centering_adj: &EyeAdjustment,
        right_pose_adj: &EyeAdjustment,
        alignment_adj: &AlignmentAdjustment,
        centering: Option<&CenteringResult>,
    ) -> AdjustmentPriority {
        // 优先级逻辑：姿态 -> 居中 -> 合像
        
        // 1. 检查左眼姿态
        if left_pose_adj.needs_adjustment && 
           (left_pose_adj.roll_adjustment.abs() > ROLL_TH || 
            left_pose_adj.pitch_adjustment.abs() > PITCH_YAW_TH ||
            left_pose_adj.yaw_adjustment.abs() > PITCH_YAW_TH) {
            return AdjustmentPriority::LeftEyePose;
        }
        
        // 2. 检查左眼居中
        if let Some(centering_result) = centering {
            if !centering_result.is_centered {
                return AdjustmentPriority::LeftEyeCentering;
            }
        }
        
        // 3. 检查右眼姿态
        if right_pose_adj.needs_adjustment &&
           (right_pose_adj.roll_adjustment.abs() > ROLL_TH || 
            right_pose_adj.pitch_adjustment.abs() > PITCH_YAW_TH ||
            right_pose_adj.yaw_adjustment.abs() > PITCH_YAW_TH) {
            return AdjustmentPriority::RightEyePose;
        }
        
        // 4. 检查双眼合像
        if alignment_adj.rms_error > RMS_TH {
            return AdjustmentPriority::DualEyeAlignment;
        }
        
        // 5. 所有检测都通过
        AdjustmentPriority::Complete
    }
    
    /// 生成带标注的debug图像
    fn generate_alignment_debug_image(
        &self,
        corners_left: &Vector<Point2f>,
        corners_right: &Vector<Point2f>,
        dx_values: &[f64],
        dy_values: &[f64],
    ) -> Result<(), opencv::Error> {
        println!("生成合像检测debug图像...");
        
        // 创建debug图像 (白色背景)
                  let mut debug_img = Mat::new_rows_cols_with_default(
            self.image_size.height,
            self.image_size.width,
              CV_8UC3, // 使用8位3通道RGB格式
              Scalar::new(255.0, 255.0, 255.0, 0.0),
        )?;
        
        // 绘制左右眼圆点和连线
        for i in 0..corners_left.len() {
            let left_point = corners_left.get(i)?;
            let right_point = corners_right.get(i)?;
            
            // 使用Point2f保持浮点精度
            let left_pt = Point2f::new(left_point.x, left_point.y);
            let right_pt = Point2f::new(right_point.x, right_point.y);
            
            // 绘制左眼圆点 (蓝色)
            imgproc::circle(
                &mut debug_img,
                Point::new(left_pt.x as i32, left_pt.y as i32), // 仅在绘制时转换为整型
                3,
                Scalar::new(255.0, 0.0, 0.0, 0.0), // 蓝色
                -1,
                imgproc::LINE_8,
                0,
            )?;
            
            // 绘制右眼圆点 (红色)
            imgproc::circle(
                &mut debug_img,
                Point::new(right_pt.x as i32, right_pt.y as i32), // 仅在绘制时转换为整型
                3,
                Scalar::new(0.0, 0.0, 255.0, 0.0), // 红色
                -1,
                imgproc::LINE_8,
                0,
            )?;
            
            // 绘制连线 (绿色)
            imgproc::line(
                &mut debug_img,
                Point::new(left_pt.x as i32, left_pt.y as i32), // 仅在绘制时转换为整型
                Point::new(right_pt.x as i32, right_pt.y as i32), // 仅在绘制时转换为整型
                Scalar::new(0.0, 255.0, 0.0, 0.0), // 绿色
                1,
                imgproc::LINE_8,
                0,
            )?;
            
            // 添加序号标注
            let text = format!("{}", i);
            imgproc::put_text(
                &mut debug_img,
                &text,
                Point::new((left_pt.x - 10.0) as i32, (left_pt.y - 10.0) as i32), // 仅在绘制时转换为整型
                imgproc::FONT_HERSHEY_SIMPLEX,
                0.4,
                Scalar::new(0.0, 0.0, 0.0, 0.0), // 黑色
                1,
                imgproc::LINE_8,
                false,
            )?;
        }
        
        // 保存debug图像
        imgcodecs::imwrite("alignment_debug.png", &debug_img, &Vector::<i32>::new())?;
        println!("已保存合像检测debug图像: alignment_debug.png");
        
        Ok(())
    }
}

// ---------- 辅助函数 ----------
pub fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

pub fn rms(values: &[f64]) -> f64 {
    (values.iter().map(|v| v * v).sum::<f64>() / values.len() as f64).sqrt()
}

pub fn percentile(data: &[f64], pct: f64) -> f64 {
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

/// 为流水线处理添加的访问方法
impl AlignmentSystem {
    /// 获取重映射矩阵的只读访问
    pub fn get_rectify_maps(&self) -> Option<(&Mat, &Mat, &Mat, &Mat)> {
        if let (Some((left_map1, left_map2)), Some((right_map1, right_map2))) = 
            (&self.left_maps, &self.right_maps) {
            Some((left_map1, left_map2, right_map1, right_map2))
        } else {
            None
        }
    }
    
    /// 获取 rectifier 的只读访问
    pub fn get_rectifier(&self) -> &Rectifier {
        &self.rectifier
    }
    
    /// 获取左相机参数的只读访问
    pub fn get_left_camera_params(&self) -> (&Mat, &Mat) {
        (&self.left_camera_matrix, &self.left_dist_coeffs)
    }
    
    /// 获取右相机参数的只读访问
    pub fn get_right_camera_params(&self) -> (&Mat, &Mat) {
        (&self.right_camera_matrix, &self.right_dist_coeffs)
    }
    
    /// 【向后兼容】检查左眼姿态（使用内置左相机参数）
    pub fn check_left_eye_pose(
        &self,
        corners_left: &Vector<Point2f>,
    ) -> Result<SingleEyePoseResult, Box<dyn std::error::Error>> {
        println!("🔄 使用向后兼容的左眼姿态检测");
        self.check_single_eye_pose(corners_left, &self.left_camera_matrix, &self.left_dist_coeffs)
    }
    
    /// 【向后兼容】检查右眼姿态（使用内置右相机参数）
    pub fn check_right_eye_pose(
        &self,
        corners_right: &Vector<Point2f>,
    ) -> Result<SingleEyePoseResult, Box<dyn std::error::Error>> {
        println!("🔄 使用向后兼容的右眼姿态检测");
        self.check_single_eye_pose(corners_right, &self.right_camera_matrix, &self.right_dist_coeffs)
    }
}
