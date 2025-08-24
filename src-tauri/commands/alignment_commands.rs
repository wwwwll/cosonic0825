// alignment_commands.rs - 合像检测相关的Tauri命令
// 为前端提供合像检测功能的统一接口

use tauri::{AppHandle, State, Emitter};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

use crate::modules::alignment_workflow::{AlignmentWorkflow, DetectionStage, DetectionResult};

// ==================== 数据结构定义 ====================

/// 相机实时图像数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraPreviewData {
    pub left_image_base64: String,     // 左相机图像 (Base64编码)
    pub right_image_base64: String,    // 右相机图像 (Base64编码)
    pub timestamp: u64,                // 时间戳 (毫秒)
    pub width: u32,                    // 图像宽度
    pub height: u32,                   // 图像高度
    pub fps: f32,                      // 当前帧率
}

/// 合像检测状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentStatus {
    pub is_camera_active: bool,        // 相机是否激活
    pub current_stage: DetectionStage, // 当前检测阶段
    pub workflow_running: bool,        // 工作流是否运行中
    pub last_update: u64,              // 最后更新时间戳
}

/// 单光机偏差显示数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EyeDeviationDisplay {
    pub eye_name: String,              // "左眼" 或 "右眼"
    pub pose_status: String,           // 姿态状态描述
    pub pose_pass: bool,               // 姿态检测是否通过
    pub roll_adjustment: String,       // Roll调整建议
    pub pitch_adjustment: String,      // Pitch调整建议
    pub yaw_adjustment: String,        // Yaw调整建议
    pub centering_status: Option<String>, // 居中状态 (仅左眼)
    pub centering_pass: Option<bool>,     // 居中检测是否通过 (仅左眼)
    pub centering_adjustment: Option<String>, // 居中调整建议 (仅左眼)
}

/// 合像检测结果显示数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentResultDisplay {
    pub left_eye: EyeDeviationDisplay,   // 左眼偏差数据
    pub right_eye: EyeDeviationDisplay,  // 右眼偏差数据
    pub alignment_status: Option<String>, // 合像状态描述
    pub alignment_pass: Option<bool>,     // 合像检测是否通过
    pub adjustment_hint: Option<String>,  // 调整提示
    pub rms_error: Option<f64>,          // RMS误差
    pub processing_time_ms: u64,         // 处理耗时
}

/// 全局工作流状态管理
pub struct AlignmentWorkflowState {
    pub workflow: Option<AlignmentWorkflow>,
    pub is_active: bool,
    pub last_preview: Option<CameraPreviewData>,
    pub last_result: Option<AlignmentResultDisplay>,
}

impl AlignmentWorkflowState {
    pub fn new() -> Self {
        Self {
            workflow: None,
            is_active: false,
            last_preview: None,
            last_result: None,
        }
    }
}

// ==================== Tauri 命令实现 ====================

