/**
 * @file camera_init.c
 * @brief Initialize HIK SDK for dual-camera
 * 
 * Main progress:
 * 1. Initialize SDK
 * 2. Enum device and
 * 3. Recognize camera position
 * 4. Create handle
 * 5. Open device
 * 
 * @version 1.0
 * @date 2025/07/09
 * @author 李天都 Li Tiandu
 */

#include <stdio.h>
#include <string.h>
// #include <stdlib.h>  // [配置系统] 原用于atof函数，现已注释
#include <stdint.h>
#include "MvCameraControl.h"
#include "MvErrorDefine.h"
#include "CameraParams.h"
#include "MvISPErrorDefine.h"
#include "MvSdkExport.h"
#include "PixelType.h"
#include "camera_api.h"

// ============= [配置系统 - 已注释] 配置相关代码开始 =============
/*
// 全局变量定义：相机序列号
char g_left_camera_serial[32] = {0};
char g_right_camera_serial[32] = {0};

// 获取相机序列号的函数实现
const char* get_left_camera_serial(void) {
    return g_left_camera_serial[0] ? g_left_camera_serial : DEFAULT_LEFT_CAMERA_SERIAL;
}

const char* get_right_camera_serial(void) {
    return g_right_camera_serial[0] ? g_right_camera_serial : DEFAULT_RIGHT_CAMERA_SERIAL;
}

// 工作模式定义
static int g_current_mode = 0; // 0=default, 1=calibration, 2=alignment

// 函数前向声明
static float read_float_config(const char* key, float default_value);
static void read_string_config(const char* key, char* output, size_t output_size, const char* default_value);

// 设置相机工作模式
void set_camera_mode(int mode) {
    g_current_mode = mode;
    printf("camera_init: Set camera mode to %d (%s)\n", mode, 
           mode == 1 ? "calibration" : (mode == 2 ? "alignment" : "default"));
}
*/

// ============= [配置系统 - 已注释] 配置读取函数 =============
/*
static float read_float_config(const char* key, float default_value) {
    // 根据工作模式选择配置文件
    const char* config_file = "src-tauri/configs/default_config.txt";
    if (g_current_mode == 1) {
        config_file = "src-tauri/configs/calibration_config.txt";
    } else if (g_current_mode == 2) {
        config_file = "src-tauri/configs/alignment_config.txt";
    }
    
    FILE* file = fopen(config_file, "r");
    if (!file) {
        // 配置文件不存在，使用默认值
        return default_value;
    }
    
    char line[256];
    char target[64];
    memset(target, 0, sizeof(target));
    snprintf(target, sizeof(target), "%s=", key);
    
    while (fgets(line, sizeof(line), file)) {
        // 跳过注释行
        if (line[0] == '#' || line[0] == '\n' || line[0] == '\r') {
            continue;
        }
        // 查找匹配的键
        if (strncmp(line, target, strlen(target)) == 0) {
            float value = (float)atof(line + strlen(target));
            fclose(file);
            return value;
        }
    }
    
    fclose(file);
    return default_value;
}

/**
 * @brief 读取字符串配置值
 * @param key 配置键名
 * @param output 输出缓冲区
 * @param output_size 缓冲区大小
 * @param default_value 默认值
 */

/*static void read_string_config(const char* key, char* output, size_t output_size, const char* default_value) {
    // 根据工作模式选择配置文件
    const char* config_file = "src-tauri/configs/default_config.txt";
    if (g_current_mode == 1) {
        config_file = "src-tauri/configs/calibration_config.txt";
    } else if (g_current_mode == 2) {
        config_file = "src-tauri/configs/alignment_config.txt";
    }
    
    FILE* file = fopen(config_file, "r");
    if (!file) {
        // 配置文件不存在，使用默认值
        strncpy(output, default_value, output_size - 1);
        output[output_size - 1] = '\0';
        return;
    }
    
    char line[256];
    char target[64];
    memset(target, 0, sizeof(target));
    snprintf(target, sizeof(target), "%s=", key);
    
    while (fgets(line, sizeof(line), file)) {
        // 跳过注释行
        if (line[0] == '#' || line[0] == '\n' || line[0] == '\r') {
            continue;
        }
        // 查找匹配的键
        if (strncmp(line, target, strlen(target)) == 0) {
            char* value = line + strlen(target);
            // 去除换行符
            char* newline = strchr(value, '\n');
            if (newline) *newline = '\0';
            newline = strchr(value, '\r');
            if (newline) *newline = '\0';
            
            strncpy(output, value, output_size - 1);
            output[output_size - 1] = '\0';
            fclose(file);
            return;
        }
    }
    
    fclose(file);
    strncpy(output, default_value, output_size - 1);
    output[output_size - 1] = '\0';
}
*/
// ============= [配置系统 - 已注释] 配置相关代码结束 =============

