// 限定使用 opencv 4.10.0
// 仅支持格式为 asymmetric circle grid

use opencv::{
    calib3d::{self, CALIB_CB_ASYMMETRIC_GRID, CALIB_CB_CLUSTERING}, 
    core::{AlgorithmHint, Ptr, Mat, Point2f, Point3f, Rect, Size, TermCriteria, Vector, CV_8UC1}, 
    features2d::{SimpleBlobDetector, SimpleBlobDetector_Params}, 
    imgcodecs, imgproc::{self, COLOR_BGR2GRAY}, 
    prelude::*
};
use crate::modules::param_io::*;

/// 相机类型枚举
#[derive(Debug, Clone, Copy)]
pub enum CameraType {
    Left,
    Right,
}

impl CameraType {
    fn get_prefix(&self) -> &'static str {
        match self {
            CameraType::Left => "l",
            CameraType::Right => "r",
        }
    }
}

pub struct Calibrator {
    image_size: Size,                 // Size::new(width pixel i32, height pixel i32) image pixel size
    diameter: f32,                    // 圆点实际直径(mm)
    center_distance: f32,             // 圆点间距(mm)
    pattern_size: Size,               // number of circles per row and column ( patternSize = Size(points_per_row, points_per_colum) 
    //detector: opencv::core::Ptr<SimpleBlobDetector>,     // 圆点detector
    detector: opencv::core::Ptr<opencv::features2d::Feature2D>, // 圆点detector
    error_threshold: f64,             // 重投影误差阈值
}

impl Calibrator {
    /// 创建标定器实例
    pub fn new(
        image_size: Size,              // image pixel size width & height
        diameter: f32,                // 输入mm，内部转换为meters
        center_distance: f32,         // 输入mm，内部转换为meters
        pattern_size: Size,           // row markers number, column markers number
        error_threshold: f64,         // 重投影误差阈值
    ) -> Result<Self, opencv::Error> {

        // 创建 SimpleBlobDetector 参数 - 专门针对 asymmetric circles grid 优化
        let mut blob_params = SimpleBlobDetector_Params::default()?;

        // [配置系统 - 已注释] 使用简单配置加载参数
        // let config = crate::modules::simple_config::load_calibration_blob_params();
        
        // 写死的参数配置
        // 阈值设置
        blob_params.min_threshold = 10.0;
        blob_params.max_threshold = 200.0;
        blob_params.threshold_step = 10.0;

        // 面积过滤 - 根据25mm圆心距离调整
        blob_params.filter_by_area = true;
        blob_params.min_area = 1000.0;
        blob_params.max_area = 70000.0;

        // 圆形度过滤
        blob_params.filter_by_circularity = true;
        blob_params.min_circularity = 0.5;
        blob_params.max_circularity = 1.0;

        // 凸性过滤
        blob_params.filter_by_convexity = true;
        blob_params.min_convexity = 0.8;
        blob_params.max_convexity = 1.0;

        // 惯性过滤
        blob_params.filter_by_inertia = true;
        blob_params.min_inertia_ratio = 0.1;
        blob_params.max_inertia_ratio = 1.0;
        
        // [配置系统] 打印实际使用的参数
        println!("标定SimpleBlobDetector参数:");
        println!("  阈值: {:.1} - {:.1}, 步长: {:.1}", 
                blob_params.min_threshold, blob_params.max_threshold, blob_params.threshold_step);
        println!("  面积: {:.0} - {:.0}", blob_params.min_area, blob_params.max_area);
        println!("  圆形度: {:.2} - {:.2}", blob_params.min_circularity, blob_params.max_circularity);

        // Create a detector with the parameters
        let detector = SimpleBlobDetector::create(blob_params)?;
        // 将 SimpleBlobDetector 转换为 Feature2D
        let detector: Ptr<opencv::features2d::Feature2D> = detector.into(); 
        
        Ok(Self {
            image_size,
            diameter,
            center_distance,
            pattern_size,
            detector,
            error_threshold,
        })
    }

