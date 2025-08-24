// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod camera_ffi;
pub mod camera_manager;
pub mod config;
pub mod commands {
    pub mod config_commands;
    pub mod calibration_commands;
    pub mod alignment_commands;
}
pub mod modules {
    pub mod calibration;
    pub mod calibration_circles;
    pub mod rectification;
    pub mod merging_check;
    pub mod param_io;
    pub mod alignment;
    pub mod alignment_workflow;
    pub mod alignment_pipeline;
    // pub mod camera_workflow;
    pub mod calibration_workflow;
    pub mod simple_config;  // 添加simple_config模块
    pub mod alignment_circles_detection;  // 🆕 连通域圆点检测核心算法模块
}

//pub use config::simple_config;

pub use modules::calibration;

// 导入假的CameraManager用于编译兼容
use crate::camera_manager::CameraManager;
// 导出新的SimpleCameraManager供测试使用
pub use crate::camera_manager::{SimpleCameraManager, CameraError};
// 导出连通域圆点检测核心算法模块
pub use crate::modules::alignment_circles_detection::{ConnectedComponentsDetector, RefineTag};
use crate::camera_ffi::CameraHandle;
use crate::modules::alignment_workflow::{AlignmentWorkflow, WorkflowCommand, DetectionStage};
use crate::config::{ConfigManager, CompatibilityManager};
use crate::commands::{config_commands, calibration_commands, alignment_commands};
use tauri::{Manager, State};
use tauri_plugin_opener;
use std::sync::{Arc, Mutex};
//use std::sync::Mutex;

// FFI test
//pub mod camera_manager_test;

#[cfg(test)]
mod tests {
    //mod calibration_test;
    mod rectification_test;
    mod merging_check_test;
    mod calibration_test_new;
    mod calibration_circles_test;
    mod alignment_test;
}


#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// ==================== 假的相机控制命令，请使用SimpleCameraManager ====================
#[tauri::command]
fn start_cam(state: State<'_, CameraManager>) -> Result<(), String> {
  state.start_preview()
       .map_err(|e| format!("start failed: 0x{:x}", e))
}

#[tauri::command]
fn stop_cam(state: State<'_, CameraManager>) -> Result<(), String> {
  state.stop_preview()
       .map_err(|e| format!("stop failed: 0x{:x}", e))
}

#[tauri::command]
fn capture_cam(state: State<'_, CameraManager>) -> Result<(String,String), String> {
  state.capture_frame()
       .map_err(|e| format!("capture failed: 0x{:x}", e))
}

// ==================== 合像检测工作流程命令 ====================

/// 初始化合像检测系统
#[tauri::command]
async fn init_alignment_system(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<(), String> {
    let mut workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref mut workflow) = *workflow_guard {
        workflow.initialize_alignment_system()
            .map_err(|e| format!("初始化合像检测系统失败: {}", e))?;
        Ok(())
    } else {
        Err("工作流程未创建".to_string())
    }
}

/// 启动合像检测工作流程
#[tauri::command]
async fn start_alignment_workflow(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<(), String> {
    let mut workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref mut workflow) = *workflow_guard {
        workflow.start_workflow()
            .map_err(|e| format!("启动工作流程失败: {}", e))?;
        Ok(())
    } else {
        Err("工作流程未创建".to_string())
    }
}

/// 开始检测
#[tauri::command]
async fn start_detection(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<(), String> {
    let workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref workflow) = *workflow_guard {
        workflow.start_detection()
            .map_err(|e| format!("开始检测失败: {}", e))?;
        Ok(())
    } else {
        Err("工作流程未创建".to_string())
    }
}

/// 进入下一阶段
#[tauri::command]
async fn next_detection_stage(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<(), String> {
    let workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref workflow) = *workflow_guard {
        workflow.next_stage()
            .map_err(|e| format!("进入下一阶段失败: {}", e))?;
        Ok(())
    } else {
        Err("工作流程未创建".to_string())
    }
}

/// 重置到预览模式
#[tauri::command]
async fn reset_alignment_workflow(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<(), String> {
    let workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref workflow) = *workflow_guard {
        workflow.reset_to_preview()
            .map_err(|e| format!("重置工作流程失败: {}", e))?;
        Ok(())
    } else {
        Err("工作流程未创建".to_string())
    }
}