/**
 * @brief camera info structure global array
 * 
 * - handle:    camera handle
 * - serial:    serial number
 * - opened:    camera open status
 * - position:  camera installation position sequence in jig
 * 
 * @note CAMERA_NUM = 2
 */
Camera cameras[CAMERA_NUM] = {
    {.handle = NULL, .serial = '\0', .opened = false, .position = UNINITIALIZED},
    {.handle = NULL, .serial = '\0', .opened = false, .position = UNINITIALIZED}
};

/**
 * @brief buffer size of one captured frame
 */
uint32_t g_frame_buf_size = 0;

/**
 * @brief set camera info structure
 * 
 * @param cam pointer of camera info structure
 * @param serial camera serial number
 * @param opened camera open status
 * @param position camera installation position sequence
 */
static void camera_set_info(Camera* cam, const char* serial, bool opened, CameraPosition position) {
    if (NULL == cam) {
        printf("No Camera.\n");
        return;
    }
    strncpy(cam->serial, serial, sizeof(cam->serial)-1);
    cam->serial[sizeof(cam->serial)-1] = '\0';
    cam->opened = opened;
    cam->position = position;
}

/**
 * @brief print device info
 * 
 * @param pstMVDevInfo device info sturcture
 * @return true:   successfully print
 * @return false:  print failed
 * 
 * @note Only support USB3.0 device
 */
bool print_device_info(MV_CC_DEVICE_INFO* pstMVDevInfo) {
    if (NULL == pstMVDevInfo) {
        printf("print_device_info: NULL Device Pointer\n");
        return false;
    }
    if (pstMVDevInfo->nTLayerType == MV_USB_DEVICE) {
        printf("UserDefinedName: %s\n", pstMVDevInfo->SpecialInfo.stUsb3VInfo.chUserDefinedName);
        printf("Serial Number: %s\n", pstMVDevInfo->SpecialInfo.stUsb3VInfo.chSerialNumber);
        printf("Device Number: %d\n\n", pstMVDevInfo->SpecialInfo.stUsb3VInfo.nDeviceNumber);
    } else {
        printf("print_device_info: USB3.0 Supported only.\n");
    }

    return true;
}

/**
 * @brief main function of camera initialization
 * 
 * Execution progress:
 * 1. Initialize SDK
 * 2. Enum device
 * 3. Ensure device number = 2
 * 4. Recognize camera left/right position
 * 5. Create device handle
 * 6. Open device in exclusive mode
 * 
 * @return int error code (MV_OK: success)
 * @note Resource will be released after failed initialization using camera_release()
 */
