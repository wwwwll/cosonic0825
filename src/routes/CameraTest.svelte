<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  //import { invoke } from "D:/rust_projects/merging_image/node_modules/@tauri-apps/api/tauri";

  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  // 预览帧（Base64 JPEG）
  let previewLeft = '';
  let previewRight = '';
  // 抓拍结果：最多三组
  let captures: { left: string; right: string }[] = [];

  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    // 监听后端 emit 的 "camera-frame" 事件
    unlisten = await listen<[string, string]>('camera-frame', event => {
      const [l, r] = event.payload;
      previewLeft = `data:image/jpeg;base64,${l}`;
      previewRight = `data:image/jpeg;base64,${r}`;
    });
  });

  onDestroy(() => {
    // 取消监听
    unlisten && unlisten();
  });

  // “开始采集” 按钮
  async function startPreview() {
    try {
      await invoke('start_cam');
    } catch (err) {
      console.error('start_cam error', err);
    }
  }

  // “结束采集” 按钮
  async function stopPreview() {
    try {
      await invoke('stop_cam');
      // 清空预览
      previewLeft = '';
      previewRight = '';
    } catch (err) {
      console.error('stop_cam error', err);
    }
  }

  // “拍摄图像” 按钮
  async function captureOnce() {
    try {
      const [l, r] = await invoke<[string, string]>('capture_cam');
      const left = `data:image/jpeg;base64,${l}`;
      const right = `data:image/jpeg;base64,${r}`;
      captures = [...captures, { left, right }];
      // 如果只保留最新三组
      if (captures.length > 3) {
        captures = captures.slice(captures.length - 3);
      }
    } catch (err) {
      console.error('capture_cam error', err);
    }
  }
</script>

<style>
  .preview, .captures { display: flex; gap: 1rem; }
  img { max-width: 200px; border: 1px solid #ccc; }
</style>

<div>
  <button on:click={startPreview}>开始采集</button>
  <button on:click={stopPreview}>结束采集</button>
  <button on:click={captureOnce}>拍摄图像</button>
</div>

<h3>实时预览（双目）</h3>
<div class="preview">
  {#if previewLeft}
    <img src={previewLeft} alt="Left Preview" />
  {/if}
  {#if previewRight}
    <img src={previewRight} alt="Right Preview" />
  {/if}
  {#if !previewLeft && !previewRight}
    <em>尚未开始采集</em>
  {/if}
</div>

<h3>抓拍结果（最多三组）</h3>
<div class="captures">
  {#each captures as cap, i}
    <div>
      <div>第 {i+1} 张左：</div>
      <img src={cap.left} alt="Left Capture" />
      <div>第 {i+1} 张右：</div>
      <img src={cap.right} alt="Right Capture" />
    </div>
  {/each}
  {#if captures.length === 0}
    <em>尚未拍摄任何图像</em>
  {/if}
</div>
