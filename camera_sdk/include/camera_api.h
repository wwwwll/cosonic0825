/**
 * @file camera_api.h
 * @brief Camera API Header for dual-camera system
 * 
 * Enhanced version with trigger mode and precise timing control
 * 
 * @version 2.0
 * @date 2025/01/15
 * @author Enhanced for alignment workflow
 */

#ifndef CAMERA_API_H
#define CAMERA_API_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Camera constants
#define CAMERA_NUM 2
#define TIMEOUT_MS 1000

// 相机序列号 - 写死配置
// #define LEFT_CAMERA_SERIAL "L17065426"
// #define RIGHT_CAMERA_SERIAL "L17065458"
// #define LEFT_CAMERA_SERIAL "00D12345678"
// #define RIGHT_CAMERA_SERIAL "00D12345679"
// #define LEFT_CAMERA_SERIAL "DA5158733"
// #define RIGHT_CAMERA_SERIAL "DA5158736"
#define LEFT_CAMERA_SERIAL "DA6869958"
#define RIGHT_CAMERA_SERIAL "DA6869956"
// #define LEFT_CAMERA_SERIAL "DA4347673"
// #define RIGHT_CAMERA_SERIAL "DA4347675"

// [配置系统 - 已注释] 相机序列号配置相关代码
// #define DEFAULT_LEFT_CAMERA_SERIAL "DA4347673"
// #define DEFAULT_RIGHT_CAMERA_SERIAL "DA4347675"
// extern char g_left_camera_serial[32];
// extern char g_right_camera_serial[32];
// const char* get_left_camera_serial(void);
// const char* get_right_camera_serial(void);

// Camera position enumeration
typedef enum {
    LEFT_CAM = 0,
    RIGHT_CAM = 1,
    UNINITIALIZED = -1
} CameraPosition;

// Trigger mode enumeration
typedef enum {
    TRIGGER_OFF = 0,        // Continuous acquisition
    TRIGGER_SOFTWARE = 1,   // Software trigger
    TRIGGER_HARDWARE = 2    // Hardware trigger (if supported)
} TriggerMode;

// TODO: 配置系统暂时禁用，避免编译问题
// Camera configuration structure
/*
typedef struct CameraConfigStruct {
    bool configured;                    // Configuration status
    float acquisition_frame_rate;       // Acquisition frame rate
    bool frame_rate_enable;            // Frame rate control enable
    int trigger_mode;                  // Trigger mode (0=Off, 1=On)
    int acquisition_mode;              // Acquisition mode (0=SingleFrame, 2=Continuous)
} CameraConfig;
*/

// Camera structure
typedef struct {
    void* handle;              // Camera handle
    char serial[64];           // Serial number
    uint8_t opened;           // Open status
    CameraPosition position;   // Camera position
    TriggerMode trigger_mode; // Current trigger mode
    uint32_t frame_rate;      // Target frame rate (fps)
} Camera;

// Global variables
extern Camera cameras[CAMERA_NUM];
extern uint32_t g_frame_buf_size;

// ==================== 原有底层API ====================

// === Original API functions ===
int camera_init();
// camera_set_param and camera_set_all_param are deprecated - use config system instead
int camera_start();
int camera_get_frame(uint8_t* out_bufs[], uint32_t out_sizes[]);
uint32_t camera_get_frame_buf_size();
int camera_release();

// === Configuration API ===
// [配置系统 - 已注释]
// /**
//  * @brief Set camera working mode for configuration selection
//  * @param mode Working mode: 0=default, 1=calibration, 2=alignment
//  */
// void set_camera_mode(int mode);

// === Enhanced API functions for alignment workflow ===

// 已删除触发模式相关函数 - 新架构下只使用连续采集

/**
 * @brief Get camera status and performance metrics
 * @param cam_index Camera index
 * @param fps_actual Actual frame rate output
 * @param frames_dropped Dropped frames count output
 * @return Error code (0=success)
 */
int camera_get_status(unsigned int cam_index, float* fps_actual, uint32_t* frames_dropped);

/**
 * @brief Configure cameras for specific workflow stage
 * @param stage_name Stage identifier ("preview", "detection", "alignment")
 * @return Error code (0=success)
 */
int camera_configure_for_stage(const char* stage_name);

// 已删除软件配置函数 - 所有参数在camera_init.c中写死配置



/*
配置相关函数声明（暂时禁用）
const void* get_camera_config(void);
void reset_camera_config(void);
*/

#ifdef __cplusplus
}
#endif

#endif // CAMERA_API_H
