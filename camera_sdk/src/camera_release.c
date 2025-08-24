/**
 * @file camera_release.c
 * @brief Release camera resource
 * 
 * Main progress:
 * 1. Stop grabbing image
 * 2. Close device
 * 3. Destory handle
 * 4. Finalize SDK
 * 
 * @version 1.0
 * @date 2025/07/10
 * @author 李天都 Li Tiandu
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

/**
 * @brief release all camera resource
 */
int camera_release() {
    int nRet = MV_OK;
    int lastErr = MV_OK;

    printf("camera_release: Starting camera resource cleanup...\n");

    // stop grab image - 对调用顺序错误更加宽容
    for (unsigned int i = 0; i < CAMERA_NUM; i++) {
        if (cameras[i].handle && cameras[i].opened) {
            nRet = MV_CC_StopGrabbing(cameras[i].handle);
            if (MV_OK != nRet) {
                if (nRet == 0x80000003) { // MV_E_CALLORDER
                    printf("camera_release: Camera %d was not grabbing (expected in some modes)\n", i);
                } else {
                    printf("camera_release: Fail to Stop Grabbing from Camera %d: 0x%x\n", i, nRet);
                    lastErr = nRet;
                }
            } else {
                printf("camera_release: Successfully stopped grabbing from Camera %d\n", i);
            }
        }
    }

    // close device
    for (unsigned int i = 0; i < CAMERA_NUM; i++) {
        if (cameras[i].handle && cameras[i].opened) {
            nRet = MV_CC_CloseDevice(cameras[i].handle);
            if (MV_OK != nRet) {
                printf("camera_release: Fail to Close Device for Camera %d: 0x%x\n", i, nRet);
                lastErr = nRet;
            } else {
                printf("camera_release: Successfully closed Camera %d\n", i);
            }
        }
    }

    // destory handle
    for (unsigned int i = 0; i < CAMERA_NUM; i++) {
        if (cameras[i].handle) {
            nRet = MV_CC_DestroyHandle(cameras[i].handle);
            if (MV_OK != nRet) {
                printf("camera_release: Fail to Destory Handle for Camera %d: 0x%x\n", i, nRet);
                lastErr = nRet;
            } else {
                printf("camera_release: Successfully destroyed handle for Camera %d\n", i);
            }
        }
        cameras[i].handle = NULL;
        cameras[i].opened = false;
        memset(cameras[i].serial, 0, sizeof(cameras[i].serial));
        cameras[i].position = UNINITIALIZED;
    }

    // finalize SDK
    nRet = MV_CC_Finalize();
    if (MV_OK != nRet) {
        printf("camera_release: Fail to Finalize SDK: 0x%x\n", nRet);
        lastErr = nRet;
    } else {
        printf("camera_release: Successfully finalized SDK\n");
    }

    if (MV_OK != lastErr) {
        printf("camera_release: Camera release completed with some warnings (this may be normal).\n");
        // 对于调用顺序错误，我们认为是正常的，不返回错误
        if (lastErr == 0x80000003) {
            printf("camera_release: Call order warnings are normal in some workflow states.\n");
            return MV_OK;
        }
        return lastErr;
    }

    printf("camera_release: All camera resources released successfully.\n");
    return MV_OK;
}
