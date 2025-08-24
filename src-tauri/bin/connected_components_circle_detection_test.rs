// connected_components_circle_detection_test.rs - 连通域圆点检测测试
// 替换SimpleBlobDetector，使用连通域+面积过滤实现更快速、精确的圆点检测

use std::path::Path;
use std::time::Instant;
use opencv::{core, imgcodecs, imgproc, prelude::*};

// 导入核心算法模块
use merging_image_lib::modules::alignment_circles_detection::{ConnectedComponentsDetector, RefineTag};

/// 测试主函数
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动连通域圆点检测测试程序");
    println!("📁 测试目录: C:\\Users\\Y000010\\MVS\\Data\\test_0822\\");
    println!("🎯 测试范围: l_01.bmp~l_06.bmp, r_01.bmp~r_06.bmp");
    println!("🔍 测试内容: 连通域+面积过滤圆点检测");
    println!("{}", "=".repeat(60));
    
    let mut detector = ConnectedComponentsDetector::new();
    let mut results = Vec::new();
    
    // 测试所有图像
    for i in 1..=6 {
        // 测试左图
        let left_path = format!("C:\\Users\\Y000010\\MVS\\Data\\test_0822\\l_{:02}.bmp", i);
        if Path::new(&left_path).exists() {
            let left_name = format!("l_{:02}", i);
            match test_single_image(&mut detector, &left_path, &left_name) {
                Ok((count, time)) => {
                    results.push((left_name, count, time, true));
                }
                Err(e) => {
                    println!("❌ 测试失败 {}: {}", left_name, e);
                    results.push((left_name, 0, 0, false));
                }
            }
        }
        
        // 测试右图
        let right_path = format!("C:\\Users\\Y000010\\MVS\\Data\\test_0822\\r_{:02}.bmp", i);
        if Path::new(&right_path).exists() {
            let right_name = format!("r_{:02}", i);
            match test_single_image(&mut detector, &right_path, &right_name) {
                Ok((count, time)) => {
                    results.push((right_name, count, time, true));
                }
                Err(e) => {
                    println!("❌ 测试失败 {}: {}", right_name, e);
                    results.push((right_name, 0, 0, false));
                }
            }
        }
    }
    
    // 打印测试结果总结
    print_test_summary(&results);
    
    println!("\n🎉 连通域圆点检测测试完成");
    println!("请查看生成的cc_detection_*.png文件查看检测效果");
    
    Ok(())
}

/// 测试单张图像
fn test_single_image(
    detector: &mut ConnectedComponentsDetector,
    image_path: &str,
    image_name: &str,
) -> Result<(usize, u128), Box<dyn std::error::Error>> {
    println!("\n🔍 测试图像: {}", image_name);
    
    // 加载图像
    let image = imgcodecs::imread(image_path, imgcodecs::IMREAD_GRAYSCALE)?;
    if image.empty() {
        return Err(format!("无法加载图像: {}", image_path).into());
    }
    
    println!("   图像尺寸: {}×{}", image.cols(), image.rows());
    
    // 检测圆点
    let start_time = Instant::now();
    let mut centers = detector.detect_circles(&image)?;
    let detection_time = start_time.elapsed();
    
    // 排序
    detector.sort_asymmetric_grid(&mut centers)?;
    
    // 保存debug图像
    let debug_filename = format!("cc_detection_{}_count{}.png", image_name, centers.len());
    detector.save_debug_image(&image, &centers, &debug_filename)?;
    
    let count = centers.len();
    let time_ms = detection_time.as_millis();
    
    println!("   ✅ 检测结果: {} 个圆点, 耗时: {} ms", count, time_ms);
    
    // 评估结果
    if count == 40 {
        println!("   🎯 完美检测!");
    } else if count >= 35 {
        println!("   ✅ 检测良好 (≥87.5%)");
    } else if count >= 25 {
        println!("   ⚠️ 检测一般 (≥62.5%)");
    } else {
        println!("   ❌ 检测不足 (<62.5%)");
    }
    
    Ok((count, time_ms))
}

/// 打印测试结果总结
fn print_test_summary(results: &[(String, usize, u128, bool)]) {
    println!("\n📊 测试结果总结");
    println!("{}", "=".repeat(70));
    println!("图像名称 | 检测数量 | 耗时(ms) | 状态 | 评估");
    println!("{}", "-".repeat(70));
    
    for (name, count, time, success) in results {
        let status = if *success { "✅" } else { "❌" };
        let evaluation = if *count == 40 {
            "完美"
        } else if *count >= 35 {
            "良好"
        } else if *count >= 25 {
            "一般"
        } else {
            "不足"
        };
        
        println!("{:8} | {:8} | {:8} | {:4} | {}", 
                name, count, time, status, evaluation);
    }
    
    // 统计分析
    let successful_tests: Vec<_> = results.iter().filter(|(_, _, _, s)| *s).collect();
    if !successful_tests.is_empty() {
        let total_count: usize = successful_tests.iter().map(|(_, c, _, _)| c).sum();
        let total_time: u128 = successful_tests.iter().map(|(_, _, t, _)| t).sum();
        let avg_count = total_count as f64 / successful_tests.len() as f64;
        let avg_time = total_time as f64 / successful_tests.len() as f64;
        
        let perfect_count = successful_tests.iter().filter(|(_, c, _, _)| *c == 40).count();
        let good_count = successful_tests.iter().filter(|(_, c, _, _)| *c >= 35).count();
        
        println!("\n📈 统计分析:");
        println!("  成功测试: {}/{}", successful_tests.len(), results.len());
        println!("  平均检测: {:.1} 个圆点", avg_count);
        println!("  平均耗时: {:.1} ms", avg_time);
        println!("  完美检测: {} 个图像 (40/40)", perfect_count);
        println!("  良好检测: {} 个图像 (≥35/40)", good_count);
        
        // 性能评估
        if avg_time < 50.0 {
            println!("  性能评估: ✅ 优秀 (<50ms, 目标达成!)");
        } else if avg_time < 80.0 {
            println!("  性能评估: ⚠️ 良好 (<80ms, 优于SBD)");
        } else {
            println!("  性能评估: ❌ 需要优化 (≥80ms)");
        }
        
        // 检测评估
        if avg_count >= 38.0 {
            println!("  检测评估: ✅ 连通域方法优秀");
        } else if avg_count >= 35.0 {
            println!("  检测评估: ⚠️ 连通域方法良好，可进一步优化");
        } else {
            println!("  检测评估: ❌ 连通域方法需要调整参数");
        }
    }
} 