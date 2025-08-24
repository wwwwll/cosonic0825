/**
 * @file camera_workflow.c
 * @brief 工作流程适配实现 - 根据阶段配置相机参数
 */

#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include "MvCameraControl.h"
#include "MvErrorDefine.h"
#include "CameraParams.h"
#include "MvISPErrorDefine.h"
#include "MvSdkExport.h"
#include "PixelType.h"
#include "camera_api.h"

// 外部变量声明（在camera_init.c中定义）
extern Camera cameras[CAMERA_NUM];

/**
 * @brief 根据工作流程阶段配置相机
 * @param stage_name 阶段标识符 ("preview", "detection", "alignment")
 * @return 错误码 (0=成功)
 */
int camera_configure_for_stage(const char* stage_name) {
    int nRet = MV_OK;
    
    if (NULL == stage_name) {
        printf("camera_configure_for_stage: Invalid stage name\n");
        return MV_E_PARAMETER;
    }
    
    printf("camera_configure_for_stage: Configuring cameras for stage '%s'\n", stage_name);
    
    // 根据不同阶段配置相机参数
    if (strcmp(stage_name, "preview") == 0) {
        // 预览模式：10fps连续采集
        for (unsigned int i = 0; i < CAMERA_NUM; i++) {
            if (cameras[i].opened && cameras[i].handle) {
                // 设置连续采集模式
                nRet = camera_set_trigger_mode(i, TRIGGER_OFF);
                if (MV_OK != nRet) {
                    printf("camera_configure_for_stage: Failed to set continuous mode for camera %d\n", i);
                    continue;
                }
                
                // 设置预览帧率
                nRet = camera_set_frame_rate(i, 10);
                if (MV_OK != nRet) {
                    printf("camera_configure_for_stage: Failed to set preview frame rate for camera %d\n", i);
                }
            }
        }
    } else if (strcmp(stage_name, "detection") == 0) {
        // 检测模式：软触发按需模式
        for (unsigned int i = 0; i < CAMERA_NUM; i++) {
            if (cameras[i].opened && cameras[i].handle) {
                nRet = camera_set_trigger_mode(i, TRIGGER_SOFTWARE);
                if (MV_OK != nRet) {
                    printf("camera_configure_for_stage: Failed to set software trigger mode for camera %d\n", i);
                }
            }
        }
    } else if (strcmp(stage_name, "alignment") == 0) {
        // 合像模式：高精度同步模式
        for (unsigned int i = 0; i < CAMERA_NUM; i++) {
            if (cameras[i].opened && cameras[i].handle) {
                nRet = camera_set_trigger_mode(i, TRIGGER_SOFTWARE);
                if (MV_OK != nRet) {
                    printf("camera_configure_for_stage: Failed to set sync mode for camera %d\n", i);
                    continue;
                }
                
                // 设置高精度曝光时间
                nRet = MV_CC_SetFloatValue(cameras[i].handle, "ExposureTime", 10000.0f); // 10ms
                if (MV_OK != nRet) {
                    printf("camera_configure_for_stage: Failed to set exposure time for camera %d\n", i);
                }
            }
        }
    } else {
        printf("camera_configure_for_stage: Unknown stage '%s'\n", stage_name);
        return MV_E_PARAMETER;
    }
    
    printf("camera_configure_for_stage: Successfully configured cameras for stage '%s'\n", stage_name);
    return MV_OK;
}