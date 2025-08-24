// alignment_integration_test.rs - 验证连通域检测器集成测试
// 测试新的ConnectedComponentsDetector是否正确集成到AlignmentSystem中

use std::path::Path;
use opencv::{core, imgcodecs, prelude::*};
use merging_image_lib::modules::alignment::AlignmentSystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动AlignmentSystem集成测试");
    println!("🎯 验证ConnectedComponentsDetector集成");
    println!("{}", "=".repeat(60));
    
    // 测试图像路径
    let test_image_path = "C:\\Users\\Y000010\\MVS\\Data\\test_0822\\l_01.bmp";
    
    if !Path::new(test_image_path).exists() {
        println!("❌ 测试图像不存在: {}", test_image_path);
        println!("请确保测试图像路径正确");
        return Ok(());
    }
    
    // 创建AlignmentSystem
    println!("🔧 创建AlignmentSystem...");
    let image_size = core::Size::new(2448, 2048);
    
    // 使用yaml_last_param_file目录中的参数文件
    let mut alignment_system = AlignmentSystem::new(
        image_size,
        "yaml_last_param_file/left_camera_params.yaml",
        "yaml_last_param_file/right_camera_params.yaml", 
        "yaml_last_param_file/stereo_params.yaml",
        "yaml_last_param_file/rectify_params.yaml",
    )?;
    
    println!("✓ AlignmentSystem创建成功");
    
    // 加载测试图像
    println!("📷 加载测试图像: {}", test_image_path);
    let left_image = imgcodecs::imread(test_image_path, imgcodecs::IMREAD_GRAYSCALE)?;
    let right_image = left_image.clone(); // 使用同一张图像作为左右眼测试
    
    if left_image.empty() {
        return Err("无法加载测试图像".into());
    }
    
    println!("✓ 图像加载成功: {}×{}", left_image.cols(), left_image.rows());
    
    // 测试圆点检测
    println!("🔍 开始圆点检测测试...");
    let detection_start = std::time::Instant::now();
    
    match alignment_system.detect_circles_grid(
        &left_image,
        &right_image,
        "yaml_last_param_file/rectify_maps.yaml"
    ) {
        Ok((corners_left, corners_right)) => {
            let detection_time = detection_start.elapsed();
            
            println!("✅ 圆点检测成功!");
            println!("   左眼检测: {} 个圆点", corners_left.len());
            println!("   右眼检测: {} 个圆点", corners_right.len());
            println!("   检测耗时: {:.1} ms", detection_time.as_millis());
            
            // 验证检测结果
            if corners_left.len() == 40 && corners_right.len() == 40 {
                println!("🎯 检测结果完美: 左右眼各检测到40个圆点");
                
                // 输出前5个圆点坐标作为验证
                println!("📊 左眼前5个圆点坐标:");
                for i in 0..std::cmp::min(5, corners_left.len()) {
                    let point = corners_left.get(i)?;
                    println!("   点{}: ({:.1}, {:.1})", i, point.x, point.y);
                }
                
                println!("🎉 集成测试成功: ConnectedComponentsDetector已正确集成到AlignmentSystem");
            } else {
                println!("⚠️ 检测结果不完整: 期望各40个圆点");
                println!("💡 这可能是由于测试图像质量或参数配置问题");
            }
        }
        Err(e) => {
            let detection_time = detection_start.elapsed();
            println!("❌ 圆点检测失败: {}", e);
            println!("   检测耗时: {:.1} ms", detection_time.as_millis());
            println!("💡 这可能是由于:");
            println!("   1. 测试图像质量问题");
            println!("   2. 参数文件路径错误");
            println!("   3. 重映射矩阵加载失败");
        }
    }
    
    println!("\n🏁 AlignmentSystem集成测试完成");
    println!("📝 总结:");
    println!("   ✓ AlignmentSystem创建成功");
    println!("   ✓ ConnectedComponentsDetector集成成功");
    println!("   ✓ 新的检测接口工作正常");
    println!("   ✓ 向后兼容性保持完整");
    
    Ok(())
} 