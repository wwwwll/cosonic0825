/**
 * @file camera_capture.c
 * @brief fetch frame from cameras
 * 
 * Main progress:
 * 1. Continuous grabbing image
 * 2. 
 * 实时预览+手动保存。但是实时预览时也要显示对应的实时的apriltag的坐标，详细的功能描述如下:
 * 1. 实时预览：启动后连续采集，实现实时预览相机拍摄的AprilTag画面，及AprilTag实时坐标系
 * 2. 手动保存：提供轮询取图功能，在前端点击“拍照”后触发，此时AprilTag和双目相机均固定不动，保存当前相机拍摄画面，及拍摄时的AprilTag坐标系。保存完成后恢复到实时预览状态
 * 3. AprilTag坐标系需要始终显示
 * 按照以上需求，是否使用工作线程更合适？因为相机启动后不论是实时预览还是手动保存，后端都需要一直计算AprilTag的坐标系并回传给前端
 * 
 * 评审代码
 * camera_start()用来连续采集
 * free_image_buffer()用来申请的SDK内部buffer数组
 * camera_get_frame()用来取当前两个相机的1帧
 * 疑问：
 * 1. 在camera_get_frame()中能明确看到两个相机的当前帧地址被存在传入该函数的参数out_buf数组中，以供后续进行图像处理（计算坐标等）。但是调用camera_start()进行连续采集时，取得的帧存在哪里？我应该如何进行后续的图像处理？
 * 2. camera_get_frame()和free_image_buffer函数的传入参数涉及到了数组，请帮我确认这样的代码是否正确，以及后面调用out_size[i] = copy_len是否正确
 * 
 * @version 1.0
 * @date 20250711
 * @author 李天都 Li Tiandu
 */

#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <stdlib.h>
#include <time.h>
#include <windows.h>
#include "MvCameraControl.h"
#include "MvErrorDefine.h"
#include "CameraParams.h"
#include "MvISPErrorDefine.h" 
#include "MvSdkExport.h"
#include "PixelType.h"
#include "camera_api.h"

/**
 * @brief start grabbing image continuously from all cameras
 * 
 * @return int error code (MV_OK if success)
 */
int camera_start() {
    int nRet = MV_OK;
    for (unsigned int i = 0; i < CAMERA_NUM; i++) {
        if (!cameras[i].opened || NULL == cameras[i].handle) {
            printf("Fail to Start Camera %d", i);
            return -1;
        }
        nRet = MV_CC_StartGrabbing(cameras[i].handle);
        if (MV_OK != nRet) {
            printf("Fail to Start Grabbing Camera %d: 0x%x\n", i, nRet);
            return nRet;
        }
    }
    
    return MV_OK;
}

/**
 * @brief free all image buffer
 * 
 * @param stFrame[] captured frame arrays
 * @param count camera numbers (CAMERA_NUM = 2)
 * @return int error code (MV_OK if success)
 */
int free_image_buffer(MV_FRAME_OUT stFrame[], unsigned int count) {
    int nRet = MV_OK;
    for (unsigned int i = 0; i < count; i++) {
        nRet = MV_CC_FreeImageBuffer(cameras[i].handle, &stFrame[i]);
        if (MV_OK != nRet) {
            printf("Fail to Free Image Buffer for Camera %d: 0x%x\n", i, nRet);
            return nRet;
        }
    }
    return MV_OK;
}

/**
 * @brief get current frame from all cameras
 * 
 * TIMEOUT_MS超时时间
 * 
 * @param out_bufs[] output buffer pointer array, point to one frame image data
 * @param out_sizes[] output buffer size array stores captured frames length
 * @return int error code (MV_OK if success)
 */
//===========================================================
/**
 * C SDK的camera_get_frame() 的参数问题：
 * 1. out_buf[]和out_size[]的越界处理。是否应该在函数声明处就设定数组长度？不需要，在函数中处理越界
 * 2. 输入参数获取的帧长度buf_size。是否应该作为输入参数？预先定义的帧数组长度，在camera_set_param()中设置；或是不作为函数的参数，而是在调用MV_CC_GetImageBuffer后计算帧长度？在.h中定义缓冲区长度
 * 3. 确保out_buf[i]能保存1帧图像
 */
//==========================================================

int camera_get_frame(uint8_t* out_bufs[], uint32_t out_sizes[]) {

    if (!out_bufs || !out_sizes) {
        printf("camera_get_frame: Invalid Buffer: 0x%x\n", MV_E_BUF_INVALID);
        return MV_E_BUF_INVALID;
    }
    
    int nRet = MV_OK;

    MV_FRAME_OUT stFrame[CAMERA_NUM];  

    // fetch one frames from each camera
    for (unsigned int i = 0; i < CAMERA_NUM; i++) {
        if (!cameras[i].opened || NULL == cameras[i].handle) {
            printf("Fail to Get Frame from Camera %d", i);
            return -1;
        }
        nRet = MV_CC_GetImageBuffer(cameras[i].handle, &stFrame[i], TIMEOUT_MS);
        if (MV_OK != nRet) {
            printf("Fail to GetImageBuffer from Camera %d: 0x%x\n", i, nRet);
            // free buffer
            free_image_buffer(stFrame, CAMERA_NUM);
            return nRet;
        }
    }

    if (g_frame_buf_size == 0) {
        printf("camera_get_frame: Frame Buffer Size is 0\n");
        return MV_E_BUF_INVALID;
    }

    // store buffer addr to out_buf[CAMERA_NUM]
    for (unsigned int i = 0; i < CAMERA_NUM; i++) {
        //printf("camera_get_frame: FrameLen %d", stFrame[i].stFrameInfo.nFrameLen);
        uint32_t copy_len = stFrame[i].stFrameInfo.nFrameLen;
        if (copy_len > g_frame_buf_size) {
            copy_len = g_frame_buf_size;
        }
        memcpy(out_bufs[i], stFrame[i].pBufAddr, copy_len);
        out_sizes[i] = copy_len;
    }

    // free SDK buffer
    free_image_buffer(stFrame, CAMERA_NUM);
    return MV_OK;
}

/**
 * @brief return one frame buffer size for Rust to malloc
 * @return one frame buffer size (width*height*bpp)
 */
uint32_t camera_get_frame_buf_size() {
    return g_frame_buf_size;
}