/// 启动相机并开始合像检测
#[tauri::command]
pub async fn start_alignment_camera(
    app_handle: AppHandle,
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<AlignmentStatus, String> {
    println!("🚀 启动合像检测相机...");
    
    let mut workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if workflow_state.is_active {
        return Ok(AlignmentStatus {
            is_camera_active: true,
            current_stage: DetectionStage::Preview,
            workflow_running: true,
            last_update: chrono::Utc::now().timestamp_millis() as u64,
        });
    }
    
    // 创建工作流实例
    let mut workflow = AlignmentWorkflow::new(app_handle.clone())
        .map_err(|e| format!("创建工作流失败: {}", e))?;
    
    // 初始化合像检测系统
    workflow.initialize_alignment_system()
        .map_err(|e| format!("初始化检测系统失败: {}", e))?;
    
    // 启动工作流
    workflow.start_workflow()
        .map_err(|e| format!("启动工作流失败: {}", e))?;
    
    workflow_state.workflow = Some(workflow);
    workflow_state.is_active = true;
    
    // 发送状态更新事件
    let _ = app_handle.emit("alignment-camera-started", ());
    
    println!("✓ 合像检测相机启动成功");
    
    Ok(AlignmentStatus {
        is_camera_active: true,
        current_stage: DetectionStage::Preview,
        workflow_running: true,
        last_update: chrono::Utc::now().timestamp_millis() as u64,
    })
}

/// 关闭相机并结束合像检测
#[tauri::command]
pub async fn stop_alignment_camera(
    app_handle: AppHandle,
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<AlignmentStatus, String> {
    println!("🛑 关闭合像检测相机...");
    
    let mut workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if !workflow_state.is_active {
        return Ok(AlignmentStatus {
            is_camera_active: false,
            current_stage: DetectionStage::Idle,
            workflow_running: false,
            last_update: chrono::Utc::now().timestamp_millis() as u64,
        });
    }
    
    // 停止工作流
    if let Some(mut workflow) = workflow_state.workflow.take() {
        workflow.stop_workflow()
            .map_err(|e| format!("停止工作流失败: {}", e))?;
    }
    
    workflow_state.is_active = false;
    workflow_state.last_preview = None;
    workflow_state.last_result = None;
    
    // 发送状态更新事件
    let _ = app_handle.emit("alignment-camera-stopped", ());
    
    println!("✓ 合像检测相机关闭成功");
    
    Ok(AlignmentStatus {
        is_camera_active: false,
        current_stage: DetectionStage::Idle,
        workflow_running: false,
        last_update: chrono::Utc::now().timestamp_millis() as u64,
    })
}

/// 获取当前合像检测状态
#[tauri::command]
pub async fn get_alignment_status(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<AlignmentStatus, String> {
    let workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    let current_stage = if let Some(ref workflow) = workflow_state.workflow {
        workflow.get_current_stage()
    } else {
        DetectionStage::Idle
    };
    
    Ok(AlignmentStatus {
        is_camera_active: workflow_state.is_active,
        current_stage,
        workflow_running: workflow_state.is_active,
        last_update: chrono::Utc::now().timestamp_millis() as u64,
    })
}

/// 获取左右相机实时图像预览
#[tauri::command]
pub async fn get_camera_preview(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<Option<CameraPreviewData>, String> {
    let workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if !workflow_state.is_active {
        return Ok(None);
    }
    
    // 直接从工作流获取当前帧并转换为Base64
    if let Some(ref workflow) = workflow_state.workflow {
        match workflow.get_current_preview_frame() {
            Ok(preview_data) => Ok(Some(preview_data)),
            Err(e) => {
                eprintln!("获取预览帧失败: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// 获取单光机偏差值和调整建议
#[tauri::command]
pub async fn get_alignment_deviation(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<Option<AlignmentResultDisplay>, String> {
    let workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if !workflow_state.is_active {
        return Ok(None);
    }
    
    // 直接从工作流获取当前检测结果
    if let Some(ref workflow) = workflow_state.workflow {
        match workflow.get_current_detection_result() {
            Ok(detection_result) => {
                let display_result = convert_detection_result_to_display(&detection_result);
                Ok(Some(display_result))
            }
            Err(e) => {
                eprintln!("获取检测结果失败: {}", e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// 手动触发单次合像检测
#[tauri::command]
pub async fn trigger_alignment_detection(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<String, String> {
    let mut workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if !workflow_state.is_active {
        return Err("相机未启动".to_string());
    }
    
    if let Some(ref workflow) = workflow_state.workflow {
        workflow.start_detection()
            .map_err(|e| format!("启动检测失败: {}", e))?;
        
        Ok("检测已启动".to_string())
    } else {
        Err("工作流未初始化".to_string())
    }
}

/// 保存调试图像（用于问题排查）
#[tauri::command] 
pub async fn save_debug_images(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<String, String> {
    let workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if let Some(ref workflow) = workflow_state.workflow {
        // 强制保存当前帧的调试图像
        workflow.save_debug_images_manual()
            .map_err(|e| format!("保存调试图像失败: {}", e))?;
        Ok("调试图像已保存到项目根目录".to_string())
    } else {
        Err("工作流未启动".to_string())
    }
}

/// 重置到预览模式
#[tauri::command]
pub async fn reset_to_preview(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<String, String> {
    let workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if !workflow_state.is_active {
        return Err("相机未启动".to_string());
    }
    
    if let Some(ref workflow) = workflow_state.workflow {
        workflow.reset_to_preview()
            .map_err(|e| format!("重置失败: {}", e))?;
        
        Ok("已重置到预览模式".to_string())
    } else {
        Err("工作流未初始化".to_string())
    }
}

/// 获取系统性能统计
#[tauri::command]
pub async fn get_alignment_performance(
    state: State<'_, Arc<Mutex<AlignmentWorkflowState>>>,
) -> Result<Option<serde_json::Value>, String> {
    let workflow_state = state.lock().map_err(|e| format!("状态锁定失败: {}", e))?;
    
    if !workflow_state.is_active {
        return Ok(None);
    }
    
    if let Some(ref workflow) = workflow_state.workflow {
        let stats = workflow.get_performance_stats()
            .map_err(|e| format!("获取性能统计失败: {}", e))?;
        Ok(Some(stats))
    } else {
        Ok(None)
    }
}

// ==================== 辅助函数 ====================

/// 将原始图像数据转换为Base64缩略图
fn create_thumbnail_base64(image_data: &[u8], width: u32, height: u32, thumbnail_size: u32) -> Result<String, String> {
    // 这里应该实现图像缩放和Base64编码
    // 为了简化，暂时返回占位符
    // TODO: 实现真实的图像处理逻辑
    
    let placeholder = format!("data:image/jpeg;base64,/9j/4AAQSkZJRgABAQEAYABgAAD//thumbnail_{}x{}", thumbnail_size, thumbnail_size);
    Ok(placeholder)
}

/// 将检测结果转换为前端显示格式
pub fn convert_detection_result_to_display(result: &DetectionResult) -> AlignmentResultDisplay {
    match result {
        DetectionResult::LeftEyePose { roll, pitch, yaw, pass, message } => {
            AlignmentResultDisplay {
                left_eye: EyeDeviationDisplay {
                    eye_name: "左眼".to_string(),
                    pose_status: message.clone(),
                    pose_pass: *pass,
                    roll_adjustment: format!("Roll: {:.3}°", -roll),
                    pitch_adjustment: format!("Pitch: {:.3}°", -pitch),
                    yaw_adjustment: format!("Yaw: {:.3}°", -yaw),
                    centering_status: None,
                    centering_pass: None,
                    centering_adjustment: None,
                },
                right_eye: EyeDeviationDisplay {
                    eye_name: "右眼".to_string(),
                    pose_status: "等待左眼检测完成".to_string(),
                    pose_pass: false,
                    roll_adjustment: "待检测".to_string(),
                    pitch_adjustment: "待检测".to_string(),
                    yaw_adjustment: "待检测".to_string(),
                    centering_status: None,
                    centering_pass: None,
                    centering_adjustment: None,
                },
                alignment_status: None,
                alignment_pass: None,
                adjustment_hint: None,
                rms_error: None,
                processing_time_ms: 0,
            }
        },
        DetectionResult::RightEyePose { roll, pitch, yaw, pass, message } => {
            AlignmentResultDisplay {
                left_eye: EyeDeviationDisplay {
                    eye_name: "左眼".to_string(),
                    pose_status: "检测完成".to_string(),
                    pose_pass: true,
                    roll_adjustment: "已通过".to_string(),
                    pitch_adjustment: "已通过".to_string(),
                    yaw_adjustment: "已通过".to_string(),
                    centering_status: Some("已通过".to_string()),
                    centering_pass: Some(true),
                    centering_adjustment: Some("无需调整".to_string()),
                },
                right_eye: EyeDeviationDisplay {
                    eye_name: "右眼".to_string(),
                    pose_status: message.clone(),
                    pose_pass: *pass,
                    roll_adjustment: format!("Roll: {:.3}°", -roll),
                    pitch_adjustment: format!("Pitch: {:.3}°", -pitch),
                    yaw_adjustment: format!("Yaw: {:.3}°", -yaw),
                    centering_status: None,
                    centering_pass: None,
                    centering_adjustment: None,
                },
                alignment_status: None,
                alignment_pass: None,
                adjustment_hint: None,
                rms_error: None,
                processing_time_ms: 0,
            }
        },
        DetectionResult::DualEyeAlignment { mean_dx, mean_dy, rms, p95: _, max_err: _, pass, adjustment_hint } => {
            AlignmentResultDisplay {
                left_eye: EyeDeviationDisplay {
                    eye_name: "左眼".to_string(),
                    pose_status: "检测完成".to_string(),
                    pose_pass: true,
                    roll_adjustment: "已通过".to_string(),
                    pitch_adjustment: "已通过".to_string(),
                    yaw_adjustment: "已通过".to_string(),
                    centering_status: Some("已通过".to_string()),
                    centering_pass: Some(true),
                    centering_adjustment: Some("无需调整".to_string()),
                },
                right_eye: EyeDeviationDisplay {
                    eye_name: "右眼".to_string(),
                    pose_status: "检测完成".to_string(),
                    pose_pass: true,
                    roll_adjustment: "已通过".to_string(),
                    pitch_adjustment: "已通过".to_string(),
                    yaw_adjustment: "已通过".to_string(),
                    centering_status: None,
                    centering_pass: None,
                    centering_adjustment: None,
                },
                alignment_status: Some(if *pass { "✓ 合像检测通过".to_string() } else { "❌ 合像精度不足".to_string() }),
                alignment_pass: Some(*pass),
                adjustment_hint: Some(adjustment_hint.clone()),
                rms_error: Some(*rms),
                processing_time_ms: 0,
            }
        },
        DetectionResult::Error { message } => {
            AlignmentResultDisplay {
                left_eye: EyeDeviationDisplay {
                    eye_name: "左眼".to_string(),
                    pose_status: format!("错误: {}", message),
                    pose_pass: false,
                    roll_adjustment: "检测失败".to_string(),
                    pitch_adjustment: "检测失败".to_string(),
                    yaw_adjustment: "检测失败".to_string(),
                    centering_status: Some("检测失败".to_string()),
                    centering_pass: Some(false),
                    centering_adjustment: Some("检测失败".to_string()),
                },
                right_eye: EyeDeviationDisplay {
                    eye_name: "右眼".to_string(),
                    pose_status: format!("错误: {}", message),
                    pose_pass: false,
                    roll_adjustment: "检测失败".to_string(),
                    pitch_adjustment: "检测失败".to_string(),
                    yaw_adjustment: "检测失败".to_string(),
                    centering_status: None,
                    centering_pass: None,
                    centering_adjustment: None,
                },
                alignment_status: Some(format!("❌ 检测错误: {}", message)),
                alignment_pass: Some(false),
                adjustment_hint: Some("请检查设备连接和标定板位置".to_string()),
                rms_error: None,
                processing_time_ms: 0,
            }
        }
    }
} 