<script lang="ts" setup>
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import ThemeIcon from '@/components/ThemeIcon.vue';
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';

const { t } = useI18n();
</script>
<template>
  <div
    class="flex h-screen w-screen flex-col overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/80">
    <!-- Tres hijos y no dos: el espaciador vacío es lo que hace que el
         `justify-between` de la barra deje el título en el centro. Es el mismo
         molde que usan vasak-settings, vasak-gallery y vasak-resonance; acá el
         título iba pegado al icono, a la izquierda, y con `text-sm` en vez de la
         tipografía que usa el resto. -->
    <TopBarComponent>
      <ThemeIcon nombre="utilities-system-monitor" :tamano="24" :alt="t('app.titulo')" />
      <div class="text-lg font-semibold text-tx-main">{{ t('app.titulo') }}</div>
      <div></div>
    </TopBarComponent>
    <!-- Con `slot`, que es lo que la plantilla no traía: sin él el layout
         descartaba en silencio todo lo que se le pusiera dentro, y la ventana
         abría vacía con el «VAPP» de relleno todavía puesto.
         
         Y **sin** `flex` acá. Con el contenedor como fila, lo que va en el slot no
         recibe `flex-1`, y en una fila un hijo sin crecimiento se encoge al ancho
         mínimo de su contenido: con `truncate` y `min-w-0` adentro eso es casi
         cero, así que los datos quedaban en una columna de un píxel con barra de
         desplazamiento. Como bloque, el hijo ocupa el ancho completo y maneja su
         propio layout. -->
    <div class="min-h-0 flex-1">
      <slot />
    </div>
  </div>
</template>
