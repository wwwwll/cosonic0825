<!-- AlignmentTest.svelte - 光机合像检测前端界面 -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  // 状态管理
  let cameraStatus = {
    is_camera_active: false,
    current_stage: 'Idle',
    workflow_running: false,
    last_update: Date.now()
  };
  
  let statusMessage = '合像检测系统准备就绪';
  let errorMessage = '';

  // 性能监控
  let actualFps = 0;
  let frameCount = 0;
  let lastFpsUpdate = Date.now();

  // 图像数据
  let previewImages = { left: null, right: null }; // 实时预览图像

  // 检测结果数据
  let leftEyeResult = null;
  let rightEyeResult = null;
  let alignmentResult = null;

  // 按钮状态控制
  let isStartCameraEnabled = true;
  let isStopCameraEnabled = false;

  // 事件监听器和定时器
  let previewUnlisten = null;
  let resultUnlisten = null;
  let statusUnlisten = null;
  let previewTimer = null;
  let isPreviewActive = false;
  let previewInterval = 100; // 100ms更新一次预览 (10fps)

  onMount(async () => {
    console.log('合像检测页面已加载');
    
    // 获取当前状态
    try {
      const status = await invoke('get_alignment_status');
      updateCameraStatus(status);
      console.log('当前合像状态:', status);
      updateButtonStates();
    } catch (error) {
      console.error('获取合像状态失败:', error);
      statusMessage = '获取系统状态失败，请检查后端连接';
      errorMessage = `后端连接失败: ${error}`;
    }

    // 启动状态同步定时器，每5秒同步一次状态，防止前后端状态不一致
    setInterval(async () => {
      if (cameraStatus.is_camera_active) {
        try {
          const status = await invoke('get_alignment_status');
          // 如果后端状态与前端不一致，同步状态
          if (status.is_camera_active !== cameraStatus.is_camera_active) {
            updateCameraStatus(status);
            updateButtonStates();
            if (!status.is_camera_active) {
              // 后端相机已关闭，停止前端轮询
              stopPreviewPolling();
              previewImages = { left: null, right: null };
              leftEyeResult = null;
              rightEyeResult = null;
              alignmentResult = null;
              errorMessage = '相机连接已断开';
            }
          }
        } catch (error) {
          console.error('状态同步失败:', error);
          // 如果状态同步失败，可能是后端异常，显示错误信息
          if (error.toString().includes('相机') || error.toString().includes('连接')) {
            errorMessage = `状态同步失败: ${error}`;
          }
        }
      }
    }, 5000);
  });

  onDestroy(() => {
    if (previewUnlisten) previewUnlisten();
    if (resultUnlisten) resultUnlisten();
    if (statusUnlisten) statusUnlisten();
    if (previewTimer) clearInterval(previewTimer);
  });

  // 更新相机状态
  function updateCameraStatus(status) {
    cameraStatus = status;
    
    if (status.is_camera_active) {
      if (status.workflow_running) {
        statusMessage = '✓ 相机已启动，实时合像检测运行中 (10fps)';
      } else {
        statusMessage = '相机已启动，等待开始检测';
      }
    } else {
      statusMessage = '相机未启动';
    }
  }

  // 更新按钮状态
  function updateButtonStates() {
    isStartCameraEnabled = !cameraStatus.is_camera_active;
    isStopCameraEnabled = cameraStatus.is_camera_active;
  }

  // 启动相机
  async function startCamera() {
    try {
      statusMessage = '正在启动相机...';
      errorMessage = '';
      updateButtonStates();
      
      // 立即显示加载提示
      statusMessage = '⏳ 正在启动相机，首次检测时需要加载重投影矩阵（约30秒），请耐心等待...';
      
      // 调用后端API启动相机
      const status = await invoke('start_alignment_camera');
      console.log('相机启动成功:', status);
      updateCameraStatus(status);
      
      // 相机启动成功，但重投影矩阵将在首次检测时加载
      statusMessage = '✓ 相机已启动，实时合像检测运行中';
      
      // 启动实时预览轮询
      startPreviewPolling();
      
      updateButtonStates();
    } catch (error) {
      console.error('启动相机失败:', error);
      errorMessage = `启动相机失败: ${error}`;
      cameraStatus.is_camera_active = false;
      updateButtonStates();
    }
  }

  // ===== DEBUG START: 可在正式版本中删除 =====
  // 保存调试图像
  async function saveDebugImages() {
    try {
      statusMessage = '正在保存调试图像...';
      errorMessage = '';
      
      const result = await invoke('save_debug_images');
      console.log('调试图像保存成功:', result);
      statusMessage = `✓ 调试图像已保存到 captures/alignment_workflow_debug/`;
      
      // 3秒后恢复原状态信息
      setTimeout(() => {
        if (cameraStatus.is_camera_active) {
          statusMessage = '✓ 相机已启动，实时合像检测运行中 (10fps)';
        }
      }, 3000);
      
    } catch (error) {
      console.error('保存调试图像失败:', error);
      errorMessage = `保存调试图像失败: ${error}`;
    }
  }
  // ===== DEBUG END: 可在正式版本中删除 =====

  // 关闭相机
  async function stopCamera() {
    try {
      statusMessage = '正在关闭相机...';
      errorMessage = '';
      
      // 调用后端API关闭相机
      const status = await invoke('stop_alignment_camera');
      console.log('相机关闭成功:', status);
      updateCameraStatus(status);
      statusMessage = '相机已关闭';
      
      previewImages = { left: null, right: null };
      
      // 清空检测结果
      leftEyeResult = null;
      rightEyeResult = null;
      alignmentResult = null;
      
      // 停止实时预览轮询
      stopPreviewPolling();
      
      updateButtonStates();
    } catch (error) {
      console.error('关闭相机失败:', error);
      errorMessage = `关闭相机失败: ${error}`;
    }
  }

  // 启动实时预览轮询
  function startPreviewPolling() {
    if (previewTimer) clearInterval(previewTimer);
    
    isPreviewActive = true;
    console.log(`启动实时预览，更新间隔: ${previewInterval}ms (10fps)`);
    
        previewTimer = setInterval(async () => {
      if (!isPreviewActive) return;
      
      try {
        // 获取实时预览图像
        const preview = await invoke('get_camera_preview');
        if (preview && isPreviewActive) {
          previewImages = {
            left: preview.left_image_base64,
            right: preview.right_image_base64
          };
          console.log('预览图像更新:', preview.timestamp ? new Date(preview.timestamp).toLocaleTimeString() : 'no timestamp');
        }

        // 获取检测结果
        const deviation = await invoke('get_alignment_deviation');
        if (deviation && isPreviewActive) {
          updateDetectionResults(deviation);
          console.log('检测结果更新:', {
            left_pass: deviation.left_eye?.pose_pass,
            right_pass: deviation.right_eye?.pose_pass,
            alignment_pass: deviation.alignment_pass
          });
        }
        
        // 更新性能指标
        updatePerformanceMetrics();
        
      } catch (error) {
        console.error('获取预览数据失败:', error);
        
        // 处理不同类型的错误
        if (error.toString().includes('相机') || error.toString().includes('连接') || error.toString().includes('硬件')) {
          errorMessage = `相机连接异常: ${error}`;
          // 相机断开时自动停止预览
          stopPreviewPolling();
          cameraStatus.is_camera_active = false;
          updateButtonStates();
        } else if (error.toString().includes('Command') && error.toString().includes('not found')) {
          errorMessage = `后端命令未找到: ${error}`;
        } else {
          // 对于其他错误，只在控制台记录，不影响主流程
          console.warn('轻微错误，继续运行:', error);
        }
      }
    }, previewInterval);
  }

  // 停止实时预览轮询
  function stopPreviewPolling() {
    isPreviewActive = false;
    if (previewTimer) {
      clearInterval(previewTimer);
      previewTimer = null;
      console.log('实时预览已停止');
    }
  }

  // 更新检测结果
  function updateDetectionResults(deviation) {
    leftEyeResult = deviation.left_eye;
    rightEyeResult = deviation.right_eye;
    
    // 只有当有合像状态时才更新合像结果
    if (deviation.alignment_status) {
      alignmentResult = {
        alignment_status: deviation.alignment_status,
        alignment_pass: deviation.alignment_pass,
        adjustment_hint: deviation.adjustment_hint,
        rms_error: deviation.rms_error
      };
    } else {
      alignmentResult = null;
    }
  }



  // 性能监控：更新FPS计算
  function updatePerformanceMetrics() {
    frameCount++;
    const now = Date.now();
    
    if (now - lastFpsUpdate >= 1000) {
      actualFps = frameCount * 1000 / (now - lastFpsUpdate);
      frameCount = 0;
      lastFpsUpdate = now;
    }
  }
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
          <li><a href="/AlignmentTest" class="nav-item active">🔧 光机合像</a></li>
          <li><a href="/calibration" class="nav-item">📷 相机标定</a></li>
        </ul>
      </div>
      
      <div class="nav-section">
        <h3>参数设置</h3>
        <ul>
          <li><a href="#" class="nav-item disabled">⚙️ 参数配置</a></li>
        </ul>
      </div>
      
      <div class="nav-section">
        <h3>系统设置</h3>
        <ul>
          <li><a href="#" class="nav-item disabled">📋 许可</a></li>
          <li><a href="#" class="nav-item disabled">💾 文件保存</a></li>
        </ul>
      </div>
    </nav>
  </div>

  <!-- 主内容区域 -->
  <div class="main-content">
    <div class="alignment-test">
      <h1>🔧 光机合像检测</h1>

      <!-- 状态显示区域 -->
      <div class="status-panel">
        <div class="status-item">
          <label>检测状态:</label>
          <span class="status-badge status-{cameraStatus.is_camera_active ? 'active' : 'inactive'}">
            {cameraStatus.is_camera_active ? '运行中' : '未启动'}
          </span>
        </div>
        
        <div class="status-item">
          <label>状态信息:</label>
          <span class="status-message">
            {statusMessage}
          </span>
        </div>

        {#if cameraStatus.is_camera_active && actualFps > 0}
          <div class="status-item">
            <label>实际帧率:</label>
            <span class="performance-info">
              {actualFps.toFixed(1)} fps
            </span>
          </div>
        {/if}

        {#if errorMessage}
          <div class="error-message">
            ❌ {errorMessage}
          </div>
        {/if}
      </div>

      <!-- 左右相机实时图像区域 -->
      <div class="live-preview-panel">
        <h3 class="panel-title">📺 左右相机实时图像</h3>
        <div class="live-image-container">
          <div class="image-box">
            <h4>左相机</h4>
            {#if previewImages.left}
              <img src="{previewImages.left}" alt="左相机实时图像" />
              <div class="image-status">实时检测中 (10fps)...</div>
            {:else}
              <div class="no-image">
                {cameraStatus.is_camera_active ? '等待图像数据...' : '相机未启动'}
              </div>
            {/if}
          </div>
          
          <div class="image-box">
            <h4>右相机</h4>
            {#if previewImages.right}
              <img src="{previewImages.right}" alt="右相机实时图像" />
              <div class="image-status">实时检测中 (10fps)...</div>
            {:else}
              <div class="no-image">
                {cameraStatus.is_camera_active ? '等待图像数据...' : '相机未启动'}
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- 单眼检测结果区域 -->
      <div class="single-eye-panel">
        <h3 class="panel-title">👁️ 单眼检测结果</h3>
        <div class="eye-results-container">
          <!-- 左眼检测结果 -->
          <div class="eye-result-box">
            <h4>左眼检测</h4>
            {#if leftEyeResult}
              <div class="eye-status {leftEyeResult.pose_pass ? 'pass' : 'fail'}">
                {leftEyeResult.pose_status}
              </div>
              <div class="adjustment-info">
                <div class="adjustment-item">
                  <label>Roll调整:</label>
                  <span class="adjustment-value">{leftEyeResult.roll_adjustment}</span>
                </div>
                <div class="adjustment-item">
                  <label>Pitch调整:</label>
                  <span class="adjustment-value">{leftEyeResult.pitch_adjustment}</span>
                </div>
                <div class="adjustment-item">
                  <label>Yaw调整:</label>
                  <span class="adjustment-value">{leftEyeResult.yaw_adjustment}</span>
                </div>
                {#if leftEyeResult.centering_status}
                  <div class="adjustment-item">
                    <label>居中调整:</label>
                    <span class="adjustment-value {leftEyeResult.centering_pass ? 'pass' : 'fail'}">{leftEyeResult.centering_adjustment}</span>
                  </div>
                {/if}
              </div>
            {:else}
              <div class="no-result">
                {cameraStatus.is_camera_active ? '等待检测数据...' : '相机未启动'}
              </div>
            {/if}
          </div>

          <!-- 右眼检测结果 -->
          <div class="eye-result-box">
            <h4>右眼检测</h4>
            {#if rightEyeResult}
              <div class="eye-status {rightEyeResult.pose_pass ? 'pass' : 'fail'}">
                {rightEyeResult.pose_status}
              </div>
              <div class="adjustment-info">
                <div class="adjustment-item">
                  <label>Roll调整:</label>
                  <span class="adjustment-value">{rightEyeResult.roll_adjustment}</span>
                </div>
                <div class="adjustment-item">
                  <label>Pitch调整:</label>
                  <span class="adjustment-value">{rightEyeResult.pitch_adjustment}</span>
                </div>
                <div class="adjustment-item">
                  <label>Yaw调整:</label>
                  <span class="adjustment-value">{rightEyeResult.yaw_adjustment}</span>
                </div>
              </div>
            {:else}
              <div class="no-result">
                {cameraStatus.is_camera_active ? '等待检测数据...' : '相机未启动'}
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- 双眼合像检测结果区域 -->
      <div class="dual-eye-panel">
        <h3 class="panel-title">👀 双眼合像检测结果</h3>
        {#if alignmentResult}
          <div class="alignment-result-box">
            <div class="alignment-status {alignmentResult.alignment_pass ? 'pass' : 'fail'}">
              {alignmentResult.alignment_status}
            </div>
            {#if alignmentResult.rms_error !== null}
              <div class="rms-error">
                <label>RMS误差:</label>
                <span class="rms-value">{alignmentResult.rms_error.toFixed(3)} px</span>
              </div>
            {/if}
            {#if alignmentResult.adjustment_hint}
              <div class="adjustment-hint">
                <strong>调整建议:</strong> {alignmentResult.adjustment_hint}
              </div>
            {/if}
          </div>
        {:else}
          <div class="no-alignment-result">
            {cameraStatus.is_camera_active ? '等待合像检测数据...' : '相机未启动'}
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
            on:click={stopCamera} 
            disabled={!isStopCameraEnabled}
            class="btn-danger"
          >
            ⏹️ 关闭相机
          </button>
          
          <!-- ===== DEBUG START: 可在正式版本中删除 ===== -->
          <button 
            on:click={saveDebugImages}
            disabled={!cameraStatus.is_camera_active}
            class="btn-debug"
            title="保存当前帧的调试图像到项目根目录"
          >
            🐛 保存调试图像
          </button>
          <!-- ===== DEBUG END: 可在正式版本中删除 ===== -->
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

  /* 左侧导航栏样式（复用标定页面的样式） */
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
    padding: 12px 20px;
    color: #bdc3c7;
    text-decoration: none;
    transition: all 0.2s;
    font-size: 14px;
  }

  .nav-item:hover:not(.disabled) {
    background: #34495e;
    color: #ecf0f1;
  }

  .nav-item.active {
    background: #3498db;
    color: white;
    font-weight: bold;
  }

  .nav-item.disabled {
    color: #7f8c8d;
    cursor: not-allowed;
  }

  /* 主内容区域 */
  .main-content {
    flex: 1;
    overflow-y: auto;
    background: #f8f9fa;
  }

  .alignment-test {
    padding: 15px;
    max-width: 1400px;
    margin: 0 auto;
  }

  h1 {
    text-align: center;
    color: #2c3e50;
    margin-bottom: 20px;
    font-size: 24px;
  }



  /* 面板标题样式 */
  .panel-title {
    margin: 0 0 10px 0;
    color: #495057;
    font-size: 16px;
    font-weight: bold;
  }

  /* 状态面板 - 压缩高度 */
  .status-panel {
    background: #f8f9fa;
    border: 1px solid #dee2e6;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 15px;
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

  .status-active { background: #28a745; color: white; }
  .status-inactive { background: #6c757d; color: white; }

  .status-message {
    color: #495057;
  }

  .performance-info {
    font-family: 'Courier New', monospace;
    font-size: 12px;
    color: #007bff;
    font-weight: bold;
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

  /* 实时预览面板 - 压缩高度 */
  .live-preview-panel {
    background: white;
    border: 1px solid #dee2e6;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 15px;
  }

  .live-image-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .image-box {
    text-align: center;
  }

  .image-box h4 {
    margin: 0 0 6px 0;
    color: #495057;
    font-size: 14px;
  }

  .image-box img {
    max-width: 100%;
    max-height: 200px;
    width: auto;
    height: auto;
    object-fit: contain; /* 保持宽高比，适应任何分辨率 */
    border: 2px solid #28a745;
    border-radius: 4px;
    background: #f8f9fa;
    /* 图像自适应显示 - 支持从1x1到2448x2048的任何尺寸 */
    image-rendering: auto;
    image-rendering: -webkit-optimize-contrast; /* 优化小图像显示 */
  }

  .image-status {
    margin-top: 3px;
    font-size: 11px;
    color: #28a745;
    font-weight: bold;
  }

  .no-image {
    height: 150px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #f8f9fa;
    border: 2px dashed #dee2e6;
    border-radius: 4px;
    color: #6c757d;
    font-style: italic;
    font-size: 12px;
  }

  /* 单眼检测结果面板 - 压缩高度 */
  .single-eye-panel {
    background: white;
    border: 1px solid #dee2e6;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 15px;
  }

  .eye-results-container {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .eye-result-box {
    border: 1px solid #dee2e6;
    border-radius: 6px;
    padding: 10px;
    background: #f8f9fa;
  }

  .eye-result-box h4 {
    margin: 0 0 6px 0;
    color: #495057;
    font-size: 14px;
    text-align: center;
  }

  .eye-status {
    text-align: center;
    font-weight: bold;
    padding: 4px;
    border-radius: 3px;
    margin-bottom: 6px;
    font-size: 12px;
  }

  .eye-status.pass {
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
  }

  .eye-status.fail {
    background: #f8d7da;
    color: #721c24;
    border: 1px solid #f5c6cb;
  }

  .adjustment-info {
    margin-top: 6px;
  }

  .adjustment-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
    padding: 3px 6px;
    background: white;
    border-radius: 3px;
  }

  .adjustment-item label {
    font-weight: bold;
    color: #495057;
    font-size: 11px;
  }

  .adjustment-value {
    font-size: 11px;
    color: #6c757d;
  }

  .adjustment-value.pass {
    color: #28a745;
    font-weight: bold;
  }

  .adjustment-value.fail {
    color: #dc3545;
    font-weight: bold;
  }

  .no-result {
    text-align: center;
    color: #6c757d;
    font-style: italic;
    padding: 20px 10px;
    font-size: 12px;
  }

  /* 双眼合像检测结果面板 - 压缩高度 */
  .dual-eye-panel {
    background: white;
    border: 1px solid #dee2e6;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 15px;
  }

  .alignment-result-box {
    border: 1px solid #dee2e6;
    border-radius: 6px;
    padding: 12px;
    background: #f8f9fa;
  }

  .alignment-status {
    text-align: center;
    font-weight: bold;
    font-size: 14px;
    padding: 8px;
    border-radius: 4px;
    margin-bottom: 8px;
  }

  .alignment-status.pass {
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
  }

  .alignment-status.fail {
    background: #f8d7da;
    color: #721c24;
    border: 1px solid #f5c6cb;
  }

  .rms-error {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 8px;
    background: white;
    border-radius: 3px;
    margin-bottom: 6px;
  }

  .rms-error label {
    font-weight: bold;
    color: #495057;
    font-size: 12px;
  }

  .rms-value {
    font-family: 'Courier New', monospace;
    font-weight: bold;
    color: #28a745;
    font-size: 12px;
  }

  .adjustment-hint {
    padding: 8px;
    background: #fff3cd;
    border: 1px solid #ffeaa7;
    border-radius: 3px;
    color: #856404;
    line-height: 1.3;
    font-size: 12px;
  }

  .no-alignment-result {
    text-align: center;
    color: #6c757d;
    font-style: italic;
    padding: 20px 10px;
    font-size: 12px;
  }

  /* 控制面板 - 压缩高度 */
  .control-panel {
    margin-bottom: 15px;
  }

  .button-group {
    display: flex;
    gap: 15px;
    justify-content: center;
    flex-wrap: wrap;
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
  .btn-danger:not(:disabled) { background: #dc3545; color: white; }

  /* ===== DEBUG START: 可在正式版本中删除 ===== */
  .btn-debug:not(:disabled) { 
    background: #f39c12; 
    color: white; 
  }

  .btn-debug:not(:disabled):hover {
    background: #e67e22;
  }
  /* ===== DEBUG END: 可在正式版本中删除 ===== */

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
    
    .eye-results-container {
      grid-template-columns: 1fr;
    }
    
    .button-group {
      flex-direction: column;
    }
  }

  @media (max-width: 480px) {    
    .nav-section {
      display: block;
      margin-bottom: 15px;
    }
    
    .nav-section ul {
      display: block;
    }
  }
</style> 