    /// 生成 asymmetric circle grid 的世界坐标点
    /// 按照 OpenCV 的要求：10列4行，先遍历列再遍历行
    /// TODO：该函数生成逻辑有问题，需要修改
    pub fn generate_asymmetric_circle_grid_world_points(&self) -> Result<Vector<Point3f>, opencv::Error> {
        // diagonal spacing = 25mm，所以水平/垂直间距 = 25 / √2 mm
        let spacing = self.center_distance / (2.0_f32.sqrt()); // x = 25/√2 ≈ 17.68mm
        let n_rows = self.pattern_size.height;    // 4行
        let n_cols = self.pattern_size.width;     // 10列

        let mut world_points = Vector::<Point3f>::new();

        println!("生成世界坐标: {}列 x {}行", n_cols, n_rows);
        println!("diagonal spacing = {:.2}mm, 水平/垂直间距 x = {:.2}mm", self.center_distance, spacing);

        // 根据用户提供的坐标表格：从右到左遍历列，每列内从上到下遍历行
        println!("=== 按照用户提供的坐标表格生成世界坐标 ===");
        
        // 从右到左遍历列（col从最大到0），每列内从上到下遍历行
        for col in (0..n_cols).rev() {  // 从右到左：9,8,7...1,0
            for row in 0..n_rows {      // 从上到下：0,1,2,3
                // x坐标：第9列是9x，第8列是8x，...，第0列是0x
                let x = (col as f32) * spacing;
                
                // y坐标根据列的奇偶性确定：
                // 偶数列（0,2,4,6,8）：0, 2x, 4x, 6x
                // 奇数列（1,3,5,7,9）：x, 3x, 5x, 7x
                let y = if col % 2 == 0 {
                    // 偶数列：0, 2x, 4x, 6x
                    (row * 2) as f32 * spacing
                } else {
                    // 奇数列：x, 3x, 5x, 7x  
                    ((row * 2) + 1) as f32 * spacing
                };
                
                let z = 0.0;
                
                world_points.push(Point3f::new(x, y, z));
                
                // 输出所有点的坐标用于调试
                println!("世界坐标点{}: 列{}行{} -> ({:.1}, {:.1}, {:.1})", 
                         world_points.len()-1, col, row, x, y, z);
            }
        }

        println!("总共生成了 {} 个世界坐标点", world_points.len());
        Ok(world_points)
    }

    /// 根据固定的坐标清单生成世界坐标点
    /// 坐标清单中x = diagonal_spacing / √2，其中diagonal_spacing = 25mm
    pub fn generate_world_points_from_list(&self) -> Result<Vector<Point3f>, opencv::Error> {
        let x = self.center_distance / (2.0_f32.sqrt()); // x ≈ 17.68mm
        let mut world_points = Vector::<Point3f>::new();

        // 按照坐标清单顺序生成点（序号0在右上角）
        let coordinates = [
            (9.0, 0.0), (9.0, 2.0), (9.0, 4.0), (9.0, 6.0), // 0-3
            (8.0, 1.0), (8.0, 3.0), (8.0, 5.0), (8.0, 7.0), // 4-7
            (7.0, 0.0), (7.0, 2.0), (7.0, 4.0), (7.0, 6.0), // 8-11
            (6.0, 1.0), (6.0, 3.0), (6.0, 5.0), (6.0, 7.0), // 12-15
            (5.0, 0.0), (5.0, 2.0), (5.0, 4.0), (5.0, 6.0), // 16-19
            (4.0, 1.0), (4.0, 3.0), (4.0, 5.0), (4.0, 7.0), // 20-23
            (3.0, 0.0), (3.0, 2.0), (3.0, 4.0), (3.0, 6.0), // 24-27
            (2.0, 1.0), (2.0, 3.0), (2.0, 5.0), (2.0, 7.0), // 28-31
            (1.0, 0.0), (1.0, 2.0), (1.0, 4.0), (1.0, 6.0), // 32-35
            (0.0, 1.0), (0.0, 3.0), (0.0, 5.0), (0.0, 7.0), // 36-39
        ];

        println!("=== 根据固定坐标清单生成世界坐标 ===");
        println!("diagonal spacing = {:.2}mm, 基础单位 x = {:.2}mm", self.center_distance, x);

        for (i, &(col, row)) in coordinates.iter().enumerate() {
            let world_x = col * x;
            let world_y = row * x;
            let world_z = 0.0;
            world_points.push(Point3f::new(world_x, world_y, world_z));
            
            //println!("世界坐标点{}: ({:.1}, {:.1}, {:.1}) mm", i, world_x, world_y, world_z);
        }

        println!("总共生成了 {} 个世界坐标点", world_points.len());
        Ok(world_points)
    }

    /// 3.2.1-3.2.2 Asymmetric Circles Grid 角点检测
    // pub fn detect_corners(&self, image: &Mat) -> Result<(Vector<Vector<Point2f>>, Mat), opencv::Error> {
    //     // 这个函数保留原有的 ArUco 检测逻辑，但对于 circles grid 应该使用 find_asymmetric_circles_grid_points
    //     let mut corners = Vector::<Vector<Point2f>>::new();
    //     let mut ids = Mat::default();
        
    //     // 对于 circles grid，不使用 ArUco 检测
    //     // 应该调用 find_asymmetric_circles_grid_points
    //     Ok((corners, ids))
    // }

