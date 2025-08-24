//! 调试标定板圆点检测顺序问题
//! 
//! 用于诊断重投影误差过大的原因

use std::fs;
use opencv::{core::{Size, Vector, Point2f, Point3f, Scalar}, imgcodecs, imgproc, prelude::*};
use merging_image_lib::modules::calibration_circles::*;

fn main() {
    println!("🔍 调试标定板圆点检测顺序");
    println!("=====================================");
    
    // 测试两组图像
    let good_image = r"C:\Users\Y000010\MVS\Data\point_5_4\png\l_0.png";
    let bad_image = r"D:\rust_projects\merging_image\src-tauri\captures\calibration_calibration_1755503179\calib_left_01.png";
    
    println!("\n📸 第一组（成功）: {}", good_image);
    analyze_calibration_image(good_image, "good");
    
    println!("\n📸 第二组（失败）: {}", bad_image);
    analyze_calibration_image(bad_image, "bad");
    
    println!("\n🔬 对比分析");
    compare_detection_order(good_image, bad_image);
}

fn analyze_calibration_image(image_path: &str, label: &str) {
    // 创建标定器
    let mut calibrator = Calibrator::new(
        Size::new(2448, 2048),
        15.0,
        25.0,
        Size::new(4, 10),
        1.0,
    ).expect("创建标定器失败");
    
    // 读取图像
    let img = imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR)
        .expect("读取图像失败");
    
    if img.empty() {
        println!("❌ 图像为空: {}", image_path);
        return;
    }
    
    // 检测圆点
    let centers = calibrator.find_asymmetric_circles_grid_points(&img, false)
        .expect("检测圆点失败");
    
    println!("✅ 检测到 {} 个圆点", centers.len());
    
    // 生成世界坐标
    let world_points = calibrator.generate_world_points_from_list()
        .expect("生成世界坐标失败");
    
    // 创建可视化图像
    let mut vis_img = img.clone();
    
    // 绘制前10个点的顺序和连线
    for i in 0..10.min(centers.len()) {
        let center = centers.get(i).unwrap();
        let world_pt = world_points.get(i).unwrap();
        
        // 绘制圆点
        let color = match i {
            0 => Scalar::new(0.0, 0.0, 255.0, 0.0),     // 第1个点：红色
            1 => Scalar::new(0.0, 255.0, 0.0, 0.0),     // 第2个点：绿色
            2 => Scalar::new(255.0, 0.0, 0.0, 0.0),     // 第3个点：蓝色
            3 => Scalar::new(255.0, 255.0, 0.0, 0.0),   // 第4个点：青色
            _ => Scalar::new(128.0, 128.0, 128.0, 0.0), // 其他：灰色
        };
        
        imgproc::circle(
            &mut vis_img,
            opencv::core::Point::new(center.x as i32, center.y as i32),
            10,
            color,
            -1,
            imgproc::LINE_8,
            0
        ).unwrap();
        
        // 添加序号和世界坐标
        let text = format!("{}:({:.0},{:.0})", i, world_pt.x, world_pt.y);
        imgproc::put_text(
            &mut vis_img,
            &text,
            opencv::core::Point::new(center.x as i32 + 15, center.y as i32 - 10),
            imgproc::FONT_HERSHEY_SIMPLEX,
            0.5,
            color,
            2,
            imgproc::LINE_8,
            false
        ).unwrap();
        
        // 连接前4个点形成参考线
        if i > 0 && i < 4 {
            let prev_center = centers.get(i-1).unwrap();
            imgproc::line(
                &mut vis_img,
                opencv::core::Point::new(prev_center.x as i32, prev_center.y as i32),
                opencv::core::Point::new(center.x as i32, center.y as i32),
                Scalar::new(255.0, 0.0, 255.0, 0.0),
                2,
                imgproc::LINE_8,
                0
            ).unwrap();
        }
    }
    
    // 保存可视化结果
    let output_file = format!("debug_order_{}.png", label);
    imgcodecs::imwrite(&output_file, &vis_img, &Vector::new()).unwrap();
    println!("💾 保存可视化结果: {}", output_file);
    
    // 打印前10个点的详细信息
    println!("\n📊 前10个点的对应关系:");
    println!("序号 | 图像坐标(x,y) | 世界坐标(x,y)");
    println!("-----|---------------|---------------");
    for i in 0..10.min(centers.len()) {
        let center = centers.get(i).unwrap();
        let world_pt = world_points.get(i).unwrap();
        println!("{:3} | ({:4.0},{:4.0}) | ({:4.1},{:4.1})", 
                i, center.x, center.y, world_pt.x, world_pt.y);
    }
}

