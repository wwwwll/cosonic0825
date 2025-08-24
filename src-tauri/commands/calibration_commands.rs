//! 标定工作流程 Tauri 命令接口
//! 
//! 基于SimpleCameraManager的简化标定流程接口
//! 
//! ## 🎯 API 设计
//! 
//! 简化的命令接口，支持完整的标定工作流程：
//! 1. `start_calibration_session()` - 开始标定会话
//! 2. `capture_calibration_image()` - 拍摄标定图像
//! 3. `get_captured_images()` - 获取已采集图像列表
//! 4. `delete_captured_image(pair_id)` - 删除指定图像对
//! 5. `run_calibration_process()` - 执行标定算法
//! 6. `get_calibration_status()` - 获取标定状态
//! 7. `get_preview_frame()` - 获取实时预览帧
//! 
//! ## 🏗️ 架构分层
//! 
//! ```
//! Frontend (Svelte) → Commands (Tauri) → Workflow (Business) → Circles (Algorithm)
//! ```
//! 
//! - **数据结构定义**: 在 `calibration_workflow.rs` 中定义业务数据结构
//! - **命令接口实现**: 在 `calibration_commands.rs` 中实现 Tauri 命令
//! - **依赖方向**: Commands 依赖 Workflow，而非反向依赖
//! 
//! @version 2.1 - 架构优化版本
//! @date 2025-01-15

use tauri::State;
use std::sync::{Arc, Mutex};
use crate::modules::calibration_workflow::{
    CalibrationWorkflow, 
    CalibrationStatus, 
    CalibrationResult, 
    ImagePair,
    PreviewFrame
};

/// 标定工作流程管理器状态
pub type CalibrationWorkflowState = Arc<Mutex<Option<CalibrationWorkflow>>>;

/// 开始标定会话
/// 
/// 启动相机并开始标定图像采集会话
/// 
/// # 返回值
/// - `Ok(session_id)`: 成功启动，返回会话ID
/// - `Err(String)`: 启动失败的错误信息
#[tauri::command]
pub async fn start_calibration_session(
    state: State<'_, CalibrationWorkflowState>
) -> Result<String, String> {
    println!("🎬 Tauri命令: start_calibration_session");
    
    let mut workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    // 如果没有实例，创建新实例
    if workflow_guard.is_none() {
        let workflow = CalibrationWorkflow::new()?;
        *workflow_guard = Some(workflow);
    }
    
    // 启动标定会话
    if let Some(workflow) = workflow_guard.as_mut() {
        workflow.start_calibration()?;
        Ok("calibration_session_started".to_string())
    } else {
        Err("无法创建标定工作流程".to_string())
    }
}

/// 保存当前帧为标定图像
/// 
/// 从缓冲区读取当前帧并保存为标定图像对
/// 
/// # 返回值
/// - `Ok(ImagePair)`: 成功保存的图像对信息
/// - `Err(String)`: 保存失败的错误信息
#[tauri::command]
pub async fn capture_calibration_image(
    state: State<'_, CalibrationWorkflowState>
) -> Result<ImagePair, String> {
    println!("💾 Tauri命令: capture_calibration_image (保存当前帧)");
    
    let mut workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_mut() {
        workflow.save_current_frame_as_calibration()
    } else {
        Err("标定会话未启动".to_string())
    }
}

/// 获取已拍摄的图像列表
/// 
/// 返回当前会话中所有已采集的图像对信息
/// 
/// # 返回值
/// - `Ok(Vec<ImagePair>)`: 图像对列表
/// - `Err(String)`: 获取失败的错误信息
#[tauri::command]
pub async fn get_captured_images(
    state: State<'_, CalibrationWorkflowState>
) -> Result<Vec<ImagePair>, String> {
    println!("📋 Tauri命令: get_captured_images");
    
    let workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_ref() {
        Ok(workflow.get_captured_images())
    } else {
        Err("标定会话未启动".to_string())
    }
}

/// 删除指定的图像对
/// 
/// 删除指定ID的图像对及其文件
/// 
/// # 参数
/// - `pair_id`: 要删除的图像对ID
/// 
/// # 返回值
/// - `Ok(())`: 删除成功
/// - `Err(String)`: 删除失败的错误信息
#[tauri::command]
pub async fn delete_captured_image(
    pair_id: u32,
    state: State<'_, CalibrationWorkflowState>
) -> Result<(), String> {
    println!("🗑️ Tauri命令: delete_captured_image({})", pair_id);
    
    let mut workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_mut() {
        workflow.delete_captured_image(pair_id)
    } else {
        Err("标定会话未启动".to_string())
    }
}

/// 执行标定算法
/// 
/// 停止相机采集，加载已保存的图像，执行完整的标定流程
/// 
/// # 返回值
/// - `Ok(CalibrationResult)`: 标定结果
/// - `Err(String)`: 标定失败的错误信息
#[tauri::command]
pub async fn run_calibration_process(
    state: State<'_, CalibrationWorkflowState>
) -> Result<CalibrationResult, String> {
    println!("🚀 Tauri命令: run_calibration_process");
    
    let mut workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_mut() {
        workflow.run_calibration()
    } else {
        Err("标定会话未启动".to_string())
    }
}

/// 获取当前标定状态
/// 
/// 返回标定工作流程的当前状态
/// 
/// # 返回值
/// - `Ok(CalibrationStatus)`: 当前标定状态
/// - `Err(String)`: 获取失败的错误信息
#[tauri::command]
pub async fn get_calibration_status(
    state: State<'_, CalibrationWorkflowState>
) -> Result<CalibrationStatus, String> {
    println!("📊 Tauri命令: get_calibration_status");
    
    let workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_ref() {
        Ok(workflow.get_status())
    } else {
        // 如果没有工作流程实例，返回未开始状态
        Ok(CalibrationStatus::NotStarted)
    }
}

