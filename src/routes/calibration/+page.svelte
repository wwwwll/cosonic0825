<!-- CalibrationTest.svelte - 相机标定工作流程前端界面 -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  // 状态管理
  let calibrationStatus = 'NotStarted'; // NotStarted, Capturing, ReadyToCalibrate, Calibrating, Completed, Failed
  let failedReason = ''; // 用于存储Failed状态的错误信息
  let sessionId = null;
  let statusMessage = '相机标定系统准备就绪';
  let errorMessage = '';

  // 图像数据
  let liveImages = { left: null, right: null }; // 实时预览图像
  let capturedImages = []; // 已采集的标定图像对
  let calibrationResult = null;

  // 按钮状态控制
  let isStartCameraEnabled = true;
  let isCaptureImageEnabled = false;
  let isStartCalibrationEnabled = false;
  let isStopCameraEnabled = false;

  // 事件监听器和定时器
  let statusUnlisten = null;
  let previewUnlisten = null;
  let previewTimer = null;
  let isPreviewActive = false;
  let previewInterval = 125; // 默认8fps (125ms)，匹配硬件性能
  let previewErrorCount = 0; // 预览错误计数器
  
  // Debug和性能监控变量
  let previewFrameCount = 0;
  let lastPreviewTime = 0;
  let previewStartTime = 0;

  onMount(async () => {
    console.log('相机标定页面已加载');
    
    // 获取当前标定状态
    try {
      const status = await invoke('get_calibration_status');
      processBackendStatus(status);
      console.log('当前标定状态:', status);
      
      // 如果有活跃的标定会话，获取已采集的图像
      if (status === 'Capturing' || status === 'ReadyToCalibrate') {
        const images = await invoke('get_captured_images');
        capturedImages = images;
        console.log('已加载图像列表:', images.length, '组');
        
        if (status === 'Capturing') {
          statusMessage = `已采集 ${capturedImages.length} 组图像，可以继续采集`;
          // TODO: 恢复实时预览
        } else if (status === 'ReadyToCalibrate') {
          statusMessage = `已采集 ${capturedImages.length} 组图像，可以开始标定`;
        }
      } else {
        statusMessage = '相机标定系统准备就绪';
      }
      
      updateButtonStates();
    } catch (error) {
      console.error('获取标定状态失败:', error);
      statusMessage = '获取系统状态失败，请检查后端连接';
    }
    
    // TODO: 添加事件监听（实时预览等）
  });

  onDestroy(() => {
    if (statusUnlisten) statusUnlisten();
    if (previewUnlisten) previewUnlisten();
    if (previewTimer) clearInterval(previewTimer);
  });

  // 处理后端状态（支持Failed(String)类型）
  function processBackendStatus(status) {
    if (typeof status === 'string') {
      calibrationStatus = status;
      failedReason = '';
    } else if (typeof status === 'object' && status.Failed) {
      calibrationStatus = 'Failed';
      failedReason = status.Failed;
    } else {
      calibrationStatus = status;
      failedReason = '';
    }
  }

  // 更新按钮状态
  function updateButtonStates() {
    switch (calibrationStatus) {
      case 'NotStarted':
        isStartCameraEnabled = true;
        isCaptureImageEnabled = false;
        isStartCalibrationEnabled = false;
        isStopCameraEnabled = false;
        break;
      case 'Capturing':
        isStartCameraEnabled = false;
        isCaptureImageEnabled = true;
        isStartCalibrationEnabled = capturedImages.length >= 15; // 15组图像后可标定（匹配后端配置）
        isStopCameraEnabled = true;
        break;
      case 'ReadyToCalibrate':
        isStartCameraEnabled = false;
        isCaptureImageEnabled = false;
        isStartCalibrationEnabled = true;
        isStopCameraEnabled = true;
        break;
      case 'Calibrating':
        isStartCameraEnabled = false;
        isCaptureImageEnabled = false;
        isStartCalibrationEnabled = false;
        isStopCameraEnabled = false;
        break;
      case 'Completed':
      case 'Failed':
        isStartCameraEnabled = true;
        isCaptureImageEnabled = false;
        isStartCalibrationEnabled = false;
        isStopCameraEnabled = false;
        break;
    }
  }

  // 启动相机 - 调用Tauri命令
  async function startCamera() {
    try {
      console.log('🎬 [启动相机] 开始启动流程...');
      statusMessage = '正在启动相机...';
      errorMessage = '';
      updateButtonStates();
      
      // 调用后端Tauri命令启动标定会话
      console.log('📞 [启动相机] 调用 start_calibration_session...');
      const sessionId = await invoke('start_calibration_session');
      console.log('✅ [启动相机] 标定会话已启动，会话ID:', sessionId);
      
      // 使用合理的预览频率，匹配硬件性能
      const targetPreviewFps = 8; // 使用8fps，略低于硬件15fps
      previewInterval = Math.floor(1000 / targetPreviewFps);
      console.log(`设置预览帧率为 ${targetPreviewFps}fps，间隔 ${previewInterval}ms (匹配硬件15fps)`);
      
      calibrationStatus = 'Capturing';
      statusMessage = '✓ 相机已启动，可以开始采集标定图像';
      
      console.log('🎥 [启动相机] 开始启动实时预览...');
      
      // 启动实时预览
      startPreviewPolling();
      
      // 暂时使用占位图像，等待预览数据
      liveImages = {
        left: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==',
        right: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=='
      };
      
      console.log('✅ [启动相机] 启动流程完成，状态切换为 Capturing');
      updateButtonStates();
    } catch (error) {
      console.error('启动相机失败:', error);
      errorMessage = `启动相机失败: ${error}`;
      calibrationStatus = 'NotStarted';
      updateButtonStates();
    }
  }

  // 采集图像 - 使用统一接口
  async function captureImage() {
    try {
      console.log(`📷 [采集图像] 开始采集第${capturedImages.length + 1}组图像...`);
      statusMessage = '正在采集标定图像 (2448×2048分辨率)...';
      errorMessage = '';
      updateButtonStates();
      
      // 使用统一接口，预览并保存
      console.log('📞 [采集图像] 调用 get_preview_frame(shouldSave: true)...');
      const previewFrame = await invoke('get_preview_frame', { shouldSave: true });
      console.log('✅ [采集图像] 采集预览帧成功:', previewFrame ? '有数据' : '无数据');
      
              // 获取最新保存的图像信息
        console.log('📞 [采集图像] 调用 get_latest_captured_image...');
        const latestImage = await invoke('get_latest_captured_image');
        
                  if (latestImage) {
            console.log('✅ [采集图像] 最新保存的图像:', `ID:${latestImage.pair_id}, 检测到标定板:${latestImage.has_calibration_pattern}`);
          } else {
            console.log('❌ [采集图像] 无数据');
          }
      
              if (latestImage && latestImage.has_calibration_pattern) {
          // 采集成功 - 添加到图像列表
          capturedImages = [...capturedImages, latestImage];
          console.log(`✅ [采集图像] 成功采集第${capturedImages.length}组图像`);
          
          if (capturedImages.length >= 15) {
            // 检查后端状态是否也更新了
            const backendStatus = await invoke('get_calibration_status');
            console.log(`📊 [采集图像] 采集完成后后端状态: ${JSON.stringify(backendStatus)}`);
            
            calibrationStatus = 'ReadyToCalibrate';
            statusMessage = `✓ 已采集 ${capturedImages.length} 组图像，可以开始标定`;
          } else {
            statusMessage = `✓ 已采集 ${capturedImages.length} 组图像，还需 ${15 - capturedImages.length} 组`;
          }
      } else if (latestImage && !latestImage.has_calibration_pattern) {
        // 采集失败 - 未检测到标定板
        errorMessage = '❌ 采集失败：未检测到标定板，请调整标定板位置后重试';
        statusMessage = `当前已采集 ${capturedImages.length} 组图像，请重新拍摄`;
      } else {
        // 没有获取到图像信息
        errorMessage = '❌ 采集失败：无法获取图像信息';
        statusMessage = `当前已采集 ${capturedImages.length} 组图像，请重试`;
      }
      
      updateButtonStates();
      
    } catch (error) {
      console.error('采集图像失败:', error);
      errorMessage = `采集图像失败: ${error}`;
      statusMessage = `当前已采集 ${capturedImages.length} 组图像`;
      updateButtonStates();
    }
  }

  // 开始标定 - 调用Tauri命令
  async function startCalibration() {
    try {
      console.log(`🎯 [开始标定] 当前状态: ${calibrationStatus}, 图像数量: ${capturedImages.length}`);
      
              // 检查前端状态和图像数量
        if (capturedImages.length < 15) {
          errorMessage = `图像数量不足：当前${capturedImages.length}组，需要15组`;
          console.error('❌ [开始标定] 图像数量不足');
          return;
        }
      
      // 检查后端状态
      console.log('📞 [开始标定] 获取后端状态...');
      const backendStatus = await invoke('get_calibration_status');
      console.log('📊 [开始标定] 后端状态:', backendStatus);
      
      if (backendStatus !== 'ReadyToCalibrate') {
        errorMessage = `后端状态错误：当前为${JSON.stringify(backendStatus)}，需要ReadyToCalibrate`;
        console.error('❌ [开始标定] 后端状态不正确');
        return;
      }
      
      calibrationStatus = 'Calibrating';
      statusMessage = '正在执行标定算法，请稍候...';
      errorMessage = '';
      updateButtonStates();
      
      // 停止预览轮询，因为标定过程中相机会被关闭
      console.log('⏹️ [开始标定] 停止预览轮询...');
      stopPreviewPolling();
      
      // 调用后端Tauri命令执行标定
      console.log('📞 [开始标定] 调用 run_calibration_process...');
      const result = await invoke('run_calibration_process');
              console.log('✅ [开始标定] 标定结果:', result);
        console.log(`📊 [标定结果] 左相机RMS: ${result.left_rms_error?.toFixed(4)}`);
        console.log(`📊 [标定结果] 右相机RMS: ${result.right_rms_error?.toFixed(4)}`);
        console.log(`📊 [标定结果] 双目RMS: ${result.stereo_rms_error?.toFixed(4)}`);
        
        if (result.success) {
          calibrationStatus = 'Completed';
          statusMessage = '✓ 标定完成！相机已关闭';
          calibrationResult = result;
          liveImages = { left: null, right: null };
        
        // 标定完成后滚动到结果区域
        setTimeout(() => {
          scrollToResult();
        }, 100);
      } else {
        calibrationStatus = 'Failed';
        statusMessage = '❌ 标定失败';
        errorMessage = `标定算法执行失败，请检查图像质量`;
      }
      
      updateButtonStates();
      
    } catch (error) {
      console.error('标定失败:', error);
      errorMessage = `标定失败: ${error}`;
      calibrationStatus = 'Failed';
      updateButtonStates();
    }
  }

  // 滚动到标定结果区域
  function scrollToResult() {
    const resultElement = document.querySelector('.result-panel');
    if (resultElement) {
      resultElement.scrollIntoView({ 
        behavior: 'smooth', 
        block: 'start' 
      });
    }
  }

  // 启动实时预览轮询
  function startPreviewPolling() {
    if (previewTimer) clearInterval(previewTimer);
    
    isPreviewActive = true;
    previewFrameCount = 0;
    previewStartTime = Date.now();
    console.log(`🚀 启动实时预览，帧率间隔: ${previewInterval}ms`);
    
    previewTimer = setInterval(async () => {
      if (!isPreviewActive) return;
      
      const requestStartTime = Date.now();
      const actualInterval = lastPreviewTime > 0 ? requestStartTime - lastPreviewTime : 0;
      
      try {
        console.log(`📸 [帧${previewFrameCount + 1}] 开始请求预览帧 (间隔: ${actualInterval}ms)`);
        
        // 使用统一接口，只预览不保存
        const previewFrame = await invoke('get_preview_frame', { shouldSave: false });
        const requestEndTime = Date.now();
        const requestDuration = requestEndTime - requestStartTime;
        
        if (previewFrame && isPreviewActive) {
          previewFrameCount++;
          console.log(`✅ [帧${previewFrameCount}] 预览帧获取成功 (耗时: ${requestDuration}ms)`);
          
          // 检查图像数据
          const leftSize = previewFrame.left_preview ? previewFrame.left_preview.length : 0;
          const rightSize = previewFrame.right_preview ? previewFrame.right_preview.length : 0;
          console.log(`📊 [帧${previewFrameCount}] 图像数据大小 - 左: ${leftSize}, 右: ${rightSize}`);
          
          liveImages = {
            left: previewFrame.left_preview,
            right: previewFrame.right_preview
          };
          
          // 每10帧输出一次性能统计
          if (previewFrameCount % 10 === 0) {
            const totalTime = Date.now() - previewStartTime;
            const avgFps = (previewFrameCount / totalTime * 1000).toFixed(2);
            const avgDuration = (totalTime / previewFrameCount).toFixed(2);
            console.log(`📊 [性能统计] 已处理${previewFrameCount}帧，平均帧率: ${avgFps}fps，平均耗时: ${avgDuration}ms`);
          }
          
          // 可选：显示标定板检测状态
          if (previewFrame.has_pattern !== undefined && previewFrame.has_pattern !== null) {
            console.log(`🎯 [帧${previewFrameCount}] 检测到标定板:`, previewFrame.has_pattern);
          }
          
          // 重置错误计数器
          previewErrorCount = 0;
        } else {
          console.warn(`⚠️ [帧${previewFrameCount + 1}] 预览帧为空或预览已停止`);
        }
        
        lastPreviewTime = requestEndTime;
        
      } catch (error) {
        previewErrorCount++;
        const requestEndTime = Date.now();
        const requestDuration = requestEndTime - requestStartTime;
        
        console.error(`❌ [帧${previewFrameCount + 1}] 获取预览帧失败 (${previewErrorCount}/3, 耗时: ${requestDuration}ms):`, error);
        console.error(`🔍 [错误详情] 错误类型: ${typeof error}, 错误内容:`, error);
        
        // 如果连续失败3次，停止预览避免崩溃
        if (previewErrorCount >= 3) {
          console.error('🚨 预览连续失败3次，暂停预览以避免系统崩溃');
          console.error(`📊 [崩溃统计] 总帧数: ${previewFrameCount}, 失败前间隔: ${actualInterval}ms`);
          stopPreviewPolling();
          errorMessage = '预览功能异常，请重新启动相机';
        }
        
        lastPreviewTime = requestEndTime;
      }
    }, previewInterval);
  }

  // 停止实时预览轮询
  function stopPreviewPolling() {
    isPreviewActive = false;
    previewErrorCount = 0; // 重置错误计数器
    
    if (previewTimer) {
      clearInterval(previewTimer);
      previewTimer = null;
      
      // 输出最终统计
      if (previewFrameCount > 0) {
        const totalTime = Date.now() - previewStartTime;
        const avgFps = (previewFrameCount / totalTime * 1000).toFixed(2);
        console.log(`🏁 [预览停止] 总计处理${previewFrameCount}帧，平均帧率: ${avgFps}fps，总时长: ${totalTime}ms`);
      }
      
      console.log('⏹️ 实时预览已停止');
    }
    
    // 重置统计变量
    previewFrameCount = 0;
    lastPreviewTime = 0;
    previewStartTime = 0;
  }

  // 关闭相机 - 调用Tauri命令
  async function stopCamera() {
    try {
      statusMessage = '正在关闭相机...';
      errorMessage = '';
      
      // 调用后端Tauri命令停止标定会话
      await invoke('stop_calibration_session');
      console.log('标定会话已停止');
      
      calibrationStatus = 'NotStarted';
      statusMessage = '相机已关闭';
      liveImages = { left: null, right: null };
      
      // 停止实时预览
      stopPreviewPolling();
      
      updateButtonStates();
      
    } catch (error) {
      console.error('关闭相机失败:', error);
      errorMessage = `关闭相机失败: ${error}`;
    }
  }

  // 删除图像对 - 调用Tauri命令
  async function deleteImagePair(pairId) {
    try {
      // 调用后端Tauri命令删除图像
              console.log(`🗑️ [删除图像] 删除图像对 ID: ${pairId} (类型: ${typeof pairId})`);
        // 根据错误信息，使用pairId作为参数名
        await invoke('delete_captured_image', { pairId: Number(pairId) });
      console.log('✅ [删除图像] 后端删除成功:', pairId);
      
      // 重新获取图像列表
      capturedImages = await invoke('get_captured_images');
      statusMessage = `已删除图像对 ${pairId}，当前有 ${capturedImages.length} 组图像`;
      
              if (capturedImages.length < 15 && calibrationStatus === 'ReadyToCalibrate') {
          calibrationStatus = 'Capturing';
        }
      updateButtonStates();
      
    } catch (error) {
      console.error('删除图像失败:', error);
      errorMessage = `删除图像失败: ${error}`;
    }
  }

  // 重置系统 - 调用Tauri命令
  async function resetSystem() {
    try {
      statusMessage = '正在重置系统...';
      
      // 调用后端Tauri命令重置工作流程
      await invoke('reset_calibration_workflow');
      console.log('系统已重置');
      
      calibrationStatus = 'NotStarted';
      capturedImages = [];
      liveImages = { left: null, right: null };
      calibrationResult = null;
      errorMessage = '';
      statusMessage = '系统已重置';
      updateButtonStates();
      
    } catch (error) {
      console.error('重置系统失败:', error);
      errorMessage = `重置系统失败: ${error}`;
    }
  }

  // 初始化按钮状态
  updateButtonStates();
