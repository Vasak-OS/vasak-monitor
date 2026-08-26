<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import Icono from '@/components/Icono.vue';
import ActionControlsComponent from '@/components/topbar/ActionControlsComponent.vue';

const { t } = useI18n();
</script>
<template>
  <div class="flex justify-between p-1" data-tauri-drag-region>
    <div class="flex flex-1 items-center gap-2 px-1" data-tauri-drag-region>
      <!-- El título va acá y traducido, no en duro: la barra propia reemplaza a la
           del gestor de ventanas, así que si no lo pone nadie la ventana no tiene
           nombre en ninguna parte. -->
      <Icono nombre="utilities-system-monitor" :tamano="16" />
      <span class="text-sm text-tx-main">{{ t('app.titulo') }}</span>
      <slot />
    </div>
    <ActionControlsComponent />
  </div>
</template>