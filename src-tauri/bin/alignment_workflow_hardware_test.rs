// alignment_workflow_hardware_test.rs - 硬件环境下的合像工作流测试
// 适配双标定板测试方案：左右相机分别拍摄独立标定板

use std::time::{Duration, Instant};
use opencv::{core, imgcodecs, prelude::*};
use tauri::{App, AppHandle, Manager};
use merging_image_lib::modules::alignment_workflow::{
    AlignmentWorkflow, DetectionResult, DetectionStage
};

/// 硬件测试配置
pub struct HardwareTestConfig {
    pub use_mock_calibration: bool,    // 是否使用模拟标定参数
    pub test_duration_seconds: u64,    // 测试持续时间
    pub capture_test_images: bool,     // 是否保存测试图像
    pub skip_precision_check: bool,    // 跳过精度验证（因为是模拟场景）
}

impl Default for HardwareTestConfig {
    fn default() -> Self {
        Self {
            use_mock_calibration: true,     // 默认使用模拟参数
            test_duration_seconds: 30,      // 测试30秒
            capture_test_images: true,      // 保存测试图像用于分析
            skip_precision_check: true,     // 跳过精度检查
        }
    }
}

/// 硬件工作流测试器
pub struct AlignmentWorkflowHardwareTest {
    workflow: Option<AlignmentWorkflow>,
    config: HardwareTestConfig,
    test_results: Vec<TestResult>,
    start_time: Option<Instant>,
}

/// 测试结果记录
#[derive(Debug, Clone)]
pub struct TestResult {
    pub timestamp: Instant,
    pub stage: DetectionStage,
    pub result: Option<DetectionResult>,
    pub processing_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

impl AlignmentWorkflowHardwareTest {
    /// 创建硬件测试实例
    pub fn new(config: Option<HardwareTestConfig>) -> Self {
        Self {
            workflow: None,
            config: config.unwrap_or_default(),
            test_results: Vec::new(),
            start_time: None,
        }
    }
    
    /// 初始化工作流（需要真实的AppHandle）
    pub fn initialize_with_app(&mut self, app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 初始化硬件测试工作流...");
        
        // 创建工作流实例
        let mut workflow = AlignmentWorkflow::new(app_handle)?;
        
        if self.config.use_mock_calibration {
            println!("⚠️ 使用模拟标定参数 (适合双标定板测试)");
            self.create_mock_calibration_files()?;
        }
        
        // 初始化合像检测系统
        workflow.initialize_alignment_system()?;
        
        self.workflow = Some(workflow);
        println!("✓ 硬件测试工作流初始化完成");
        Ok(())
    }
    
    /// 创建模拟标定文件（用于双标定板测试）
    fn create_mock_calibration_files(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📝 创建模拟标定参数文件...");
        
        // 创建基本的相机参数（适合测试）
        let mock_camera_matrix = vec![
            vec![2000.0, 0.0, 1224.0],     // fx=2000, cx=1224 (图像中心)
            vec![0.0, 2000.0, 1024.0],     // fy=2000, cy=1024 (图像中心)
            vec![0.0, 0.0, 1.0]
        ];
        
        let mock_dist_coeffs = vec![0.0, 0.0, 0.0, 0.0, 0.0]; // 无畸变
        
        // 创建左相机参数
        let left_params = merging_image_lib::modules::param_io::CameraParams {
            camera_matrix: mock_camera_matrix.clone(),
            dist_coeffs: mock_dist_coeffs.clone(),
        };
        
        // 创建右相机参数
        let right_params = merging_image_lib::modules::param_io::CameraParams {
            camera_matrix: mock_camera_matrix.clone(),
            dist_coeffs: mock_dist_coeffs.clone(),
        };
        
        // 创建双目参数（基线距离设置较大）
        let stereo_params = merging_image_lib::modules::param_io::StereoParams {
            r: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0]
            ],
            t: vec![100.0, 0.0, 0.0], // 100mm基线距离
        };
        