    // 3.2.1-3.2.2 Asymmetric Circles Grid 圆心检测
    pub fn find_asymmetric_circles_grid_points(
        &mut self,
        image: &Mat,
        draw_debug_image: bool
    ) -> Result<Vector<Point2f>, opencv::Error> {
        
        // 绘制debug图像
        //=======================================================================
        if draw_debug_image {
            let mut gray_image = Mat::default();
            //imgproc::cvt_color(image, &mut gray_image, COLOR_BGR2GRAY, 0, AlgorithmHint::ALGO_HINT_DEFAULT)?;
            
            let mut gray_image = Mat::default();
            if image.channels() == 3 {
                gray_image = image.clone();
            } else {
                imgproc::cvt_color(
                    &image,
                    &mut gray_image,
                    COLOR_BGR2GRAY,
                    0,
                    AlgorithmHint::ALGO_HINT_DEFAULT
                )?;
            }
            
            let mut keypoints = Vector::new();
            self.detector.detect(&gray_image, &mut keypoints, &Mat::default())?;

            // Draw detected blobs as red circles.
            let mut im_with_keypoints = Mat::default();
            opencv::features2d::draw_keypoints(
                image, 
                &keypoints, 
                &mut im_with_keypoints, 
                opencv::core::Scalar::new(0.0, 255.0, 0.0, 0.0), 
                opencv::features2d::DrawMatchesFlags::DRAW_RICH_KEYPOINTS
            )?;
            imgcodecs::imwrite("im_with_keypoints.png", &im_with_keypoints, &Vector::new())?;
        }
        //=======================================================================

        // 检测圆心
        let mut centers = Vector::<Point2f>::new();

        println!("尝试检测 asymmetric circles grid，模式尺寸: {}x{} (cols x rows)", 
                 self.pattern_size.width, self.pattern_size.height);

        // 第一次尝试：使用基本参数 + 自定义detector（必须提供detector）
        println!("第一次尝试：ASYMMETRIC_GRID + 自定义detector...");
        let result = calib3d::find_circles_grid(
            image, 
            self.pattern_size, 
            &mut centers, 
            CALIB_CB_ASYMMETRIC_GRID, 
            Some(&self.detector),  // 必须提供detector
            calib3d::CirclesGridFinderParameters::default()?
        )?;

        // 如果检测成功且需要debug，绘制检测到的圆心
        if result && draw_debug_image {
            let mut debug_image = image.clone();
            
            // 🔍 新增：输出前10个点的详细信息用于诊断
            println!("\n🔍 圆点检测顺序诊断:");
            println!("=========================");
            if centers.len() >= 10 {
                // 分析第一个点的位置
                let first_pt = centers.get(0).unwrap();
                let image_width = image.cols();
                let image_height = image.rows();
                let cx = image_width as f32 / 2.0;
                let cy = image_height as f32 / 2.0;
                
                let quadrant = if first_pt.x < cx && first_pt.y < cy {
                    "左上"
                } else if first_pt.x >= cx && first_pt.y < cy {
                    "右上"
                } else if first_pt.x < cx && first_pt.y >= cy {
                    "左下"
                } else {
                    "右下"
                };
                
                println!("📍 第一个点位置: ({:.0}, {:.0}) - 位于{}象限", 
                        first_pt.x, first_pt.y, quadrant);
                
                // 输出前4个点的坐标和向量
                println!("\n📊 前4个点坐标:");
                for i in 0..4 {
                    let pt = centers.get(i).unwrap();
                    println!("  点{}: ({:.0}, {:.0})", i, pt.x, pt.y);
                }
                
                // 计算前两个点的向量方向
                let p0 = centers.get(0).unwrap();
                let p1 = centers.get(1).unwrap();
                let vec_x = p1.x - p0.x;
                let vec_y = p1.y - p0.y;
                println!("\n📐 点0->点1的向量: ({:.0}, {:.0})", vec_x, vec_y);
                
                // 判断排列方向
                if vec_y.abs() < 50.0 {
                    println!("  → 水平排列（同一行）");
                } else {
                    println!("  ↓ 垂直排列（同一列）");
                }
                
                // 警告：如果第一个点不在右上角，可能有问题
                if quadrant != "右上" {
                    println!("\n⚠️ 警告: 第一个点不在右上角！");
                    println!("    这可能导致世界坐标对应错误。");
                    println!("    请确保标定板方向一致。");
                }
            }
            println!("=========================\n");
            
            // 绘制检测到的所有圆心
            for (i, center) in centers.iter().enumerate() {
                // 绘制圆心
                imgproc::circle(
                    &mut debug_image,
                    opencv::core::Point::new(center.x as i32, center.y as i32),
                    5,  // 半径
                    opencv::core::Scalar::new(0.0, 0.0, 255.0, 0.0),  // 红色
                    2,  // 线宽
                    imgproc::LINE_8,
                    0
                )?;
                
                // 添加序号和坐标
                let text = format!("{}:({:.0},{:.0})", i, center.x, center.y);
                imgproc::put_text(
                    &mut debug_image,
                    &text,
                    opencv::core::Point::new(center.x as i32 + 10, center.y as i32 + 10),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.4,  // 稍微减小字体避免重叠
                    opencv::core::Scalar::new(0.0, 255.0, 0.0, 0.0),  // 绿色
                    1,    // 线宽
                    imgproc::LINE_8,
                    false
                )?;
                println!("序号{}: 坐标({:.0},{:.0})", i, center.x, center.y);
            }
            // 生成带时间戳和图像信息的文件名
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let expected_points = (self.pattern_size.width * self.pattern_size.height) as usize;
            let success_flag = if centers.len() == expected_points { "SUCCESS" } else { "FAILED" };
            let debug_filename = format!("debug_{}_{}_detected{}_expected{}.png", 
                                       timestamp, success_flag, centers.len(), expected_points);
            
            imgcodecs::imwrite(&debug_filename, &debug_image, &Vector::new())?;
            println!("🔍 已保存圆心检测结果图像：{} (检测到{}个圆心)", debug_filename, centers.len());
        }

        if !result {
            println!("第一次失败，第二次尝试：添加CLUSTERING...");
            let result2 = calib3d::find_circles_grid(
                image, 
                self.pattern_size, 
                &mut centers, 
                CALIB_CB_ASYMMETRIC_GRID | CALIB_CB_CLUSTERING, 
                Some(&self.detector),
                calib3d::CirclesGridFinderParameters::default()?
            )?;
            
            if !result2 {
                println!("第二次失败，第三次尝试：交换行列尺寸...");
                let swapped_size = Size::new(self.pattern_size.height, self.pattern_size.width);
                let result3 = calib3d::find_circles_grid(
                    image, 
                    swapped_size, 
                    &mut centers, 
                    CALIB_CB_ASYMMETRIC_GRID, 
                    Some(&self.detector),
                    calib3d::CirclesGridFinderParameters::default()?
                )?;
                
                if !result3 {
                    println!("第三次失败，第四次尝试：交换尺寸 + CLUSTERING...");
                    let result4 = calib3d::find_circles_grid(
                        image, 
                        swapped_size, 
                        &mut centers, 
                        CALIB_CB_ASYMMETRIC_GRID | CALIB_CB_CLUSTERING, 
                        Some(&self.detector),
                        calib3d::CirclesGridFinderParameters::default()?
                    )?;
                    
                    if !result4 {
                        return Err(opencv::Error::new(
                            opencv::core::StsError,
                            format!("所有尝试都失败了。预期圆点数: {}, 请检查：\n\
                                   1. 图像中是否有清晰的圆点\n\
                                   2. 圆点数量是否为{}列x{}行\n\
                                   3. 是否为asymmetric grid布局（偶数列偏移）", 
                                   self.pattern_size.width * self.pattern_size.height,
                                   self.pattern_size.width, self.pattern_size.height)
                        ));
                    } else {
                        println!("✓ 成功！使用交换尺寸 + CLUSTERING: {}x{}", swapped_size.width, swapped_size.height);
                    }
                } else {
                    println!("✓ 成功！使用交换后的尺寸: {}x{}", swapped_size.width, swapped_size.height);
                }
            } else {
                println!("✓ 成功！使用 ASYMMETRIC_GRID + CLUSTERING");
            }
        } else {
            println!("✓ 成功！使用基本 ASYMMETRIC_GRID");
        }

        println!("检测到的圆心数量: {}", centers.len());

        // 🔧 新增：验证并修正圆点顺序
        if centers.len() == (self.pattern_size.width * self.pattern_size.height) as usize {
            println!("🔧 验证圆点检测顺序...");
            
            // 重新排序圆点以确保与世界坐标对应
            let corrected_centers = self.reorder_asymmetric_circles(&centers)?;
            
            // 检查是否需要修正
            let first_original = centers.get(0).unwrap();
            let first_corrected = corrected_centers.get(0).unwrap();
            
            if (first_original.x - first_corrected.x).abs() > 1.0 || 
               (first_original.y - first_corrected.y).abs() > 1.0 {
                println!("⚠️ 检测到圆点顺序错误，已自动修正");
                println!("   原始第0点: ({:.0}, {:.0})", first_original.x, first_original.y);
                println!("   修正后第0点: ({:.0}, {:.0})", first_corrected.x, first_corrected.y);
                centers = corrected_centers;
            } else {
                println!("✅ 圆点顺序正确，无需修正");
            }
        }

        Ok(centers)
    }
    
