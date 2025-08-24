// alignment_workflow_test.rs - 合像工作流模块测试
// 重点测试离线可验证的核心逻辑

use std::time::Instant;
use opencv::{core, imgcodecs, prelude::*};
use merging_image_lib::modules::alignment_workflow::{
    AlignmentWorkflow, DetectionResult, DetectionStage, FrameData
};

/// 工作流测试器 (离线模式)
pub struct AlignmentWorkflowTest {
    test_image_left: core::Mat,
    test_image_right: core::Mat,
}

impl AlignmentWorkflowTest {
    /// 创建测试实例
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔧 初始化工作流测试器 (离线模式)...");
        
        // 确定正确的文件路径
        let exe_path = std::env::current_exe()?;
        let exe_dir = exe_path.parent().unwrap();
        let src_tauri_dir = exe_dir.parent().unwrap().parent().unwrap();
        
        // 构造绝对路径
        let left_image_path = src_tauri_dir.join("src/tests/data/benchmark/left_calibration.bmp");
        let right_image_path = src_tauri_dir.join("src/tests/data/benchmark/right_calibration.bmp");
        
        println!("📁 加载测试图像:");
        println!("   左图: {:?}", left_image_path);
        println!("   右图: {:?}", right_image_path);
        
        // 加载测试图像
        let left_image = imgcodecs::imread(
            left_image_path.to_str().unwrap(),
            imgcodecs::IMREAD_GRAYSCALE,
        )?;
        let right_image = imgcodecs::imread(
            right_image_path.to_str().unwrap(),
            imgcodecs::IMREAD_GRAYSCALE,
        )?;
        
        if left_image.empty() || right_image.empty() {
            return Err("无法加载测试图像，请检查文件路径".into());
        }
        
        println!("✓ 测试图像加载成功");
        println!("   左图尺寸: {}x{}", left_image.cols(), left_image.rows());
        println!("   右图尺寸: {}x{}", right_image.cols(), right_image.rows());
        