/// 停止合像检测工作流程
#[tauri::command]
async fn stop_alignment_workflow(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<(), String> {
    let mut workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref mut workflow) = *workflow_guard {
        workflow.stop_workflow()
            .map_err(|e| format!("停止工作流程失败: {}", e))?;
        Ok(())
    } else {
        Err("工作流程未创建".to_string())
    }
}

/// 获取当前检测阶段
#[tauri::command]
async fn get_current_stage(
    workflow_state: State<'_, Arc<Mutex<Option<AlignmentWorkflow>>>>
) -> Result<DetectionStage, String> {
    let workflow_guard = workflow_state.lock()
        .map_err(|e| format!("获取工作流程状态失败: {}", e))?;
    
    if let Some(ref workflow) = *workflow_guard {
        Ok(workflow.get_current_stage())
    } else {
        Ok(DetectionStage::Idle)
    }
}

/// 程序入口：注册插件、初始化 CameraManager 并管理全局状态，绑定所有命令
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 外部打开 URL 等功能，可选
        .plugin(tauri_plugin_opener::init())
        // 在 setup 阶段用 AppHandle 初始化 CameraManager 并注入到 State
        .setup(|app| {
            let handle = app.handle();
            
            // 初始化CameraManager (保持向后兼容)
            let manager = CameraManager::new(handle.clone())
                .expect("failed to initialize CameraManager");
            app.manage(manager);
            
            // 初始化AlignmentWorkflow（真正的懒加载，启动时不创建任何实例）
            //let alignment_workflow = Arc::new(Mutex::new(None::<AlignmentWorkflow>));
            
            // 🚀 注释掉启动时的自动创建，改为完全懒加载
            // 旧的启动时创建逻辑 (注释掉):
            // {
            //     let mut workflow_guard = alignment_workflow.lock().unwrap();
            //     match AlignmentWorkflow::new(handle.clone()) {
            //         Ok(workflow) => {
            //             *workflow_guard = Some(workflow);
            //             println!("✓ AlignmentWorkflow 创建成功 (SimpleCameraManager版本)");
            //         }
            //         Err(e) => {
            //             eprintln!("❌ AlignmentWorkflow 创建失败: {}", e);
            //             // 可以选择继续运行或返回错误
            //         }
            //     }
            // }
            
            // println!("✓ AlignmentWorkflow 懒加载容器创建成功（实例将在点击启动相机时创建）");
            // app.manage(alignment_workflow);
            
            // 初始化配置管理器
            let config_manager = ConfigManager::new();
            println!("✓ ConfigManager 创建成功");
            app.manage(Arc::new(Mutex::new(config_manager)));
            
            // 初始化兼容性管理器
            let compatibility_manager = CompatibilityManager::new("configs");
            println!("✓ CompatibilityManager 创建成功");
            app.manage(Arc::new(Mutex::new(compatibility_manager)));
            
            // 初始化标定工作流程状态管理器
            let calibration_workflow_state: Arc<Mutex<Option<crate::modules::calibration_workflow::CalibrationWorkflow>>> = Arc::new(Mutex::new(None));
            println!("✓ CalibrationWorkflowState 创建成功");
            app.manage(calibration_workflow_state);
            
            // 初始化合像检测状态管理器
            let alignment_state = Arc::new(Mutex::new(alignment_commands::AlignmentWorkflowState::new()));
            println!("✓ AlignmentWorkflowState 创建成功");
            app.manage(alignment_state);
            
            Ok(())
        })
        // 绑定所有命令
        .invoke_handler(tauri::generate_handler![
            // 基础命令
            greet,
            
            // 相机控制命令
            start_cam,
            stop_cam,
            capture_cam,
            
            // 合像检测工作流程命令
            init_alignment_system,
            start_alignment_workflow,
            start_detection,
            next_detection_stage,
            reset_alignment_workflow,
            stop_alignment_workflow,
            get_current_stage,
            
            // 标定工作流程命令 (SimpleCameraManager架构)
            calibration_commands::start_calibration_session,
            calibration_commands::capture_calibration_image,
            calibration_commands::get_captured_images,
            calibration_commands::delete_captured_image,
            calibration_commands::run_calibration_process,
            calibration_commands::get_calibration_status,
            calibration_commands::stop_calibration_session,
            calibration_commands::reset_calibration_workflow,
            calibration_commands::get_calibration_config,
            calibration_commands::get_preview_frame,
            calibration_commands::get_latest_captured_image,
            
            // 合像检测命令
            alignment_commands::start_alignment_camera,
            alignment_commands::stop_alignment_camera,
            alignment_commands::get_alignment_status,
            alignment_commands::get_camera_preview,
            alignment_commands::get_alignment_deviation,
            alignment_commands::trigger_alignment_detection,
            alignment_commands::reset_to_preview,
            alignment_commands::save_debug_images,
            alignment_commands::get_alignment_performance,
            
            // 配置管理命令
            config_commands::get_system_config,
            config_commands::set_system_config,
            config_commands::get_camera_config,
            config_commands::set_camera_config,
            config_commands::get_camera_serial,
            config_commands::get_alignment_config,
            config_commands::set_alignment_config,
            config_commands::save_config_to_file,
            config_commands::load_config_from_file,
            config_commands::save_config_to_default_dir,
            config_commands::list_config_files,
            config_commands::validate_all_configs,
            config_commands::generate_config_report,
            config_commands::get_effective_pattern_params,
            config_commands::get_effective_camera_serials,
            config_commands::should_use_legacy_implementations,
            config_commands::get_camera_preview_for_roi,
            config_commands::apply_roi_config,
            config_commands::list_config_presets,
            config_commands::list_builtin_presets,
            config_commands::list_user_presets,
            config_commands::get_config_preset,
            config_commands::apply_config_preset,
            config_commands::save_config_preset,
            config_commands::generate_compatibility_report,
            config_commands::load_current_hardware_config,
            config_commands::reset_to_default_config,
            config_commands::export_config_to_json,
            config_commands::import_config_from_json,
            
            // 简单配置管理命令（新增）
            config_commands::read_config_file,
            config_commands::write_config_file,
            config_commands::get_current_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


// #[tauri::command]
// async fn start_cam(app: tauri::AppHandle) -> Result<(), String> {
//   // 保证全局只 new 一次，可用 OnceCell/Mutex 管理
//   let mgr = CameraManager::new(app.clone())
//     .map_err(|e| format!("camera init failed: 0x{:x}", e))?;
//   mgr.start_preview()
//     .map_err(|e| format!("start preview failed: 0x{:x}", e))?;
//   Ok(())
// }

// #[tauri::command]
// async fn stop_cam(state: tauri::State<'_, CameraManager>) -> Result<(), String> {
//   state.stop_preview()
//     .map_err(|e| format!("stop preview failed: 0x{:x}", e))
// }

// #[tauri::command]
// fn capture(state: State<'_, Mutex<CameraManager>>) -> Result<(String,String), String> {
//   let mgr = state.inner().lock().unwrap();
//   mgr.capture_frame()
//      .map_err(|e| format!("capture failed: 0x{:x}", e))
// }

// fn main() {
//   tauri::Builder::default()
//     .manage(CameraManager::new(/*AppHandle*/app.handle()).unwrap())
//     .invoke_handler(tauri::generate_handler![start_cam, stop_cam, capture_cam])
//     .run(tauri::generate_context!())
//     .expect("error while running tauri application");
// }
// fn main() {
//     tauri::Builder::default()
//         .manage(Mutex::new(CameraManager::new(/* app handle placeholder */).unwrap()))
//         .invoke_handler(tauri::generate_handler![capture /*, start_cam, stop_cam */])
//         .run(tauri::generate_context!())
//         .expect("error while running tauri app");
// }

// fn main() {
//     if let Err(code) = camera_init_ffi() {
//     eprintln!("camera_init failed with error code: {code}");
// } else {
//     println!("camera_init success!");
// }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::camera_manager::CameraManager;
//     use tauri::AppHandle;

//     fn mock_app_handle() -> AppHandle {
//         // 你可以通过 tauri::test 或其他方式模拟 AppHandle，
//         // 如果你不使用 emit，则传空值也可
//         unimplemented!("AppHandle mock depends on Tauri setup");
//     }

//     #[test]
//     fn test_camera_lifecycle() {
//         let app_handle = mock_app_handle();
//         let manager = CameraManager::new(app_handle).expect("camera init failed");

//         manager.start_preview().expect("start failed");

//         std::thread::sleep(std::time::Duration::from_millis(500)); // 模拟运行一会

//         let (left, right) = manager.capture_frame().expect("capture failed");
//         assert!(!left.is_empty());
//         assert!(!right.is_empty());

//         manager.stop_preview().expect("stop failed");
//     }
// }