</script>

<div class="app-container">
  <!-- 左侧导航栏 -->
  <div class="sidebar">
    <div class="sidebar-header">
      <h2>COSONIC合像软件</h2>
    </div>
    
    <nav class="sidebar-nav">
      <div class="nav-section">
        <h3>相机</h3>
        <ul>
          <li><a href="/AlignmentTest" class="nav-item">🔧 光机合像</a></li>
          <li><a href="/calibration" class="nav-item active">📷 相机标定</a></li>
        </ul>
      </div>
      
      <div class="nav-section">
        <h3>参数设置</h3>
        <ul>
          <li><button class="nav-item disabled" disabled>⚙️ 参数配置</button></li>
        </ul>
      </div>
      
      <div class="nav-section">
        <h3>系统设置</h3>
        <ul>
          <li><button class="nav-item disabled" disabled>📋 许可</button></li>
          <li><button class="nav-item disabled" disabled>💾 文件保存</button></li>
        </ul>
      </div>
    </nav>
  </div>

  <!-- 主内容区域 -->
  <div class="main-content">
    <div class="calibration-test">
      <h1>📷 相机标定工作流程</h1>

  <!-- 状态显示区域 -->
  <div class="status-panel">
    <div class="status-item">
      <label>标定状态:</label>
      <span class="status-badge status-{calibrationStatus.toLowerCase()}">
        {calibrationStatus}
        {#if failedReason}
          <small>({failedReason})</small>
        {/if}
      </span>
    </div>
    
    <div class="status-item">
      <label>状态信息:</label>
      <span class="status-message">{statusMessage}</span>
    </div>

    <div class="status-item">
      <label>采集进度:</label>
      <span class="progress-info">{capturedImages.length} / 15 组图像</span>
      <div class="progress-bar">
        <div class="progress-fill" style="width: {(capturedImages.length / 15) * 100}%"></div>
      </div>
    </div>

    <div class="status-item">
      <label>图像分辨率:</label>
      <span class="resolution-info">2448 × 2048 像素</span>
    </div>

    {#if errorMessage}
      <div class="error-message">
        ❌ {errorMessage}
      </div>
    {/if}
  </div>

  <!-- 控制按钮区域 -->
  <div class="control-panel">
    <div class="button-group">
      <button 
        on:click={startCamera} 
        disabled={!isStartCameraEnabled}
        class="btn-primary"
      >
        📹 启动相机
      </button>
      
      <button 
        on:click={captureImage} 
        disabled={!isCaptureImageEnabled}
        class="btn-success"
      >
        📸 采集图像
      </button>
      
      <button 
        on:click={startCalibration} 
        disabled={!isStartCalibrationEnabled}
        class="btn-warning"
      >
        🎯 开始标定
      </button>
      
      <button 
        on:click={stopCamera} 
        disabled={!isStopCameraEnabled}
        class="btn-danger"
      >
        ⏹️ 关闭相机
      </button>
    </div>

    <div class="button-group secondary">
      <button on:click={resetSystem} class="btn-secondary">
        🔄 重置系统
      </button>
    </div>
  </div>

  <!-- 左右相机实时图像区域 -->
  <div class="live-preview-panel">
    <h3>📺 左右相机实时图像</h3>
    <div class="live-image-container">
      <div class="image-box">
        <h4>左相机</h4>
        {#if liveImages.left}
          <img src="{liveImages.left}" alt="左相机实时图像" />
          <div class="image-status">实时预览中...</div>
        {:else}
          <div class="no-image">
            {calibrationStatus === 'Capturing' ? '等待图像数据...' : '相机未启动'}
          </div>
        {/if}
      </div>
      
      <div class="image-box">
        <h4>右相机</h4>
        {#if liveImages.right}
          <img src="{liveImages.right}" alt="右相机实时图像" />
          <div class="image-status">实时预览中...</div>
        {:else}
          <div class="no-image">
            {calibrationStatus === 'Capturing' ? '等待图像数据...' : '相机未启动'}
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- 标定结果显示区域 -->
  {#if calibrationResult}
    <div class="result-panel">
      <h3>📊 标定结果</h3>
      {#if calibrationResult.success}
        <div class="result-success">
          <div class="result-grid">
            <div class="result-item">
              <label>左相机RMS误差:</label>
              <span class="rms-value">{calibrationResult.left_rms_error.toFixed(3)}</span>
            </div>
            <div class="result-item">
              <label>右相机RMS误差:</label>
              <span class="rms-value">{calibrationResult.right_rms_error.toFixed(3)}</span>
            </div>
            <div class="result-item">
              <label>双目RMS误差:</label>
              <span class="rms-value">{calibrationResult.stereo_rms_error.toFixed(3)}</span>
            </div>
            <div class="result-item">
              <label>标定状态:</label>
              <span class="success-indicator">✓ 标定成功</span>
            </div>
          </div>
        </div>
      {:else}
        <div class="result-failure">
          <span class="failure-indicator">❌ 标定失败</span>
        </div>
      {/if}
    </div>
  {/if}

  <!-- 拍摄标定图像区域 -->
  <div class="captured-images-panel">
    <h3>拍摄标定图像</h3>
    
    {#if capturedImages.length === 0}
      <div class="grid-instruction">
        <strong>📋 拍摄指南：</strong>请按照下列15个位置拍摄标定图像。每个位置需要拍摄左右相机的图像对 (2448×2048分辨率)。
        <br><small>💡 提示：确保标定板完全在相机视野内，光照均匀，避免反光。前9个位置为关键位置，后6个为补充位置。</small>
      </div>
    {/if}
    
    <div class="grid-container">
      {#each Array(15) as _, index}
        {@const imagePair = capturedImages[index]}
        <div class="grid-item" class:has-image={imagePair}>
          <div class="grid-header">
            <span class="position-number">#{index + 1}</span>
            <span class="position-name">
              {index < 9 ? 
                ['最上', '最下', '最左', '最右', '中间', '上斜', '下斜', '左斜', '右斜'][index] : 
                `位置${index + 1}`
              }
            </span>
            {#if imagePair}
              <button 
                class="delete-btn-small" 
                on:click={() => deleteImagePair(imagePair.pair_id)}
                disabled={calibrationStatus === 'Calibrating'}
                title="删除这组图像"
              >
                ×
              </button>
            {/if}
          </div>
          
          {#if imagePair}
            <!-- 已采集的图像对 - 左右排列 -->
            <div class="image-pair-horizontal">
              <div class="image-half">
                <img 
                  src="{imagePair.thumbnail_left}" 
                  alt="左相机标定图像 {imagePair.pair_id}"
                  on:error={(e) => {
                    console.error(`❌ [图像显示] 左图加载失败 (ID:${imagePair.pair_id}):`, e);
                  }}
                />
                <div class="image-label">左相机</div>
              </div>
              <div class="image-half">
                <img 
                  src="{imagePair.thumbnail_right}" 
                  alt="右相机标定图像 {imagePair.pair_id}"
                  on:error={(e) => {
                    console.error(`❌ [图像显示] 右图加载失败 (ID:${imagePair.pair_id}):`, e);
                  }}
                />
                <div class="image-label">右相机</div>
              </div>
            </div>
            <div class="grid-status">
              {#if imagePair.has_calibration_pattern}
                <span class="pattern-indicator success">✓ 检测到标定板</span>
              {:else}
                <span class="pattern-indicator fail">✗ 未检测到标定板</span>
              {/if}
            </div>
          {:else}
            <!-- 空白占位格 -->
            <div class="empty-slot">
              <div class="empty-preview">
                <div class="empty-half">
                  <div class="plus-icon">+</div>
                  <div class="empty-label">左相机</div>
                </div>
                <div class="empty-half">
                  <div class="plus-icon">+</div>
                  <div class="empty-label">右相机</div>
                </div>
              </div>
              <div class="empty-status">待采集</div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  </div>
    </div>
  </div>
</div>

  <style>
  .app-container {
    display: flex;
    min-height: 100vh;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  }

  /* 左侧导航栏 */
  .sidebar {
    width: 240px;
    background: #2c3e50;
    color: white;
    flex-shrink: 0;
    overflow-y: auto;
  }

  .sidebar-header {
    padding: 20px;
    border-bottom: 1px solid #34495e;
  }

  .sidebar-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: bold;
    color: #ecf0f1;
  }

  .sidebar-nav {
    padding: 20px 0;
  }

  .nav-section {
    margin-bottom: 30px;
  }

  .nav-section h3 {
    padding: 0 20px 10px;
    margin: 0;
    font-size: 12px;
    font-weight: bold;
    color: #95a5a6;
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .nav-section ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .nav-section li {
    margin: 0;
  }

  .nav-item {
    display: block;
    width: 100%;
    padding: 12px 20px;
    color: #bdc3c7;
    text-decoration: none;
    transition: all 0.2s;
    font-size: 14px;
    background: transparent;
    border: none;
    text-align: left;
    cursor: pointer;
  }

  .nav-item:hover:not(.disabled):not(:disabled) {
    background: #34495e;
    color: #ecf0f1;
  }

  .nav-item.active {
    background: #3498db;
    color: white;
    font-weight: bold;
  }

  .nav-item.disabled,
  .nav-item:disabled {
    color: #7f8c8d;
    cursor: not-allowed;
    opacity: 0.6;
  }

  /* 主内容区域 */
  .main-content {
    flex: 1;
    overflow-y: auto;
    background: #f8f9fa;
  }

  .calibration-test {
    padding: 20px;
    max-width: 1400px;
    margin: 0 auto;
  }

  h1 {
    text-align: center;
    color: #2c3e50;
    margin-bottom: 30px;
  }

  /* 状态面板 */
  .status-panel {
    background: #f8f9fa;
    border: 1px solid #dee2e6;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
  }

  .status-item {
    display: flex;
    align-items: center;
    margin-bottom: 10px;
  }

  .status-item label {
    font-weight: bold;
    margin-right: 10px;
    min-width: 80px;
  }

  .status-badge {
    padding: 4px 12px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: bold;
    text-transform: uppercase;
  }

  .status-notstarted { background: #6c757d; color: white; }
  .status-capturing { background: #17a2b8; color: white; }
  .status-readytocalibrate { background: #ffc107; color: black; }
  .status-calibrating { background: #fd7e14; color: white; }
  .status-completed { background: #28a745; color: white; }
  .status-failed { background: #dc3545; color: white; }

  .progress-info {
    margin-right: 15px;
    font-weight: bold;
    color: #495057;
  }

  .resolution-info {
    font-family: 'Courier New', monospace;
    font-weight: bold;
    color: #6c757d;
    background: #e9ecef;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
  }

  .progress-bar {
    flex: 1;
    height: 8px;
    background: #e9ecef;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #28a745, #20c997);
    transition: width 0.3s ease;
  }

  .error-message {
    background: #f8d7da;
    color: #721c24;
    padding: 12px;
    border-radius: 6px;
    border: 1px solid #f5c6cb;
    margin-top: 10px;
    font-weight: bold;
    animation: shake 0.5s ease-in-out;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    25% { transform: translateX(-5px); }
    75% { transform: translateX(5px); }
  }

  /* 控制面板 */
  .control-panel {
    margin-bottom: 30px;
  }

  .button-group {
    display: flex;
    gap: 15px;
    margin-bottom: 10px;
    flex-wrap: wrap;
  }

  .button-group.secondary {
    justify-content: center;
  }

  button {
    padding: 12px 24px;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
    font-weight: bold;
    transition: all 0.2s;
    min-width: 120px;
  }

  .btn-primary:not(:disabled) { background: #007bff; color: white; }
  .btn-success:not(:disabled) { background: #28a745; color: white; }
  .btn-warning:not(:disabled) { background: #ffc107; color: black; }
  .btn-danger:not(:disabled) { background: #dc3545; color: white; }
  .btn-secondary:not(:disabled) { background: #6c757d; color: white; }

  button:not(:disabled):hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0,0,0,0.2);
  }

  button:disabled {
    background: #6c757d;
    color: #adb5bd;
    cursor: not-allowed;
    transform: none;
  }

  /* 实时预览面板 */
  .live-preview-panel {
    background: white;
    border: 1px solid #dee2e6;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
  }

  .live-image-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
  }

  .image-box {
    text-align: center;
  }

  .image-box h4 {
    margin: 0 0 10px 0;
    color: #495057;
    font-size: 16px;
  }

  .image-box img {
    max-width: 100%;
    max-height: 300px;
    width: auto;
    height: auto;
    object-fit: contain; /* 保持宽高比，不裁剪 */
    border: 2px solid #28a745;
    border-radius: 4px;
    background: #f8f9fa;
  }

  .image-status {
    margin-top: 5px;
    font-size: 12px;
    color: #28a745;
    font-weight: bold;
  }

  .no-image {
    height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #f8f9fa;
    border: 2px dashed #dee2e6;
    border-radius: 4px;
    color: #6c757d;
    font-style: italic;
  }

  /* 拍摄标定图像面板 */
  .captured-images-panel {
    background: white;
    border: 1px solid #dee2e6;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 20px;
  }

  .grid-container {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 15px;
    margin-top: 20px;
  }

  .grid-item {
    border: 2px solid #dee2e6;
    border-radius: 8px;
    background: white;
    overflow: hidden;
  }

  .grid-item.has-image {
    border-color: #28a745;
  }

  /* 网格头部 */
  .grid-header {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    background: #f8f9fa;
    border-bottom: 1px solid #dee2e6;
    gap: 10px;
  }

  .position-number {
    font-weight: bold;
    color: #6c757d;
    font-size: 12px;
  }

  .position-name {
    flex: 1;
    font-weight: bold;
    color: #495057;
    font-size: 13px;
  }

  .delete-btn-small {
    width: 18px;
    height: 18px;
    border: none;
    background: #dc3545;
    color: white;
    border-radius: 50%;
    font-size: 11px;
    font-weight: bold;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: auto;
    padding: 0;
  }

  .delete-btn-small:hover:not(:disabled) {
    background: #c82333;
  }

  /* 已采集图像的左右排列布局 */
  .image-pair-horizontal {
    display: flex;
    height: 120px;
  }

  .image-half {
    flex: 1;
    position: relative;
    border-right: 1px solid #dee2e6;
  }

  .image-half:last-child {
    border-right: none;
  }

  .image-half img {
    width: 100%;
    height: 100%;
    object-fit: contain; /* 保持宽高比，不裁剪 */
    background: #f8f9fa;
  }

  .image-label {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: rgba(0,0,0,0.7);
    color: white;
    text-align: center;
    font-size: 10px;
    font-weight: bold;
    padding: 2px 4px;
  }

  /* 空白占位格样式 */
  .empty-slot {
    height: 120px;
  }

  .empty-preview {
    display: flex;
    height: 100%;
  }

  .empty-half {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border-right: 1px solid #dee2e6;
    background: #f8f9fa;
  }

  .empty-half:last-child {
    border-right: none;
  }

  .plus-icon {
    font-size: 24px;
    color: #adb5bd;
    margin-bottom: 4px;
  }

  .empty-label {
    font-size: 10px;
    color: #6c757d;
    font-weight: bold;
  }

  .empty-status {
    text-align: center;
    padding: 4px;
    background: #f8f9fa;
    border-top: 1px solid #dee2e6;
    font-size: 11px;
    color: #6c757d;
  }

  /* 网格状态 */
  .grid-status {
    text-align: center;
    padding: 6px 8px;
    background: #f8f9fa;
    border-top: 1px solid #dee2e6;
    font-size: 11px;
  }

  .pattern-indicator {
    font-weight: bold;
  }

  .pattern-indicator.success {
    color: #28a745;
  }

  .pattern-indicator.fail {
    color: #dc3545;
  }

  .grid-instruction {
    text-align: center;
    color: #495057;
    padding: 20px;
    background: linear-gradient(135deg, #e3f2fd, #f3e5f5);
    border: 1px solid #b3e5fc;
    border-radius: 8px;
    margin-top: 20px;
    line-height: 1.5;
  }

  .grid-instruction small {
    color: #6c757d;
    font-style: italic;
    margin-top: 8px;
    display: inline-block;
  }

  /* 结果面板 */
  .result-panel {
    background: white;
    border: 1px solid #dee2e6;
    border-radius: 8px;
    padding: 20px;
  }

  .result-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 15px;
    margin-top: 15px;
  }

  .result-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px;
    background: #f8f9fa;
    border-radius: 4px;
  }

  .result-item label {
    font-weight: bold;
    color: #495057;
  }

  .rms-value {
    font-family: 'Courier New', monospace;
    font-weight: bold;
    color: #28a745;
  }

  .success-indicator {
    color: #28a745;
    font-weight: bold;
  }

  .failure-indicator {
    color: #dc3545;
    font-weight: bold;
  }

  /* 响应式设计 */
  @media (max-width: 1024px) {
    .app-container {
      flex-direction: column;
    }
    
    .sidebar {
      width: 100%;
      height: auto;
    }
    
    .sidebar-nav {
      padding: 10px 0;
    }
    
    .nav-section {
      display: inline-block;
      margin: 0 20px 0 0;
      vertical-align: top;
    }
    
    .nav-section ul {
      display: flex;
      gap: 10px;
    }
    
    .nav-item {
      padding: 8px 16px;
      border-radius: 4px;
      white-space: nowrap;
    }
  }

  @media (max-width: 768px) {
    .live-image-container {
      grid-template-columns: 1fr;
    }
    
    .button-group {
      flex-direction: column;
    }
    
    .result-grid {
      grid-template-columns: 1fr;
    }

    .grid-container {
      grid-template-columns: repeat(3, 1fr);
      gap: 10px;
    }
    
    .image-pair-horizontal {
      height: 100px;
    }
    
    .empty-slot {
      height: 100px;
    }
    
    .plus-icon {
      font-size: 20px;
    }
    
    .empty-label {
      font-size: 9px;
    }
  }

  @media (max-width: 480px) {
    .grid-container {
      grid-template-columns: 1fr;
    }
    
    .nav-section {
      display: block;
      margin-bottom: 15px;
    }
    
    .nav-section ul {
      display: block;
    }
  }
</style> 