int camera_init(){
    int nRet = MV_OK;
    
    // [配置系统 - 已注释] 初始化时读取相机序列号配置
    // read_string_config("left_camera_serial", g_left_camera_serial, sizeof(g_left_camera_serial), DEFAULT_LEFT_CAMERA_SERIAL);
    // read_string_config("right_camera_serial", g_right_camera_serial, sizeof(g_right_camera_serial), DEFAULT_RIGHT_CAMERA_SERIAL);
    // printf("camera_init: Using camera serials - Left: %s, Right: %s\n", g_left_camera_serial, g_right_camera_serial);

    do {
        // initialize SDK
        nRet = MV_CC_Initialize();
        if (MV_OK != nRet) {
            printf("Fail to Initialize SDK fail: 0x%x\n", nRet);
            break;
        }

        // enum device
        MV_CC_DEVICE_INFO_LIST stDeviceList;
        memset(&stDeviceList, 0, sizeof(MV_CC_DEVICE_INFO_LIST));
        //nRet = MV_CC_EnumDevices(MV_GIGE_DEVICE | MV_USB_DEVICE | MV_GENTL_CAMERALINK_DEVICE | MV_GENTL_CXP_DEVICE | MV_GENTL_XOF_DEVICE, &stDeviceList); // transport layer protocol type, USB
        nRet = MV_CC_EnumDevices(MV_USB_DEVICE, &stDeviceList); //USB 3.0 supported only
        if (MV_OK != nRet) {
            printf("Fail to Enum Device: 0x%x\n", nRet);
            break;
        }

        if (stDeviceList.nDeviceNum > 0) {
            for (unsigned int i = 0; i < stDeviceList.nDeviceNum; i++) {
                printf("[Device %d]:\n", i);
                MV_CC_DEVICE_INFO* pDeviceInfo = stDeviceList.pDeviceInfo[i];
                if (NULL == pDeviceInfo) {
                    break;
                }
                print_device_info(pDeviceInfo);
            }
        } else {
            printf("camera_init: No Device Found\n");
            break;
        }

        // check camera number
        if (stDeviceList.nDeviceNum != CAMERA_NUM) {
            printf("Expect 2 Camera. Current: %d\n", stDeviceList.nDeviceNum);
            nRet = MV_E_SUPPORT;
            break;
        }

        // enum sequence
        char* serial0 = stDeviceList.pDeviceInfo[0]->SpecialInfo.stUsb3VInfo.chSerialNumber;
        char* serial1 = stDeviceList.pDeviceInfo[1]->SpecialInfo.stUsb3VInfo.chSerialNumber;

        // postion index
        int left_index = -1, right_index = -1;

        // recognize enum sequence position
        if (strcmp(serial0, LEFT_CAMERA_SERIAL) == 0 && strcmp(serial1, RIGHT_CAMERA_SERIAL) == 0) {
            left_index = 0;
            right_index = 1;
        } else if (strcmp(serial1, LEFT_CAMERA_SERIAL) == 0 && strcmp(serial0, RIGHT_CAMERA_SERIAL) == 0) {
            left_index = 1;
            right_index = 0;
        } else {
            printf("Need to Modify Camera Serial Number Setting.\n");
            break;
        }

        // create handle and set info for left cam
        nRet = MV_CC_CreateHandle(&cameras[0].handle, stDeviceList.pDeviceInfo[left_index]);
        if (MV_OK != nRet) {
            printf("Fail to Create Handle for Left Camera: 0x%x\n", nRet);
            break;
        }
        camera_set_info(&cameras[0], serial0, true, LEFT_CAM);

        // create handle and set info for right cam
        nRet = MV_CC_CreateHandle(&cameras[1].handle, stDeviceList.pDeviceInfo[right_index]);
        if (MV_OK != nRet) {
            printf("Fail to Create Handle for Right Camera: 0x%x\n", nRet);
            break;
        }
        camera_set_info(&cameras[1], serial1, true, RIGHT_CAM);

        // open device
        // exclusive access, SwitchoverKey = 0
        nRet = MV_CC_OpenDevice(cameras[0].handle, MV_ACCESS_Exclusive, 0);
        if (MV_OK != nRet) {
            printf("Fail to Open Left Camera: 0x%x\n", nRet);
            break;
        }
        nRet = MV_CC_OpenDevice(cameras[1].handle, MV_ACCESS_Exclusive, 0);
        if (MV_OK != nRet) {
            printf("Fail to Open Right Camera: 0x%x\n", nRet);
            break;
        }

        // 🎯 简化版参数设置：直接在camera_init中配置（10fps, continuous mode）
        printf("camera_init: Configuring cameras (10fps, continuous mode)...\n");
        for (unsigned int i = 0; i < CAMERA_NUM; i++) {
            // 设置像素格式
            // nRet = MV_CC_SetEnumValue(cameras[i].handle, "PixelFormat", 1); // 1 = Mono8
            // if (MV_OK != nRet) {
            //     printf("camera_init: Warning - Camera %d pixel format setting failed: 0x%x\n", i, nRet);
            // }
            
            // Step 1: 设置触发模式为连续
            nRet = MV_CC_SetEnumValue(cameras[i].handle, "TriggerMode", 0); // 0 = Off
            if (MV_OK != nRet) {
                printf("camera_init: Warning - Camera %d trigger mode setting failed: 0x%x\n", i, nRet);
            }
            
            // Step 2: 启用帧率控制
            nRet = MV_CC_SetBoolValue(cameras[i].handle, "AcquisitionFrameRateEnable", true);
            if (MV_OK != nRet) {
                printf("camera_init: Warning - Camera %d frame rate control enable failed: 0x%x\n", i, nRet);
            }
            
            // Step 3: 设置帧率为10fps（硬编码）
            //float frame_rate = 10.0;  // 写死配置
            // [配置系统 - 已注释] float frame_rate = read_float_config("camera_frame_rate", 10.0);
            nRet = MV_CC_SetFloatValue(cameras[i].handle, "AcquisitionFrameRate", 10.0);
            if (MV_OK != nRet) {
                printf("camera_init: Warning - Camera %d frame rate setting failed: 0x%x\n", i, nRet);
            } else {
                printf("camera_init: Camera %d frame rate set to %.1f fps\n", i, 10.0);
            }

            // Step 4: 设置曝光时间 us
            float exposure_time = 90000.0;  // 写死配置
            // [配置系统 - 已注释] float exposure_time = read_float_config("camera_exposure_time", 60000.0);
            nRet = MV_CC_SetFloatValue(cameras[i].handle, "ExposureTime", exposure_time);
            if (MV_OK != nRet) {
                printf("camera_init: Warning - Camera %d exposure time setting failed: 0x%x\n", i, nRet);
            } else {
                printf("camera_init: Camera %d exposure time set to %.1f us\n", i, exposure_time);
            }

            // Step 5: 设置增益
            nRet = MV_CC_SetEnumValueByString(cameras[i].handle, "GainAuto", "Off"); 
            if (MV_OK != nRet) {
                printf("camera_init: Warning - Camera %d gain auto setting failed: 0x%x\n", i, nRet);
            }
            
            float gain = 5.0;  // 写死配置
            // [配置系统 - 已注释] float gain = read_float_config("camera_gain", 10.0);
            nRet = MV_CC_SetFloatValue(cameras[i].handle, "Gain", gain);
            if (MV_OK != nRet) {
                printf("camera_init: Warning - Camera %d gain setting failed: 0x%x\n", i, nRet);
            } else {
                printf("camera_init: Camera %d gain set to %.1f\n", i, gain);
            }
        }
        printf("camera_init: Camera configuration applied (10fps, heating reduced)\n");
        
        printf("camera_init: All cameras configured, proceeding with buffer calculation...\n");

        // TODO: 设置ROI

        // calculate frame buffer size
        // bufferSize[]用于存储两个相机计算得出的buf_size，由于两个相机型号和设置都相同，理论上的值应该相同
        // stIntExWidth用来存储图像的宽，stIntExHeight用来存储图像的高，stEnumValueEx用来存储Pixel Size
        uint32_t bufferSize[CAMERA_NUM];
        for (unsigned int i = 0; i < CAMERA_NUM; i++) {
            MVCC_INTVALUE_EX stIntExWidth = {0};
            MVCC_INTVALUE_EX stIntExHeight = {0};
            MVCC_ENUMVALUE_EX stEnumValueExPixelSize = {0};

            nRet = MV_CC_GetIntValueEx(cameras[i].handle, "Width", &stIntExWidth);
            if (MV_OK != nRet) {
                printf("Fail to Get Camera %d Image Width: 0x%x\n", i, nRet);
                return nRet;
            }
            nRet = MV_CC_GetIntValueEx(cameras[i].handle, "Height", &stIntExHeight);
            if (MV_OK != nRet) {
                printf("Fail to Get Camera %d Image Width: 0x%x\n", i, nRet);
                return nRet;
            }
            nRet = MV_CC_GetEnumValueEx(cameras[i].handle, "PixelSize", &stEnumValueExPixelSize);
            if (MV_OK != nRet) {
                printf("Fail to Get Camera %d PixelSize: 0x%x\n", i, nRet);
                return nRet;
            }

            uint32_t bpp = (stEnumValueExPixelSize.nCurValue + 7) / 8;
            bufferSize[i] = stIntExWidth.nCurValue * stIntExHeight.nCurValue 
                                        * bpp;
        }
        if (bufferSize[0] != bufferSize[1]) {
            printf("camera_init: Camera Buffer Size Differ (%u vs %u)\n", bufferSize[0], bufferSize[1]);
            nRet = MV_E_SUPPORT;
            break;
        } else {
            g_frame_buf_size = bufferSize[0];
        }
    } while (0);

    if (MV_OK != nRet) {
        camera_release();
        return nRet;
    }
    return MV_OK;
}