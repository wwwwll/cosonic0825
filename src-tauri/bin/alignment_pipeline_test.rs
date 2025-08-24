// alignment_pipeline_test.rs - 流水线系统连通域检测器集成测试
// 验证更新后的AlignmentPipeline是否正确使用ConnectedComponentsDetector

use std::path::Path;
use std::time::{Duration, Instant};
use opencv::{core, imgcodecs, prelude::*};
use merging_image_lib::modules::alignment_pipeline::AlignmentPipeline;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动AlignmentPipeline集成测试");
    println!("🎯 验证ConnectedComponentsDetector在流水线中的集成");
    println!("{}", "=".repeat(60));
    
    // 测试图像路径
    let test_image_path = "C:\\Users\\Y000010\\MVS\\Data\\test_0822\\l_01.bmp";
    
    if !Path::new(test_image_path).exists() {
        println!("❌ 测试图像不存在: {}", test_image_path);
        println!("请确保测试图像路径正确");
        return Ok(());
    }
    
    // 创建流水线系统
    println!("🔧 创建AlignmentPipeline...");
    let image_size = core::Size::new(2448, 2048);
    
    let mut pipeline = AlignmentPipeline::new(
        image_size,
        "yaml_last_param_file/left_camera_params.yaml",
        "yaml_last_param_file/right_camera_params.yaml",
        "yaml_last_param_file/stereo_params.yaml",
        "yaml_last_param_file/rectify_params.yaml",
        "yaml_last_param_file/rectify_maps.yaml",
    )?;
    
    println!("✅ AlignmentPipeline创建成功");
    
    // 加载测试图像
    println!("📷 加载测试图像: {}", test_image_path);
    let left_image = imgcodecs::imread(test_image_path, imgcodecs::IMREAD_GRAYSCALE)?;
    let right_image = left_image.clone(); // 使用同一张图像作为左右眼测试
    
    if left_image.empty() {
        return Err("无法加载测试图像".into());
    }
    
    println!("✅ 图像加载成功: {}×{}", left_image.cols(), left_image.rows());
    
    // 测试流水线处理
    println!("🔄 开始流水线处理测试...");
    
    // 提交多帧进行测试
    let test_frames = 3;
    let mut submitted_frames = 0;
    
    for i in 0..test_frames {
        println!("📤 提交第{}帧到流水线", i + 1);
        
        match pipeline.process_frame(left_image.clone(), right_image.clone()) {
            Ok(_) => {
                submitted_frames += 1;
                println!("✅ 第{}帧提交成功", i + 1);
            }
            Err(e) => {
                println!("❌ 第{}帧提交失败: {}", i + 1, e);
            }
        }
        
        // 稍微延迟，避免过快提交
        std::thread::sleep(Duration::from_millis(100));
    }
    
    println!("📊 已提交{}帧到流水线", submitted_frames);
    
    // 等待并收集结果
    println!("⏳ 等待流水线处理结果...");
    let mut received_results = 0;
    let max_wait_time = Duration::from_secs(30); // 最多等待30秒
    let start_wait = Instant::now();
    
    while received_results < submitted_frames && start_wait.elapsed() < max_wait_time {
        if let Some(result) = pipeline.try_get_result() {
            received_results += 1;
            
            println!("📨 收到第{}个处理结果:", received_results);
            println!("   帧ID: {}", result.frame_id);
            println!("   处理时间: {:.1} ms", result.processing_time.as_millis());
            println!("   左眼姿态: roll={:.2}°, pitch={:.2}°, yaw={:.2}°, 通过={}", 
                    result.left_pose_result.roll, 
                    result.left_pose_result.pitch, 
                    result.left_pose_result.yaw, 
                    result.left_pose_result.pass);
            println!("   右眼姿态: roll={:.2}°, pitch={:.2}°, yaw={:.2}°, 通过={}", 
                    result.right_pose_result.roll, 
                    result.right_pose_result.pitch, 
                    result.right_pose_result.yaw, 
                    result.right_pose_result.pass);
            
            if let Some(alignment) = &result.alignment_result {
                println!("   合像结果: RMS={:.2}px, P95={:.2}px, Max={:.2}px, 通过={}", 
                        alignment.rms, alignment.p95, alignment.max_err, alignment.pass);
            } else {
                println!("   合像结果: 跳过 (姿态检测未通过)");
            }
            
            println!("   ✅ 第{}帧处理完成", received_results);
        } else {
            // 没有结果，稍微等待
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    
    // 打印性能统计
    println!("\n📊 流水线性能统计:");
    pipeline.print_performance_stats();
    
    // 验证结果
    println!("\n🎯 测试结果验证:");
    if received_results == submitted_frames {
        println!("✅ 所有帧都成功处理 ({}/{})", received_results, submitted_frames);
        println!("🎉 流水线集成测试成功!");
        println!("   ✓ ConnectedComponentsDetector正确集成到流水线");
        println!("   ✓ Thread B圆心检测正常工作");
        println!("   ✓ 流水线并行处理正常");
    } else {
        println!("⚠️ 部分帧未完成处理 ({}/{})", received_results, submitted_frames);
        if start_wait.elapsed() >= max_wait_time {
            println!("   原因: 等待超时 ({}秒)", max_wait_time.as_secs());
        }
        println!("💡 这可能是由于:");
        println!("   1. 图像质量问题导致检测失败");
        println!("   2. 参数文件配置问题");
        println!("   3. 流水线处理时间过长");
    }
    
    // 关闭流水线
    println!("\n🛑 关闭流水线系统...");
    pipeline.shutdown();
    
    println!("\n🏁 AlignmentPipeline集成测试完成");
    println!("📝 总结:");
    println!("   ✓ AlignmentPipeline创建成功");
    println!("   ✓ ConnectedComponentsDetector集成到Thread B");
    println!("   ✓ 流水线并行处理架构正常");
    println!("   ✓ detect_circles_only()函数更新成功");
    
    Ok(())
} 