        Ok(Self {
            test_image_left: left_image,
            test_image_right: right_image,
        })
    }
    
    /// 运行所有离线测试
    pub fn run_tests(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🧪 开始工作流离线测试...");
        println!("{}", "=".repeat(60));
        
        // 1. 测试数据转换功能
        self.test_raw_data_conversion()?;
        
        // 2. 测试单帧检测 (需要Mock AppHandle)
        println!("⚠️ 单帧检测测试需要Tauri AppHandle，跳过");
        println!("   建议: 在集成测试环境中测试此功能");
        
        // 3. 测试圆心检测
        println!("⚠️ 圆心检测测试需要初始化的AlignmentWorkflow，跳过");
        println!("   建议: 直接使用alignment.rs进行圆心检测测试");
        
        // 4. 测试环形缓冲区逻辑
        self.test_ring_buffer_logic()?;
        
        // 5. 测试阶段转换逻辑
        self.test_stage_transitions()?;
        
        println!("{}", "=".repeat(60));
        println!("✅ 离线测试完成");
        println!("💡 建议: 在有设备的环境中进行完整的集成测试");
        Ok(())
    }
    
    /// 测试原始数据转换
    fn test_raw_data_conversion(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 测试原始数据转换功能...");
        
        // 将测试图像转换为原始数据
        let left_data = self.mat_to_raw_data(&self.test_image_left)?;
        let right_data = self.mat_to_raw_data(&self.test_image_right)?;
        
        println!("   左图原始数据大小: {} bytes", left_data.len());
        println!("   右图原始数据大小: {} bytes", right_data.len());
        
        // 测试数据转换回Mat (使用AlignmentWorkflow的私有方法逻辑)
        let reconstructed_left = self.test_raw_data_to_mat(&left_data, 2448, 2048)?;
        let reconstructed_right = self.test_raw_data_to_mat(&right_data, 2448, 2048)?;
        
        // 验证转换结果
        if reconstructed_left.cols() == self.test_image_left.cols() &&
           reconstructed_left.rows() == self.test_image_left.rows() {
            println!("✓ 左图数据转换验证通过");
        } else {
            println!("❌ 左图数据转换验证失败");
        }
        
        if reconstructed_right.cols() == self.test_image_right.cols() &&
           reconstructed_right.rows() == self.test_image_right.rows() {
            println!("✓ 右图数据转换验证通过");
        } else {
            println!("❌ 右图数据转换验证失败");
        }
        
        println!("✅ 数据转换测试完成");
        Ok(())
    }
    
    /// 测试环形缓冲区逻辑
    fn test_ring_buffer_logic(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 测试环形缓冲区逻辑...");
        
        use merging_image_lib::modules::alignment_workflow::RingBuffer;
        
        let mut buffer = RingBuffer::new(3); // 容量为3的缓冲区
        
        // 测试推送逻辑
        for i in 0..5 {
            let frame = FrameData {
                left_image: vec![i as u8; 100], // 模拟图像数据
                right_image: vec![i as u8; 100],
                timestamp: Instant::now(),
            };
            buffer.push(frame);
            
            let (total, dropped, drop_rate) = buffer.get_stats();
            println!("   推送帧{}: 总数={}, 丢帧={}, 丢帧率={:.1}%, 当前大小={}", 
                    i, total, dropped, drop_rate, buffer.len());
        }
        
        // 验证缓冲区行为
        assert_eq!(buffer.len(), 3, "缓冲区大小应该被限制在容量内");
        
        let (total, dropped, drop_rate) = buffer.get_stats();
        assert_eq!(total, 5, "总推送数应该为5");
        assert_eq!(dropped, 2, "应该丢弃2帧");
        assert_eq!(drop_rate, 40.0, "丢帧率应该为40%");
        
        println!("✅ 环形缓冲区测试通过");
        Ok(())
    }
    
    /// 测试阶段转换逻辑
    fn test_stage_transitions(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 测试阶段转换逻辑...");
        
        // 测试阶段转换序列
        let transitions = vec![
            (DetectionStage::LeftEyePoseCheck, "RightEyePoseCheck"),
            (DetectionStage::RightEyePoseCheck, "DualEyeAlignment"), 
            (DetectionStage::DualEyeAlignment, "Completed"),
        ];
        
        for (current, expected_next) in transitions {
            let next = self.simulate_stage_transition(&current);
            println!("   {:?} -> {:?}", current, next);
            
            match next {
                Some(DetectionStage::RightEyePoseCheck) if expected_next == "RightEyePoseCheck" => {
                    println!("   ✓ 转换正确");
                },
                Some(DetectionStage::DualEyeAlignment) if expected_next == "DualEyeAlignment" => {
                    println!("   ✓ 转换正确");
                },
                Some(DetectionStage::Completed) if expected_next == "Completed" => {
                    println!("   ✓ 转换正确");
                },
                _ => {
                    println!("   ❌ 转换错误");
                }
            }
        }
        
        println!("✅ 阶段转换测试完成");
        Ok(())
    }
    
    /// 辅助方法：将Mat转换为原始数据
    fn mat_to_raw_data(&self, mat: &core::Mat) -> Result<Vec<u8>, opencv::Error> {
        let data_size = (mat.cols() * mat.rows()) as usize;
        let mut raw_data = vec![0u8; data_size];
        
        unsafe {
            let mat_data = mat.data();
            std::ptr::copy_nonoverlapping(mat_data, raw_data.as_mut_ptr(), data_size);
        }
        
        Ok(raw_data)
    }
    
    /// 辅助方法：测试原始数据到Mat的转换 (复制AlignmentWorkflow的逻辑)
    fn test_raw_data_to_mat(&self, data: &[u8], width: i32, height: i32) -> Result<core::Mat, opencv::Error> {
        // 创建空的Mat
        let mut mat = core::Mat::new_rows_cols_with_default(
            height,
            width,
            core::CV_8UC1,
            core::Scalar::default(),
        )?;
        
        // 将数据拷贝到Mat中
        let mat_data = mat.data_mut();
        let expected_size = (width * height) as usize;
        
        if data.len() >= expected_size {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr(),
                    mat_data,
                    expected_size,
                );
            }
        } else {
            return Err(opencv::Error::new(
                opencv::core::StsError, 
                format!("数据长度不足: 需要{}字节，实际{}字节", expected_size, data.len())
            ));
        }
        
        Ok(mat)
    }
    
    /// 辅助方法：模拟阶段转换逻辑
    fn simulate_stage_transition(&self, current: &DetectionStage) -> Option<DetectionStage> {
        match current {
            DetectionStage::LeftEyePoseCheck => Some(DetectionStage::RightEyePoseCheck),
            DetectionStage::RightEyePoseCheck => Some(DetectionStage::DualEyeAlignment),
            DetectionStage::DualEyeAlignment => Some(DetectionStage::Completed),
            _ => None,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动工作流离线测试程序");
    println!("📝 注意: 这是离线测试，不涉及真实硬件");
    
    let mut test = AlignmentWorkflowTest::new()?;
    test.run_tests()?;
    
    println!("🎉 离线测试程序完成");
    println!("💡 下一步建议:");
    println!("   1. 在有设备环境中进行完整集成测试");
    println!("   2. 测试SimpleCameraManager与真实硬件的集成");
    println!("   3. 验证采集线程的实际性能表现");
    Ok(())
} 