fn compare_detection_order(good_image: &str, bad_image: &str) {
    // 创建标定器
    let mut calibrator = Calibrator::new(
        Size::new(2448, 2048),
        15.0,
        25.0,
        Size::new(4, 10),
        1.0,
    ).expect("创建标定器失败");
    
    // 检测两组图像的圆点
    let img1 = imgcodecs::imread(good_image, imgcodecs::IMREAD_COLOR).unwrap();
    let img2 = imgcodecs::imread(bad_image, imgcodecs::IMREAD_COLOR).unwrap();
    
    let centers1 = calibrator.find_asymmetric_circles_grid_points(&img1, false).unwrap();
    let centers2 = calibrator.find_asymmetric_circles_grid_points(&img2, false).unwrap();
    
    println!("\n🔬 检测顺序对比:");
    
    // 分析第一个点的位置（应该在右上角）
    let first_pt1 = centers1.get(0).unwrap();
    let first_pt2 = centers2.get(0).unwrap();
    
    println!("第一个点位置:");
    println!("  好图像: ({:.0}, {:.0})", first_pt1.x, first_pt1.y);
    println!("  坏图像: ({:.0}, {:.0})", first_pt2.x, first_pt2.y);
    
    // 判断是否在同一象限
    let quadrant1 = get_quadrant(&first_pt1, 2448, 2048);
    let quadrant2 = get_quadrant(&first_pt2, 2448, 2048);
    
    println!("第一个点所在象限:");
    println!("  好图像: {}", quadrant1);
    println!("  坏图像: {}", quadrant2);
    
    if quadrant1 != quadrant2 {
        println!("\n⚠️ 警告：两组图像的圆点检测顺序可能不同！");
        println!("这可能是因为：");
        println!("1. 标定板方向不同（旋转或镜像）");
        println!("2. OpenCV在不同条件下选择了不同的起始点");
        
        println!("\n💡 建议解决方案：");
        println!("1. 确保所有标定图像中标定板方向一致");
        println!("2. 在标定板上添加明显的方向标记");
        println!("3. 使用ChArUco标定板（带有方向性）");
    } else {
        println!("\n✅ 两组图像的圆点检测顺序看起来一致");
        println!("重投影误差可能由其他因素导致：");
        println!("1. 圆心定位精度（光照/对焦影响）");
        println!("2. 镜头畸变模型不适用");
        println!("3. 相机参数初始估计不准确");
    }
    
    // 计算前4个点形成的向量方向
    if centers1.len() >= 4 && centers2.len() >= 4 {
        println!("\n📐 前4个点的排列方向分析:");
        
        let vec1 = compute_first_vector(&centers1);
        let vec2 = compute_first_vector(&centers2);
        
        println!("第0->1点向量:");
        println!("  好图像: ({:.0}, {:.0})", vec1.0, vec1.1);
        println!("  坏图像: ({:.0}, {:.0})", vec2.0, vec2.1);
        
        // 计算向量夹角
        let angle = compute_angle(vec1, vec2);
        println!("向量夹角: {:.1}°", angle);
        
        if angle > 90.0 {
            println!("⚠️ 标定板可能旋转了180°或镜像!");
        }
    }
}

fn get_quadrant(pt: &Point2f, width: i32, height: i32) -> &'static str {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    
    if pt.x < cx && pt.y < cy {
        "左上"
    } else if pt.x >= cx && pt.y < cy {
        "右上"
    } else if pt.x < cx && pt.y >= cy {
        "左下"
    } else {
        "右下"
    }
}

fn compute_first_vector(centers: &Vector<Point2f>) -> (f32, f32) {
    let p0 = centers.get(0).unwrap();
    let p1 = centers.get(1).unwrap();
    (p1.x - p0.x, p1.y - p0.y)
}

fn compute_angle(v1: (f32, f32), v2: (f32, f32)) -> f32 {
    let dot = v1.0 * v2.0 + v1.1 * v2.1;
    let mag1 = (v1.0 * v1.0 + v1.1 * v1.1).sqrt();
    let mag2 = (v2.0 * v2.0 + v2.1 * v2.1).sqrt();
    let cos_angle = dot / (mag1 * mag2);
    cos_angle.acos().to_degrees()
} 