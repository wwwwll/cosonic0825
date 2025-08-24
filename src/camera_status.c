/**
 * @file camera_status.c
 * @brief 性能监控实现 - 通过软件控制采集间隔实现帧率控制
 * 
 * 核心原则：
 * - 保持相机硬件参数固定，不修改曝光时间
 * - 通过采集间隔控制实际帧率
 * - 实现精确的性能监控和统计
 * 
 * @version 2.0
 * @date 2025/01/15
 * @author 性能监控专家
 */

#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#ifdef _WIN32
    #define WIN32_LEAN_AND_MEAN
    #include <windows.h>
    #include <winsock2.h>  // 包含timeval定义
#else
    #include <sys/time.h>
#endif
#include "MvCameraControl.h"
#include "MvErrorDefine.h"
#include "CameraParams.h"
#include "MvISPErrorDefine.h"
#include "MvSdkExport.h"
#include "PixelType.h"
#include "camera_api.h"

// 错误码定义（与camera_trigger.c保持一致）
#define ERR_INVALID_CAMERA_INDEX -1002
#define ERR_CAMERA_NOT_INITIALIZED -1003

// 性能监控相关定义
#define MAX_FPS 30
#define MIN_FPS 1
#define DEFAULT_FPS 10
#define STATS_WINDOW_SIZE 100  // 统计窗口大小

// 性能统计结构体
typedef struct {
    uint32_t target_fps;           // 目标帧率
    uint32_t frame_interval_ms;    // 帧间隔（毫秒）
    uint64_t last_capture_time;    // 上次采集时间戳（微秒）
    uint64_t frame_count;          // 总帧数
    uint32_t dropped_frames;       // 丢帧计数
    float actual_fps;              // 实际帧率
    uint64_t fps_calculation_start; // FPS计算起始时间
    uint32_t frames_in_window;     // 当前窗口内帧数
} CameraPerformanceStats;

// 每个相机的性能统计
static CameraPerformanceStats camera_stats[CAMERA_NUM] = {0};

#ifdef _WIN32
// Windows兼容的gettimeofday实现
static int gettimeofday(struct timeval* tp, void* tzp) {
    (void)tzp; // 避免未使用参数警告
    
    // 获取1970年1月1日到现在的100纳秒数
    FILETIME ft;
    ULARGE_INTEGER li;
    GetSystemTimeAsFileTime(&ft);
    li.LowPart = ft.dwLowDateTime;
    li.HighPart = ft.dwHighDateTime;
    
    // 转换为Unix时间戳（从1601年1月1日到1970年1月1日有11644473600秒）
    li.QuadPart -= 116444736000000000ULL;
    
    tp->tv_sec = (long)(li.QuadPart / 10000000ULL);
    tp->tv_usec = (long)((li.QuadPart % 10000000ULL) / 10ULL);
    return 0;
}
#endif

/**
 * @brief 获取当前时间戳（微秒）
 */
static uint64_t get_timestamp_us() {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (uint64_t)tv.tv_sec * 1000000 + tv.tv_usec;
}

/**
 * @brief 初始化相机性能统计
 * @param cam_index 相机索引
 * @param fps 目标帧率
 */
static void init_performance_stats(unsigned int cam_index, uint32_t fps) {
    if (cam_index >= CAMERA_NUM) return;
    
    CameraPerformanceStats* stats = &camera_stats[cam_index];
    stats->target_fps = fps;
    stats->frame_interval_ms = 1000 / fps;  // 计算帧间隔
    stats->last_capture_time = 0;
    stats->frame_count = 0;
    stats->dropped_frames = 0;
    stats->actual_fps = 0.0f;
    stats->fps_calculation_start = get_timestamp_us();
    stats->frames_in_window = 0;
    
    printf("camera_set_frame_rate: Initialized stats for camera %d - target %d fps (interval %d ms)\n", 
           cam_index, fps, stats->frame_interval_ms);
}

/**
 * @brief 更新帧率统计
 * @param cam_index 相机索引
 */
static void update_fps_stats(unsigned int cam_index) {
    if (cam_index >= CAMERA_NUM) return;
    
    CameraPerformanceStats* stats = &camera_stats[cam_index];
    uint64_t current_time = get_timestamp_us();
    
    stats->frame_count++;
    stats->frames_in_window++;
    
    // 每STATS_WINDOW_SIZE帧或每5秒计算一次实际帧率
    uint64_t time_diff = current_time - stats->fps_calculation_start;
    if (stats->frames_in_window >= STATS_WINDOW_SIZE || time_diff >= 5000000) { // 5秒
        if (time_diff > 0) {
            stats->actual_fps = (float)stats->frames_in_window * 1000000.0f / time_diff;
            stats->fps_calculation_start = current_time;
            stats->frames_in_window = 0;
        }
    }
}

// 已删除 should_capture_frame() - 不再需要软件帧率控制

// 已删除 camera_set_frame_rate() - 帧率在camera_init.c中写死

/**
 * @brief 获取相机状态和性能指标
 * @param cam_index 相机索引
 * @param fps_actual 实际帧率输出
 * @param frames_dropped 丢帧计数输出
 * @return 错误码 (0=成功)
 */
int camera_get_status(unsigned int cam_index, float* fps_actual, uint32_t* frames_dropped) {
    // 参数验证
    if (cam_index >= CAMERA_NUM) {
        printf("camera_get_status: Invalid camera index %d\n", cam_index);
        return ERR_INVALID_CAMERA_INDEX;
    }
    
    if (!cameras[cam_index].opened || NULL == cameras[cam_index].handle) {
        printf("camera_get_status: Camera %d not initialized\n", cam_index);
        return ERR_CAMERA_NOT_INITIALIZED;
    }
    
    if (NULL == fps_actual || NULL == frames_dropped) {
        printf("camera_get_status: Invalid output parameters\n");
        return MV_E_PARAMETER;
    }
    
    // 获取软件统计的性能数据
    CameraPerformanceStats* stats = &camera_stats[cam_index];
    
    *fps_actual = stats->actual_fps;
    *frames_dropped = stats->dropped_frames;
    
    // 额外获取相机硬件状态（用于监控，但不用于帧率控制）
    MVCC_FLOATVALUE stFloatValue;
    memset(&stFloatValue, 0, sizeof(MVCC_FLOATVALUE));
    
    int nRet = MV_CC_GetFloatValue(cameras[cam_index].handle, "AcquisitionFrameRate", &stFloatValue);
    if (MV_OK == nRet) {
        printf("camera_get_status: Camera %d hardware FPS: %.2f (software controlled: %.2f)\n", 
               cam_index, stFloatValue.fCurValue, *fps_actual);
    }
    
    printf("camera_get_status: Camera %d - Actual FPS: %.2f, Target: %d, Dropped: %d, Total frames: %llu\n", 
           cam_index, *fps_actual, stats->target_fps, *frames_dropped, 
           (unsigned long long)stats->frame_count);
    
    return MV_OK;
}

// 已删除 camera_set_exposure_time() 和 camera_set_gain() - 参数在camera_init.c中写死