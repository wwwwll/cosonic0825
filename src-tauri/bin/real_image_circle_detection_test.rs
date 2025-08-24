// real_image_circle_detection_test.rs - 最小化SimpleBlobDetector测试
// 专门测试C:\Users\Y000010\MVS\Data\test_0822\目录下的实际图像
// 只测试SimpleBlobDetector的检测效果，不进行圆点网格组织

use std::path::Path;
use std::time::Instant;
use opencv::{core, imgcodecs, imgproc, features2d, prelude::*};

/// SimpleBlobDetector测试器
pub struct SimpleBlobDetectorTest {
    test_image_dir: String,
}

impl SimpleBlobDetectorTest {
    /// 创建测试实例
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔧 初始化SimpleBlobDetector测试器...");
        
        let test_image_dir = r"C:\Users\Y000010\MVS\Data\test_0822".to_string();
        
        // 检查测试目录是否存在
        if !Path::new(&test_image_dir).exists() {
            return Err(format!("测试图像目录不存在: {}", test_image_dir).into());
        }
        
        println!("📁 测试图像目录: {}", test_image_dir);
        
        Ok(Self {
            test_image_dir,
        })
    }
    
    /// 创建优化的SimpleBlobDetector (参考alignment.rs的create_optimized_blob_detector)
    fn create_optimized_blob_detector() -> Result<core::Ptr<features2d::Feature2D>, opencv::Error> {
        let mut blob_params = features2d::SimpleBlobDetector_Params::default()?;
        
        // 实际光机投影环境优化 - 基于实测数据
        // 参考alignment.rs第376-407行的参数设置
        
        // 阈值设置 - 适应"发虚"到过曝的亮度范围
        blob_params.min_threshold = 20.0;   // 降低以捕获"发虚"圆点
        blob_params.max_threshold = 150.0;  // 适应过曝圆点
        blob_params.threshold_step = 10.0;  // 大步长提升性能，小步长适应虚化圆点
        
        // 关闭颜色筛选 - 圆点亮度差异太大
        //blob_params.filter_by_color = false;
        blob_params.filter_by_color = true;
        blob_params.blob_color = 255;
        
        // 面积过滤 - 基于实测数据（直径67-90px）
        blob_params.filter_by_area = true;
        blob_params.min_area = 3000.0;   // π*(67/2)² ≈ 3525, 留余量
        blob_params.max_area = 7000.0;   // π*(90/2)² ≈ 6362, 留余量
        
        // 关闭所有形状筛选器 - 最大化性能
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
        
        let detector = features2d::SimpleBlobDetector::create(blob_params)?;
        Ok(detector.into())
    }
    
    /// 测试单张图像的blob检测
    fn test_single_image(
        &self,
        image_path: &str,
        image_name: &str,
    ) -> Result<(usize, u128, u128), Box<dyn std::error::Error>> {
        println!("\n🔍 测试图像: {}", image_name);
        
        // 加载图像
        let image = imgcodecs::imread(image_path, imgcodecs::IMREAD_GRAYSCALE)?;
        if image.empty() {
            return Err(format!("无法加载图像: {}", image_path).into());
        }
        
        println!("   图像尺寸: {}×{}", image.cols(), image.rows());
        
        // 检查图像统计信息
        let mut min_val = 0.0;
        let mut max_val = 0.0;
        core::min_max_loc(
            &image,
            Some(&mut min_val),
            Some(&mut max_val),
            None,
            None,
            &core::no_array(),
        )?;
        println!("   灰度范围: [{:.0}, {:.0}]", min_val, max_val);
        
        // 创建detector
        let detector_start = Instant::now();
        let mut detector = Self::create_optimized_blob_detector()?;
        let detector_creation_time = detector_start.elapsed();
        
        // 检测keypoints (参考calibration_circles.rs第225行)
        let detection_start = Instant::now();
        let mut keypoints = core::Vector::new();
        detector.detect(&image, &mut keypoints, &core::Mat::default())?;
        let detection_time = detection_start.elapsed();
        
        let blob_count = keypoints.len();
        println!("   ✅ SimpleBlobDetector检测到 {} 个blob", blob_count);
        println!("   ⏱️  Detector创建耗时: {:.1} ms", detector_creation_time.as_millis());
        println!("   ⏱️  圆点检测耗时: {:.1} ms", detection_time.as_millis());
        
        // 绘制并保存debug图像 (参考calibration_circles.rs第228-236行)
        let mut im_with_keypoints = core::Mat::default();
        features2d::draw_keypoints(
            &image, 
            &keypoints, 
            &mut im_with_keypoints, 
            core::Scalar::new(0.0, 255.0, 0.0, 0.0), // 绿色
            features2d::DrawMatchesFlags::DRAW_RICH_KEYPOINTS
        )?;
        
        // 添加文字信息
        let mut result_image = im_with_keypoints.clone();
        
        // 添加统计信息
        let text_info = vec![
            format!("SimpleBlobDetector Results"),
            format!("Detected blobs: {}", blob_count),
            format!("Expected: 40 (10x4 grid)"),
            format!("Image: {}", image_name),
        ];
        
        for (i, text) in text_info.iter().enumerate() {
            imgproc::put_text(
                &mut result_image,
                text,
                core::Point::new(10, 30 + i as i32 * 25),
                imgproc::FONT_HERSHEY_SIMPLEX,
                0.7,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0), // 绿色
                2,
                imgproc::LINE_8,
                false,
            )?;
        }
        
        // 如果检测到的blob数量接近预期，标记部分blob的序号
        if blob_count > 0 {
            println!("   前10个blob的位置:");
            for i in 0..std::cmp::min(10, blob_count) {
                let kp = keypoints.get(i)?;
                let pt = kp.pt();
                let size = kp.size();
                println!("     Blob {}: ({:.0}, {:.0}), size={:.1}", i, pt.x, pt.y, size);
                
                // 在图像上标记序号
                let text = format!("{}", i);
                imgproc::put_text(
                    &mut result_image,
                    &text,
                    core::Point::new(pt.x as i32 + 10, pt.y as i32 - 10),
                    imgproc::FONT_HERSHEY_SIMPLEX,
                    0.5,
                    core::Scalar::new(0.0, 0.0, 255.0, 0.0), // 红色
                    1,
                    imgproc::LINE_8,
                    false,
                )?;
            }
        }
        
        // 保存结果图像
        let output_filename = format!("blob_detection_{}_count{}.png", image_name, blob_count);
        imgcodecs::imwrite(&output_filename, &result_image, &core::Vector::<i32>::new())?;
        println!("   💾 已保存debug图像: {}", output_filename);
        
        Ok((blob_count, detector_creation_time.as_millis(), detection_time.as_millis()))
    }
    
    /// 运行所有测试
    pub fn run_tests(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 开始SimpleBlobDetector测试");
        println!("{}", "=".repeat(60));
        
        let mut results = Vec::new();
        
        // 测试所有图像
        for i in 1..=6 {
            // 测试左图
                         let left_path = format!("{}\\l_{:02}.bmp", self.test_image_dir, i);
             if Path::new(&left_path).exists() {
                 let left_name = format!("l_{:02}", i);
                 match self.test_single_image(&left_path, &left_name) {
                     Ok((count, creation_time, detection_time)) => {
                         results.push((left_name, count, creation_time, detection_time, true));
                     }
                     Err(e) => {
                         println!("❌ 测试失败: {}", e);
                         results.push((left_name, 0, 0, 0, false));
                     }
                 }
             }
             
             // 测试右图
             let right_path = format!("{}\\r_{:02}.bmp", self.test_image_dir, i);
             if Path::new(&right_path).exists() {
                 let right_name = format!("r_{:02}", i);
                 match self.test_single_image(&right_path, &right_name) {
                     Ok((count, creation_time, detection_time)) => {
                         results.push((right_name, count, creation_time, detection_time, true));
                     }
                     Err(e) => {
                         println!("❌ 测试失败: {}", e);
                         results.push((right_name, 0, 0, 0, false));
                     }
                 }
             }
        }
        
                 // 打印总结
         println!("\n📊 测试结果总结");
         println!("{}", "=".repeat(80));
         println!("图像名称 | 检测数量 | 创建耗时 | 检测耗时 | 状态 | 评估");
         println!("{}", "-".repeat(80));
         
         for (name, count, creation_time, detection_time, success) in &results {
             let status = if *success { "✅" } else { "❌" };
             let evaluation = if *count >= 40 {
                 "识别成功"
             } else {
                 "需要调优"
             };
             
             println!("{:8} | {:8} | {:8}ms | {:8}ms | {:4} | {}", 
                     name, count, creation_time, detection_time, status, evaluation);
         }
         
         // 统计分析
         let successful_tests: Vec<_> = results.iter().filter(|(_, _, _, _, s)| *s).collect();
         if !successful_tests.is_empty() {
             let total_blobs: usize = successful_tests.iter().map(|(_, c, _, _, _)| c).sum();
             let avg_blobs = total_blobs as f64 / successful_tests.len() as f64;
             let min_blobs = successful_tests.iter().map(|(_, c, _, _, _)| c).min().unwrap();
             let max_blobs = successful_tests.iter().map(|(_, c, _, _, _)| c).max().unwrap();
             
             // 计算平均耗时
             let total_creation_time: u128 = successful_tests.iter().map(|(_, _, ct, _, _)| ct).sum();
             let total_detection_time: u128 = successful_tests.iter().map(|(_, _, _, dt, _)| dt).sum();
             let avg_creation_time = total_creation_time as f64 / successful_tests.len() as f64;
             let avg_detection_time = total_detection_time as f64 / successful_tests.len() as f64;
             let avg_total_time = avg_creation_time + avg_detection_time;
             
             println!("\n📈 统计分析:");
             println!("  成功测试: {}/{}", successful_tests.len(), results.len());
             println!("  平均检测: {:.1} 个blob", avg_blobs);
             println!("  检测范围: {} - {} 个blob", min_blobs, max_blobs);
             println!("  期望数量: 40 个 (10×4 asymmetric grid)");
             
             println!("\n⏱️ 性能分析:");
             println!("  平均创建耗时: {:.1} ms", avg_creation_time);
             println!("  平均检测耗时: {:.1} ms", avg_detection_time);
             println!("  平均总耗时: {:.1} ms", avg_total_time);
             
             if avg_total_time < 50.0 {
                 println!("  性能评估: ✅ 性能优秀 (<50ms)");
             } else if avg_total_time < 100.0 {
                 println!("  性能评估: ⚠️ 性能一般 (<100ms)");
             } else {
                 println!("  性能评估: ❌ 性能需要优化 (>100ms)");
             }
             
             if avg_blobs >= 35.0 {
                 println!("  检测评估: ✅ SimpleBlobDetector参数优秀");
             } else if avg_blobs >= 25.0 {
                 println!("  检测评估: ⚠️ SimpleBlobDetector参数良好，可进一步优化");
             } else {
                 println!("  检测评估: ❌ SimpleBlobDetector参数需要调整");
             }
         }
        
        println!("\n🎉 SimpleBlobDetector测试完成");
        println!("请查看生成的blob_detection_*.png文件查看检测效果");
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动SimpleBlobDetector最小化测试程序");
    println!("📁 测试目录: C:\\Users\\Y000010\\MVS\\Data\\test_0822\\");
    println!("🎯 测试范围: l_01.bmp~l_06.bmp, r_01.bmp~r_06.bmp");
    println!("🔍 测试内容: 仅SimpleBlobDetector blob检测");
    println!("🖼️ Debug输出: blob_detection_*.png");
    
    let mut test = SimpleBlobDetectorTest::new()?;
    test.run_tests()?;
    
    println!("\n🎉 测试程序完成");
    Ok(())
} 