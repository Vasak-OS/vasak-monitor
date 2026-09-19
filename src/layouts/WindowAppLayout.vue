<script lang="ts" setup>
/**
 * La ventana del monitor.
 *
 * No dibuja nada propio: el borde, la esquina, el fondo, la barra y los tres
 * botones salen de `WindowFrame`, que es el mismo de todas las ventanas del
 * escritorio. Estaba copiado acá, y ya había derivado de las copias vecinas.
 *
 * De arriba viene además algo que esta copia no tenía: la barra puede ir
 * arriba, abajo, a la izquierda o a la derecha según `window.barPosition` en
 * `~/.config/vasak/vasak.conf`.
 *
 * El título va en `centro`, no como una columna más. Acá estaba resuelto con un
 * tercer hijo vacío que empujaba contra el `justify-between` —el mismo molde
 * que usaban vasak-settings, vasak-gallery y vasak-resonance—, y eso lo deja
 * centrado respecto de lo que sobra entre el icono y los controles, no de la
 * ventana: los tres botones ocupan bastante más que el icono, así que se corre.
 *
 * # Sin `flex` en el contenido
 *
 * Con el contenedor como fila, lo que va en la ranura no recibe `flex-1`, y en
 * una fila un hijo sin crecimiento se encoge al ancho mínimo de su contenido:
 * con `truncate` y `min-w-0` adentro eso es casi cero, así que los datos
 * quedaban en una columna de un píxel con barra de desplazamiento. Como bloque,
 * el hijo ocupa el ancho completo y maneja su propio acomodo.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { WindowFrame } from '@vasakgroup/vue-libvasak';
import ThemeIcon from '@/components/ThemeIcon.vue';

const { t } = useI18n();
</script>

<template>
  <WindowFrame
    :minimize-label="t('ventana.minimizar')"
    :maximize-label="t('ventana.maximizar')"
    :close-label="t('ventana.cerrar')">
    <template #identidad>
      <ThemeIcon nombre="utilities-system-monitor" :tamano="24" :alt="t('app.titulo')" />
    </template>

    <template #centro>
      <div class="font-title text-lg text-tx-main">{{ t('app.titulo') }}</div>
    </template>

    <div class="min-h-0 w-full flex-1">
      <slot />
    </div>
  </WindowFrame>
</template>
