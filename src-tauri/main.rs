// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    merging_image_lib::run()
}

// #![cfg_attr(
//   all(not(debug_assertions), target_os = "windows"),
//   windows_subsystem = "windows"
// )]

// mod camera_manager;
// use camera_manager::CameraManager;

// fn main() {
//   tauri::Builder::default()
//     // 先 setup 拿到 AppHandle，再 new 出 state
//     .setup(|app| {
//       let handle = app.handle();
//       // 如果 new() 失败，直接 panic（或改成日志+return Err(...)
//       let cam_mgr = CameraManager::new(handle)
//         .expect("failed to initialize CameraManager");
//       app.manage(cam_mgr);
//       Ok(())
//     })
//     // 注册命令
//     .invoke_handler(tauri::generate_handler![
//       start_cam,
//       stop_cam,
//       capture_cam
//     ])
//     .run(tauri::generate_context!())
//     .expect("error while running tauri application");
// }