        // 创建校正参数
        let rectify_params = merging_image_lib::modules::param_io::RectifyParams {
            r1: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0]
            ],
            r2: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0]
            ],
            p1: vec![
                vec![2000.0, 0.0, 1224.0, 0.0],
                vec![0.0, 2000.0, 1024.0, 0.0],
                vec![0.0, 0.0, 1.0, 0.0]
            ],
            p2: vec![
                vec![2000.0, 0.0, 1224.0, -200000.0], // 考虑基线
                vec![0.0, 2000.0, 1024.0, 0.0],
                vec![0.0, 0.0, 1.0, 0.0]
            ],
            q: vec![
                vec![1.0, 0.0, 0.0, -1224.0],
                vec![0.0, 1.0, 0.0, -1024.0],
                vec![0.0, 0.0, 0.0, 2000.0],
                vec![0.0, 0.0, -0.01, 0.0]  // 1/基线距离
            ],
        };
        
        // 保存参数文件
        merging_image_lib::modules::param_io::save_camera_params("left_camera_params_mock.yaml", &left_params)?;
        merging_image_lib::modules::param_io::save_camera_params("right_camera_params_mock.yaml", &right_params)?;
        merging_image_lib::modules::param_io::save_stereo_params("stereo_params_mock.yaml", &stereo_params)?;
        merging_image_lib::modules::param_io::save_rectify_params("rectify_params_mock.yaml", &rectify_params)?;
        
        println!("✓ 模拟标定参数文件创建完成");
        println!("   注意：这些参数仅用于功能测试，不保证精度");
        Ok(())
    }
    
    /// 运行硬件测试
    pub fn run_hardware_test(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.workflow.is_none() {
            return Err("工作流未初始化，请先调用 initialize_with_app()".into());
        }
        
        println!("🚀 开始硬件工作流测试");
        println!("📋 测试配置:");
        println!("   使用模拟标定: {}", self.config.use_mock_calibration);
        println!("   测试时长: {}秒", self.config.test_duration_seconds);
        println!("   保存测试图像: {}", self.config.capture_test_images);
        println!("   跳过精度检查: {}", self.config.skip_precision_check);
        println!("{}", "=".repeat(60));
        
        let workflow = self.workflow.as_mut().unwrap();
        self.start_time = Some(Instant::now());
        
        // 1. 启动工作流
        println!("1️⃣ 启动工作流...");
        workflow.start_workflow()?;
        std::thread::sleep(Duration::from_secs(2)); // 等待启动完成
        
        // 2. 测试预览模式
        println!("2️⃣ 测试预览模式 (5秒)...");
        Self::test_preview_mode_static(workflow, 5)?;
        
        // 3. 测试检测流程
        println!("3️⃣ 测试检测流程...");
        Self::test_detection_workflow_static(workflow, &mut self.test_results)?;
        
        // 4. 性能统计
        println!("4️⃣ 性能统计...");
        workflow.print_performance_report();
        
        // 5. 停止工作流
        println!("5️⃣ 停止工作流...");
        workflow.stop_workflow()?;
        
        // 6. 生成测试报告
        self.generate_test_report()?;
        
        println!("{}", "=".repeat(60));
        println!("✅ 硬件工作流测试完成");
        Ok(())
    }
    
    /// 测试预览模式
    fn test_preview_mode_static(
        workflow: &mut AlignmentWorkflow, 
        duration_secs: u64
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let mut preview_count = 0;
        
        while start.elapsed().as_secs() < duration_secs {
            let current_stage = workflow.get_current_stage();
            if matches!(current_stage, DetectionStage::Preview) {
                preview_count += 1;
                println!("   📷 预览帧 #{}", preview_count);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        
        println!("   ✓ 预览模式测试完成，共{}帧", preview_count);
        Ok(())
    }
    
    /// 测试检测工作流
    fn test_detection_workflow_static(
        workflow: &mut AlignmentWorkflow,
        test_results: &mut Vec<TestResult>
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stages = vec![
            ("左眼姿态检测", DetectionStage::LeftEyePoseCheck),
            ("右眼姿态检测", DetectionStage::RightEyePoseCheck),
            ("双眼合像检测", DetectionStage::DualEyeAlignment),
        ];
        
        // 开始检测
        workflow.start_detection()?;
        std::thread::sleep(Duration::from_secs(1));
        
        for (stage_name, expected_stage) in stages {
            println!("   🔍 测试{}...", stage_name);
            let test_start = Instant::now();
            
            // 等待该阶段完成或超时
            let timeout = Duration::from_secs(10);
            let mut stage_completed = false;
            
            while test_start.elapsed() < timeout {
                let current_stage = workflow.get_current_stage();
                if matches!(current_stage, DetectionStage::Completed) {
                    stage_completed = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            
            let processing_time = test_start.elapsed();
            
            if stage_completed {
                println!("   ✓ {} 完成，耗时: {:.1}ms", stage_name, processing_time.as_millis());
                Self::record_test_result_static(test_results, expected_stage, None, processing_time, true, None);
            } else {
                println!("   ⚠️ {} 超时或未完成", stage_name);
                Self::record_test_result_static(test_results, expected_stage, None, processing_time, false, Some("超时".to_string()));
            }
            
            // 进入下一阶段
            if !stage_completed {
                workflow.next_stage()?;
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        
        Ok(())
    }
    
    /// 记录测试结果
    fn record_test_result(
        &mut self,
        stage: DetectionStage,
        result: Option<DetectionResult>,
        processing_time: Duration,
        success: bool,
        error_message: Option<String>,
    ) {
        Self::record_test_result_static(&mut self.test_results, stage, result, processing_time, success, error_message);
    }
    
    /// 静态记录测试结果方法
    fn record_test_result_static(
        test_results: &mut Vec<TestResult>,
        stage: DetectionStage,
        result: Option<DetectionResult>,
        processing_time: Duration,
        success: bool,
        error_message: Option<String>,
    ) {
        let test_result = TestResult {
            timestamp: Instant::now(),
            stage,
            result,
            processing_time_ms: processing_time.as_millis() as u64,
            success,
            error_message,
        };
        
        test_results.push(test_result);
    }
    
    /// 生成测试报告
    fn generate_test_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📊 === 硬件测试报告 ===");
        
        if let Some(start_time) = self.start_time {
            let total_duration = start_time.elapsed();
            println!("⏱️  总测试时间: {:.1}秒", total_duration.as_secs_f64());
        }
        
        println!("📈 测试结果统计:");
        let total_tests = self.test_results.len();
        let successful_tests = self.test_results.iter().filter(|r| r.success).count();
        let success_rate = if total_tests > 0 {
            (successful_tests as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };
        
        println!("   总测试数: {}", total_tests);
        println!("   成功数: {}", successful_tests);
        println!("   成功率: {:.1}%", success_rate);
        
        if !self.test_results.is_empty() {
            let avg_processing_time = self.test_results.iter()
                .map(|r| r.processing_time_ms)
                .sum::<u64>() as f64 / total_tests as f64;
            println!("   平均处理时间: {:.1}ms", avg_processing_time);
        }
        
        // 详细结果
        println!("📋 详细测试结果:");
        for (i, result) in self.test_results.iter().enumerate() {
            let status = if result.success { "✅" } else { "❌" };
            println!("   {}. {} {:?} - {}ms {}", 
                    i + 1, 
                    status, 
                    result.stage, 
                    result.processing_time_ms,
                    result.error_message.as_ref().map_or("", |s| s));
        }
        
        // 建议
        println!("💡 测试建议:");
        if self.config.use_mock_calibration {
            println!("   - 当前使用模拟标定参数，精度结果仅供参考");
            println!("   - 建议获得固定治具后重新进行精确标定");
        }
        if success_rate < 80.0 {
            println!("   - 成功率偏低，检查标定板放置和光照条件");
            println!("   - 确保左右标定板图案完全一致");
        }
        if !self.test_results.is_empty() {
            let avg_time = self.test_results.iter().map(|r| r.processing_time_ms).sum::<u64>() / total_tests as u64;
            if avg_time > 200 {
                println!("   - 处理时间较长，考虑优化图像质量或算法参数");
            }
        }
        
        println!("========================");
        Ok(())
    }
}

// 由于需要真实的Tauri AppHandle，这里提供一个集成测试的示例框架
// 实际使用时需要在Tauri应用上下文中运行

#[cfg(test)]
mod tests {
    use super::*;
    
    // 注意：这个测试需要在真实的Tauri应用环境中运行
    // 这里只是提供测试结构的示例
    
    #[test]
    fn test_hardware_config_creation() {
        let config = HardwareTestConfig::default();
        assert!(config.use_mock_calibration);
        assert_eq!(config.test_duration_seconds, 30);
        assert!(config.capture_test_images);
        assert!(config.skip_precision_check);
    }
    
    #[test]
    fn test_result_recording() {
        let mut test = AlignmentWorkflowHardwareTest::new(None);
        
        test.record_test_result(
            DetectionStage::LeftEyePoseCheck,
            None,
            Duration::from_millis(150),
            true,
            None,
        );
        
        assert_eq!(test.test_results.len(), 1);
        assert!(test.test_results[0].success);
        assert_eq!(test.test_results[0].processing_time_ms, 150);
    }
}

fn main() {
    println!("🚀 硬件工作流测试程序");
    println!("⚠️  注意：此程序需要在Tauri应用环境中运行");
    println!("📝 使用方法:");
    println!("   1. 将此代码集成到你的Tauri应用中");
    println!("   2. 在有AppHandle的上下文中调用测试");
    println!("   3. 确保左右相机已连接并放置好双标定板");
    println!("");
    println!("💡 建议的测试流程:");
    println!("   1. 固定左右相机位置，确保FOV不重合");
    println!("   2. 放置两块相同的标定板");
    println!("   3. 运行测试，观察检测结果");
    println!("   4. 根据测试报告调整硬件布局");
} 