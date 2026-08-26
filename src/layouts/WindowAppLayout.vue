<script lang="ts" setup>
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
</script>
<template>
  <div
    class="flex h-screen w-screen flex-col overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/80">
    <TopBarComponent />
    <!-- Con `slot`, que es lo que la plantilla no traía: sin él el layout
         descartaba en silencio todo lo que se le pusiera dentro, y la ventana
         abría vacía con el «VAPP» de relleno todavía puesto. -->
    <div class="flex min-h-0 flex-1">
      <slot />
    </div>
  </div>
</template>
