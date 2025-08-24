use merging_image::camera_manager::CameraManager;
use crate::camera_ffi::*;
pub mod camera_ffi;
pub mod camera_manager;

fn main() {
    let dummy_handle = unsafe { std::mem::zeroed() }; // 如果你不发事件
    let manager = CameraManager::new(dummy_handle).expect("init failed");

    manager.start_preview().expect("start preview");
    std::thread::sleep(std::time::Duration::from_secs(2));

    for i in 0..3 {
        println!("Capturing image {}", i + 1);
        let (l, r) = manager.capture_frame().expect("capture failed");
        println!("Captured {} bytes left, {} bytes right", l.len(), r.len());
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    manager.stop_preview().expect("stop failed");
}
