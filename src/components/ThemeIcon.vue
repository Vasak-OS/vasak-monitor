<script setup lang="ts">
/**
 * Un icono del tema, que sigue al tema.
 *
 * Reactivo porque el pack de iconos puede cambiar en caliente: sin eso, cambiar el
 * tema deja los iconos del anterior hasta reiniciar la aplicación.
 *
 * `tipo` elige la variante que el plugin resuelve: `icon` fuerza la regular —la
 * común, en color— y `symbol` la simbólica monocroma. Por omisión, la común: es la
 * que usa el resto del escritorio, y no todos los nombres del tema tienen versión
 * simbólica.
 *
 * El nombre siempre va al plugin; nunca se arma una ruta de archivo. El tema puede
 * cambiar en caliente y sus rutas no son estables — el plugin resuelve contra el
 * tema activo y avisa cuando cambia.
 */
import { getIconSource, getSymbolSource } from '@vasakgroup/plugin-vicons';
import { computed } from 'vue';
import { useReactiveIcon } from '@/composables/useReactiveIcon';

const props = withDefaults(
	defineProps<{ nombre: string; tipo?: 'icon' | 'symbol'; tamano?: number; alt?: string }>(),
	// `icon` por omisión: los comunes, no los simbólicos.
	//
	// `getSymbolSource` fuerza `FORCE_SYMBOLIC`, o sea la variante monocroma. Con
	// eso, los dos nombres del tema que no tienen versión simbólica —`folder-temp`
	// y `package-broken`— caían a otro tema y se veían distintos del resto. Y
	// además el escritorio usa los comunes en todas sus áreas, así que pedir
	// símbolos hacía que este monitor no se pareciera a lo demás.
	{ tipo: 'icon', tamano: 18 }
);

const fuente = useReactiveIcon(() =>
	props.tipo === 'icon' ? getIconSource(props.nombre) : getSymbolSource(props.nombre)
);

const lado = computed(() => `${props.tamano}px`);
</script>

<template>
	<!-- `alt` vacío cuando el icono acompaña a un texto que ya dice lo mismo: un
	     lector de pantalla no tiene que leer «icono de procesador» antes de
	     «Procesador». Cuando el icono **es** la única etiqueta, quien lo usa pasa
	     un `alt`. -->
	<img
		v-if="fuente"
		:src="fuente"
		:alt="alt ?? ''"
		:style="{ width: lado, height: lado }"
		class="shrink-0"
	/>
	<!-- Un hueco del mismo tamaño mientras resuelve, para que la fila no salte. -->
	<span v-else :style="{ width: lado, height: lado }" class="shrink-0"></span>
</template>
