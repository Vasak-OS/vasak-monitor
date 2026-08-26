<script lang="ts" setup>
import TopBarComponent from '@/components/topbar/TopBarComponent.vue';
</script>
<template>
  <div
    class="flex h-screen w-screen flex-col overflow-hidden rounded-corner-window border border-ui-border bg-ui-bg/80">
    <TopBarComponent />
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