    /// 重新排序 asymmetric circles 以匹配世界坐标
    /// 
    /// OpenCV的find_circles_grid可能返回不同的列顺序，
    /// 这个函数确保输出顺序与generate_world_points_from_list一致
    fn reorder_asymmetric_circles(&self, centers: &Vector<Point2f>) -> Result<Vector<Point2f>, opencv::Error> {
        if centers.len() != 40 {
            return Ok(centers.clone());
        }
        
        // 检查序号0和序号4的x坐标
        let point_0 = centers.get(0)?;
        let point_4 = centers.get(4)?;
        
        // 如果序号0的x坐标小于序号4，说明列顺序错了
        // 正确情况：序号0应该在最右边，x坐标应该更大
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

    // 生成对应的 obj/img 点
    pub fn get_image_points_and_obj_points_pairs(
        &mut self,
        image_folder: &str,
        camera_type: CameraType,
    ) -> Result<(Vector<Vector<Point3f>>, Vector<Vector<Point2f>>), opencv::Error> {
        let mut obj_points = Vector::<Vector<Point3f>>::new();
        let mut img_points = Vector::<Vector<Point2f>>::new();
        let single_obj_points = self.generate_world_points_from_list()?;

        // 🔧 优化版本 - 遍历0-19的图像序号（尝试读取更多图像）
        for i in 0..20 {
            // 构建文件名：l_0.bmp 到 l_19.bmp 或 r_0.bmp 到 r_19.bmp
            let file_name = format!("{}_{}.bmp", camera_type.get_prefix(), i);
            let file_path = format!("{}\\{}", image_folder, file_name);

        // 原版本 - 只读取9张图像
        // for i in 0..9 {
        //     // 构建文件名：l_0.bmp 到 l_8.bmp 或 r_0.bmp 到 r_8.bmp
        //     let file_name = format!("{}_{}.bmp", camera_type.get_prefix(), i);
        //     let file_path = format!("{}\\{}", image_folder, file_name);

            // 读取图像
            let img = imgcodecs::imread(&file_path, imgcodecs::IMREAD_COLOR)?;
            if img.empty() {
                println!("Unable to read {}, skipping.", file_path);
                continue;
            }

            match self.find_asymmetric_circles_grid_points(&img, true) {
                Ok(centers) => {
                    if centers.len() == (self.pattern_size.width * self.pattern_size.height) as usize {
                        let centers_len = centers.len();
                        img_points.push(centers);
                        obj_points.push(single_obj_points.clone());
                        println!("Found {} centers in {}", centers_len, file_path);
                    } else {
                        println!("Expected {} circles but found {} in {}", 
                            self.pattern_size.width * self.pattern_size.height, 
                            centers.len(), 
                            file_path);
                    }
                }
                Err(_) => {
                    println!("Asymmetric circle grid NOT found in {}.", file_path);
                }
            }
        }

        Ok((obj_points, img_points))
    }

    /// 3.2.3 单目标定
    pub fn calibrate_mono(
        &self,
        obj_points: &Vector<Vector<Point3f>>,
        img_points: &Vector<Vector<Point2f>>,
    ) -> Result<MonoCalibResult, opencv::Error> {
        let mut camera_matrix = Mat::zeros(3, 3, opencv::core::CV_64F)?.to_mat()?;
        
        // 🔧 优化版本 - 改进的初始估计
        let focal_estimate = self.image_size.width as f64 * 1.2; // 稍微增大焦距估计
        unsafe {
            *camera_matrix.at_mut::<f64>(0)? = focal_estimate; // fx
            *camera_matrix.at_mut::<f64>(4)? = focal_estimate; // fy
            *camera_matrix.at_mut::<f64>(2)? = self.image_size.width as f64 / 2.0; // cx
            *camera_matrix.at_mut::<f64>(5)? = self.image_size.height as f64 / 2.0; // cy
            *camera_matrix.at_mut::<f64>(8)? = 1.0;
        }

        let mut dist_coeffs = Mat::zeros(5, 1, opencv::core::CV_64F)?.to_mat()?;
        let mut rvecs = Vector::<Mat>::new();
        let mut tvecs = Vector::<Mat>::new();

        println!("🔧 开始单目标定，使用 {} 组图像", img_points.len());

        // 🔧 优化版本 - 优化的标定参数
        // 移除 CALIB_FIX_PRINCIPAL_POINT，让算法自由优化主点位置
        // 保留 CALIB_FIX_K3，因为k3通常影响不大
        let error = calib3d::calibrate_camera(
            obj_points,
            img_points,
            self.image_size,
            &mut camera_matrix,
            &mut dist_coeffs,
            &mut rvecs,
            &mut tvecs,
            calib3d::CALIB_FIX_K3 | calib3d::CALIB_USE_INTRINSIC_GUESS,  // 移除 CALIB_FIX_PRINCIPAL_POINT
            TermCriteria::new(
                opencv::core::TermCriteria_COUNT + opencv::core::TermCriteria_EPS,
                100,   // 增加迭代次数从30到100
                1e-8   // 提高精度要求从1e-6到1e-8
            )?,
        )?;

        println!("📊 单目标定结果:");
        println!("  RMS误差: {:.4}", error);
        println!("  Camera Matrix:\n{:?}", camera_matrix);
        println!("  Distortion Coefficients: {:?}", dist_coeffs);

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
        let mut e = Mat::default(); // essential matrix
        let mut f = Mat::default(); // fundamental matrix
        
        // 🔧 优化版本 - 添加per-view误差分析
        let mut per_view_errors = Mat::default();

        // stereo_calibrate 需要可变输入/输出矩阵，先复制一份可变矩阵
        let mut k1 = left_camera.camera_matrix.clone();
        let mut d1 = left_camera.dist_coeffs.clone();
        let mut k2 = right_camera.camera_matrix.clone();
        let mut d2 = right_camera.dist_coeffs.clone();

        println!("🔧 开始双目标定，使用 {} 组图像对", left_points.len());

        // 🔧 优化版本 - 优化的双目标定参数
        // 可以尝试不固定内参，让双目标定进一步优化
        let flags = if left_points.len() >= 15 {
            // 如果图像数量足够多（>=15），可以同时优化内参
            calib3d::CALIB_USE_INTRINSIC_GUESS  // 不固定内参，进一步优化
        } else {
            // 图像较少时，固定内参避免过拟合
            calib3d::CALIB_FIX_INTRINSIC
        };

        // 🔧 优化版本 - 执行标定并获取重投影误差
        let error = calib3d::stereo_calibrate_extended(
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
            &mut e,
            &mut f,
            &mut Vector::<Mat>::new(),  // rvecs (暂不使用)
            &mut Vector::<Mat>::new(),  // tvecs (暂不使用)
            &mut per_view_errors,  // 获取每个视图的误差
            flags,
            TermCriteria::new(
                opencv::core::TermCriteria_COUNT + opencv::core::TermCriteria_EPS,
                100,   // 增加迭代次数
                1e-8   // 提高精度要求
            )?,
        )?;

        println!("📊 双目标定结果:");
        println!("  总体RMS误差: {:.4}", error);
        
        // 🔧 优化版本 - 分析per-view误差，找出异常图像
        if !per_view_errors.empty() {
            println!("  每组图像的误差:");
            for i in 0..per_view_errors.rows() {
                unsafe {
                    let left_err = *per_view_errors.at_2d::<f64>(i, 0)?;
                    let right_err = *per_view_errors.at_2d::<f64>(i, 1)?;
                    println!("    图像对{}: 左={:.3}, 右={:.3}", i, left_err, right_err);
                    
                    // 如果某对图像误差特别大，给出警告
                    if left_err > error * 2.0 || right_err > error * 2.0 {
                        println!("    ⚠️ 图像对{}误差异常大，建议检查或剔除", i);
                    }
                }
            }
        }

        // 判断重投影误差
        if error > self.error_threshold {
            Ok(StereoCalibResult::NeedRecalibration(error))
        } else {
            Ok(StereoCalibResult::Success { r, t, error })
        }
    }

    /// 🔧 优化方案1: 剔除最差的图像对后重新标定
    /// 
    /// 根据per-view误差，剔除误差最大的10-20%图像对，然后重新标定
    pub fn calibrate_stereo_with_outlier_rejection(
        &self,
        obj_points: &Vector<Vector<Point3f>>,
        left_points: &Vector<Vector<Point2f>>,
        right_points: &Vector<Vector<Point2f>>,
        left_camera: &MonoCamera,
        right_camera: &MonoCamera,
        rejection_ratio: f64,  // 剔除比例，如0.2表示剔除最差的20%
    ) -> Result<StereoCalibResult, opencv::Error> {
        println!("🔧 执行带异常值剔除的双目标定...");
        
        // 第一次标定，获取per-view误差
        let mut r = Mat::default();
        let mut t = Mat::default();
        let mut e = Mat::default();
        let mut f = Mat::default();
        let mut per_view_errors = Mat::default();
        
        let mut k1 = left_camera.camera_matrix.clone();
        let mut d1 = left_camera.dist_coeffs.clone();
        let mut k2 = right_camera.camera_matrix.clone();
        let mut d2 = right_camera.dist_coeffs.clone();
        
        // 第一次标定
        let _first_error = calib3d::stereo_calibrate_extended(
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
            &mut e,
            &mut f,
            &mut Vector::<Mat>::new(),
            &mut Vector::<Mat>::new(),
            &mut per_view_errors,
            calib3d::CALIB_FIX_INTRINSIC,
            TermCriteria::new(
                opencv::core::TermCriteria_COUNT + opencv::core::TermCriteria_EPS,
                100,
                1e-8
            )?,
        )?;
        
        // 分析误差，找出需要剔除的图像对
        let mut errors_with_indices: Vec<(usize, f64)> = Vec::new();
        for i in 0..per_view_errors.rows() {
            unsafe {
                let left_err = *per_view_errors.at_2d::<f64>(i, 0)?;
                let right_err = *per_view_errors.at_2d::<f64>(i, 1)?;
                let avg_err = (left_err + right_err) / 2.0;
                errors_with_indices.push((i as usize, avg_err));
            }
        }
        
        // 按误差排序
        errors_with_indices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // 计算需要剔除的数量
        let num_to_reject = ((errors_with_indices.len() as f64) * rejection_ratio) as usize;
        let num_to_reject = num_to_reject.max(1).min(errors_with_indices.len() - 8); // 至少保留8组
        
        println!("  剔除误差最大的 {} 组图像对（共 {} 组）", num_to_reject, errors_with_indices.len());
        
        // 创建保留的图像对索引集合
        let mut indices_to_keep = std::collections::HashSet::new();
        for i in num_to_reject..errors_with_indices.len() {
            indices_to_keep.insert(errors_with_indices[i].0);
        }
        
        // 创建过滤后的点集
        let mut filtered_obj_points = Vector::<Vector<Point3f>>::new();
        let mut filtered_left_points = Vector::<Vector<Point2f>>::new();
        let mut filtered_right_points = Vector::<Vector<Point2f>>::new();
        
        for i in 0..obj_points.len() {
            if indices_to_keep.contains(&i) {
                filtered_obj_points.push(obj_points.get(i)?);
                filtered_left_points.push(left_points.get(i)?);
                filtered_right_points.push(right_points.get(i)?);
            } else {
                println!("  剔除图像对{}: 平均误差={:.3}", i, 
                    errors_with_indices.iter().find(|&&(idx, _)| idx == i).unwrap().1);
            }
        }
        
        println!("  使用 {} 组图像对重新标定", filtered_obj_points.len());
        
        // 使用过滤后的数据重新标定
        self.calibrate_stereo(
            &filtered_obj_points,
            &filtered_left_points,
            &filtered_right_points,
            left_camera,
            right_camera
        )
    }

    /// 🔧 优化方案2: A/B对比测试主点固定策略
    /// 
    /// 对比固定主点和自由主点两种策略，选择误差更小的方案
    pub fn calibrate_mono_with_ab_test(
        &self,
        obj_points: &Vector<Vector<Point3f>>,
        img_points: &Vector<Vector<Point2f>>,
    ) -> Result<MonoCalibResult, opencv::Error> {
        println!("🔧 执行A/B测试：固定主点 vs 自由主点...");
        
        // 方案A：固定主点
        let mut camera_matrix_a = Mat::zeros(3, 3, opencv::core::CV_64F)?.to_mat()?;
        let focal_estimate = self.image_size.width as f64 * 1.2;
        unsafe {
            *camera_matrix_a.at_mut::<f64>(0)? = focal_estimate;
            *camera_matrix_a.at_mut::<f64>(4)? = focal_estimate;
            *camera_matrix_a.at_mut::<f64>(2)? = self.image_size.width as f64 / 2.0;
            *camera_matrix_a.at_mut::<f64>(5)? = self.image_size.height as f64 / 2.0;
            *camera_matrix_a.at_mut::<f64>(8)? = 1.0;
        }
        
        let mut dist_coeffs_a = Mat::zeros(5, 1, opencv::core::CV_64F)?.to_mat()?;
        let mut rvecs_a = Vector::<Mat>::new();
        let mut tvecs_a = Vector::<Mat>::new();
        
        let error_a = calib3d::calibrate_camera(
            obj_points,
            img_points,
            self.image_size,
            &mut camera_matrix_a,
            &mut dist_coeffs_a,
            &mut rvecs_a,
            &mut tvecs_a,
            calib3d::CALIB_FIX_K3 | calib3d::CALIB_FIX_PRINCIPAL_POINT | calib3d::CALIB_USE_INTRINSIC_GUESS,
            TermCriteria::new(
                opencv::core::TermCriteria_COUNT + opencv::core::TermCriteria_EPS,
                100,
                1e-8
            )?,
        )?;
        
        println!("  方案A（固定主点）RMS误差: {:.4}", error_a);
        
        // 方案B：自由主点
        let mut camera_matrix_b = Mat::zeros(3, 3, opencv::core::CV_64F)?.to_mat()?;
        unsafe {
            *camera_matrix_b.at_mut::<f64>(0)? = focal_estimate;
            *camera_matrix_b.at_mut::<f64>(4)? = focal_estimate;
            *camera_matrix_b.at_mut::<f64>(2)? = self.image_size.width as f64 / 2.0;
            *camera_matrix_b.at_mut::<f64>(5)? = self.image_size.height as f64 / 2.0;
            *camera_matrix_b.at_mut::<f64>(8)? = 1.0;
        }
        
        let mut dist_coeffs_b = Mat::zeros(5, 1, opencv::core::CV_64F)?.to_mat()?;
        let mut rvecs_b = Vector::<Mat>::new();
        let mut tvecs_b = Vector::<Mat>::new();
        
        let error_b = calib3d::calibrate_camera(
            obj_points,
            img_points,
            self.image_size,
            &mut camera_matrix_b,
            &mut dist_coeffs_b,
            &mut rvecs_b,
            &mut tvecs_b,
            calib3d::CALIB_FIX_K3 | calib3d::CALIB_USE_INTRINSIC_GUESS,  // 不固定主点
            TermCriteria::new(
                opencv::core::TermCriteria_COUNT + opencv::core::TermCriteria_EPS,
                100,
                1e-8
            )?,
        )?;
        
        println!("  方案B（自由主点）RMS误差: {:.4}", error_b);
        
        // 选择误差更小的方案
        if error_a <= error_b {
            println!("  ✅ 选择方案A（固定主点）");
            if error_a > self.error_threshold {
                Ok(MonoCalibResult::NeedRecalibration(error_a))
            } else {
                Ok(MonoCalibResult::Success {
                    camera_matrix: camera_matrix_a,
                    dist_coeffs: dist_coeffs_a,
                    error: error_a,
                })
            }
        } else {
            println!("  ✅ 选择方案B（自由主点）");
            if error_b > self.error_threshold {
                Ok(MonoCalibResult::NeedRecalibration(error_b))
            } else {
                Ok(MonoCalibResult::Success {
                    camera_matrix: camera_matrix_b,
                    dist_coeffs: dist_coeffs_b,
                    error: error_b,
                })
            }
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

// ============== 为 calibration_workflow.rs 重构新增的函数 ==============
// 这些函数不替代原有函数，而是为新的工作流程提供支持

impl Calibrator {
    /// 从图像路径列表检测和获取特征点 (新增函数)
    /// 
    /// 这个函数是为了配合新的calibration_workflow.rs而添加的，
    /// 它可以从任意的图像路径列表中加载图像并检测特征点
    /// 
    /// # 参数
    /// - `image_paths`: 图像文件路径列表
    /// - `camera_type`: 相机类型（用于日志输出）
    /// 
    /// # 返回值
    /// - `Ok((obj_points, img_points))`: 检测到的世界坐标点和图像坐标点
    /// - `Err(opencv::Error)`: 检测失败
    pub fn detect_and_get_points_from_paths(
        &mut self,
        image_paths: &[String],
        camera_type: CameraType,
    ) -> Result<(Vector<Vector<Point3f>>, Vector<Vector<Point2f>>), opencv::Error> {
        let mut obj_points = Vector::<Vector<Point3f>>::new();
        let mut img_points = Vector::<Vector<Point2f>>::new();
        let single_obj_points = self.generate_world_points_from_list()?;

        println!("🔍 开始从{}张{}相机图像中检测特征点...", 
                image_paths.len(), camera_type.get_prefix());

        for (i, image_path) in image_paths.iter().enumerate() {
            // 读取图像
            let img = imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR)?;
            if img.empty() {
                println!("⚠️ 无法读取图像: {}, 跳过", image_path);
                continue;
            }

            println!("📷 正在处理第 {}/{} 张图像: {}", 
                    i + 1, image_paths.len(), image_path);

            match self.find_asymmetric_circles_grid_points(&img, false) {
                Ok(centers) => {
                    let expected_points = (self.pattern_size.width * self.pattern_size.height) as usize;
                    if centers.len() == expected_points {
                        let centers_len = centers.len();
                        img_points.push(centers);
                        obj_points.push(single_obj_points.clone());
                        println!("✅ 在 {} 中找到 {} 个特征点", image_path, centers_len);
                    } else {
                        println!("⚠️ 预期 {} 个圆点但找到 {} 个，跳过图像: {}", 
                                expected_points, centers.len(), image_path);
                    }
                }
                Err(e) => {
                    println!("❌ 在 {} 中检测asymmetric circle grid失败: {}", image_path, e);
                }
            }
        }

        let valid_images = obj_points.len();
        println!("📊 {}相机特征点检测完成: 成功处理 {}/{} 张图像", 
                camera_type.get_prefix(), valid_images, image_paths.len());

        if valid_images < 8 {
            return Err(opencv::Error::new(
                opencv::core::StsError,
                format!("有效图像数量不足: {}/8，需要至少8张有效图像进行标定", valid_images)
            ));
        }

        Ok((obj_points, img_points))
    }

    /// 快速检测单张图像中是否包含标定板 (新增函数)
    /// 
    /// 用于采集过程中快速验证图像质量，不进行完整的特征点检测
    /// 
    /// # 参数
    /// - `image_data`: 图像数据 (OpenCV Mat)
    /// 
    /// # 返回值
    /// - `true`: 检测到标定板
    /// - `false`: 未检测到标定板
    pub fn quick_detect_calibration_pattern(&mut self, image_data: &Mat) -> bool {
        match self.find_asymmetric_circles_grid_points(image_data, true) {
            Ok(centers) => {
                let expected_points = (self.pattern_size.width * self.pattern_size.height) as usize;
                centers.len() == expected_points
            }
            Err(_) => false
        }
    }

    /// 从临时保存的图像文件检测特征点 (新增函数)
    /// 
    /// 这是一个简化的函数，用于在标定工作流程中检测保存的图像文件
    /// 
    /// # 参数
    /// - `image_path`: 临时图像文件路径
    /// 
    /// # 返回值
    /// - `Ok((has_pattern, feature_count))`: (是否检测到标定板, 特征点数量)
    /// - `Err(opencv::Error)`: 检测失败
    pub fn validate_saved_calibration_image(
        &mut self,
        image_path: &str,
    ) -> Result<(bool, u32), opencv::Error> {
        let img = imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR)?;
        if img.empty() {
            return Ok((false, 0));
        }

        match self.find_asymmetric_circles_grid_points(&img, false) {
            Ok(centers) => {
                let expected_points = (self.pattern_size.width * self.pattern_size.height) as usize;
                let has_pattern = centers.len() == expected_points;
                Ok((has_pattern, centers.len() as u32))
            }
            Err(_) => Ok((false, 0))
        }
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