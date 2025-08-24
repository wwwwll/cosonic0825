import { listen } from '@tauri-apps/api/event';

listen('camera-frame', event => {
  const [leftB64, rightB64] = event.payload;
  leftImage.src  = `data:image/jpeg;base64,${leftB64}`;
  rightImage.src = `data:image/jpeg;base64,${rightB64}`;
});

<script>
  import { invoke } from '@tauri-apps/api/tauri';
  let captures = [];

  async function onCapture() {
    const [leftB64, rightB64] = await invoke('capture');
    captures = [...captures, { left: leftB64, right: rightB64 }];
  }
</script>

<button on:click={onCapture}>拍一张</button>

{#each captures as {left, right}, i}
  <div>第 {i+1} 组：</div>
  <img src={`data:image/jpeg;base64,${left}`} />
  <img src={`data:image/jpeg;base64,${right}`} />
{/each}