/// 停止标定会话
/// 
/// 停止相机采集并清理所有资源
/// 
/// # 返回值
/// - `Ok(())`: 停止成功
/// - `Err(String)`: 停止失败的错误信息
#[tauri::command]
pub async fn stop_calibration_session(
    state: State<'_, CalibrationWorkflowState>
) -> Result<(), String> {
    println!("⏹️ Tauri命令: stop_calibration_session");
    
    let mut workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_mut() {
        workflow.stop_calibration()?;
    }
    
    // 清理工作流程实例
    *workflow_guard = None;
    
    Ok(())
}

/// 重置标定工作流程
/// 
/// 强制重置标定状态，清理所有数据（紧急情况使用）
/// 
/// # 返回值
/// - `Ok(())`: 重置成功
/// - `Err(String)`: 重置失败的错误信息
#[tauri::command]
pub async fn reset_calibration_workflow(
    state: State<'_, CalibrationWorkflowState>
) -> Result<(), String> {
    println!("🔄 Tauri命令: reset_calibration_workflow");
    
    let mut workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    // 如果有现有工作流程，尝试停止
    if let Some(workflow) = workflow_guard.as_mut() {
        let _ = workflow.stop_calibration(); // 忽略错误，强制重置
    }
    
    // 清理工作流程实例
    *workflow_guard = None;
    
    println!("✅ 标定工作流程已重置");
    Ok(())
}

/// 获取标定配置信息
/// 
/// 返回当前标定配置参数（用于前端显示）
/// 
/// # 返回值
/// - `Ok(config_info)`: 配置信息的JSON字符串
/// - `Err(String)`: 获取失败的错误信息
#[tauri::command]
pub async fn get_calibration_config(
    _state: State<'_, CalibrationWorkflowState>
) -> Result<String, String> {
    println!("⚙️ Tauri命令: get_calibration_config");
    
    // 返回默认配置信息
    let config_info = serde_json::json!({
        "circle_diameter": 15.0,
        "center_distance": 25.0,
        "pattern_size": {"width": 10, "height": 4},
        "error_threshold": 2.0,
        "target_image_count": 10,
        "image_resolution": {"width": 2448, "height": 2048}
    });
    
    Ok(config_info.to_string())
} 



/// 获取实时预览帧
/// 
/// 从相机获取当前帧生成预览，可选择同时保存为标定图像
/// 
/// **✅ 即时处理架构优势**：
/// - 统一接口，通过参数控制保存
/// - 前端简单，无需管理两个不同命令
/// - 性能优化，按需获取最新帧
/// 
/// # 参数
/// - `should_save`: 是否同时保存当前帧为标定图像
/// 
/// # 返回值
/// - `Ok(PreviewFrame)`: 包含左右相机Base64图像的预览帧
/// - `Err(String)`: 获取失败的错误信息
#[tauri::command]
pub async fn get_preview_frame(
    should_save: Option<bool>,
    state: State<'_, CalibrationWorkflowState>
) -> Result<PreviewFrame, String> {
    let should_save = should_save.unwrap_or(false);
    println!("🎥 Tauri命令: get_preview_frame(should_save={})", should_save);
    
    // 修复Send问题：分离锁的获取和异步调用
    let frame_result = {
        let mut workflow_guard = state.lock()
            .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
        
        // 检查是否有工作流实例
        let workflow = match workflow_guard.as_mut() {
            Some(wf) => wf,
            None => {
                // 创建临时工作流实例用于预览
                println!("💡 创建临时工作流实例用于预览");
                let temp_workflow = CalibrationWorkflow::new()?;
                *workflow_guard = Some(temp_workflow);
                workflow_guard.as_mut().unwrap()
            }
        };
        
        // 检查相机状态 - 修正：预览不需要严格的标定会话检查
        // 用户场景：点击"启动相机"后即可预览，无需完整标定会话
        if workflow.get_status() == crate::modules::calibration_workflow::CalibrationStatus::NotStarted {
            // 自动启动相机用于预览
            println!("💡 自动启动相机用于预览");
            workflow.start_calibration()?;
        }
        
        // 同步获取预览帧（传入should_save参数）
        workflow.get_preview_frame_sync(should_save)
    };
    
    // 处理结果
    match frame_result {
        Ok(frame) => {
            if should_save {
                println!("✅ 预览帧获取成功，同时保存了标定图像");
            } else {
                println!("✅ 预览帧获取成功");
            }
            Ok(frame)
        }
        Err(e) => {
            println!("❌ 预览帧获取失败: {}", e);
            Err(format!("获取预览帧失败: {}", e))
        }
    }
}

/// 获取最新保存的标定图像信息
/// 
/// 返回最近一次保存的标定图像对信息（配合get_preview_frame使用）
/// 
/// # 返回值
/// - `Ok(Some(ImagePair))`: 最新的图像对信息
/// - `Ok(None)`: 暂无保存的图像
/// - `Err(String)`: 获取失败的错误信息
#[tauri::command]
pub async fn get_latest_captured_image(
    state: State<'_, CalibrationWorkflowState>
) -> Result<Option<ImagePair>, String> {
    println!("📸 Tauri命令: get_latest_captured_image");
    
    let workflow_guard = state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(workflow) = workflow_guard.as_ref() {
        Ok(workflow.get_latest_captured_image())
    } else {
        Ok(None)
    }
} 