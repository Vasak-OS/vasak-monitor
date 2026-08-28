<script lang="ts" setup>
import ActionControlsComponent from '@/components/topbar/ActionControlsComponent.vue';
</script>
<template>
  <div class="flex justify-between p-1" data-tauri-drag-region>
    <!-- `justify-between` con lo que venga del slot: el layout pone el icono, el
         título y un espaciador vacío, y así el título queda centrado en la barra
         como en el resto de las aplicaciones del escritorio. -->
    <div class="flex flex-1 items-center justify-between gap-2 px-1" data-tauri-drag-region>
      <slot />
    </div>
    <ActionControlsComponent />
  </div>
</template>