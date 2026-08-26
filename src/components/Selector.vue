<script setup lang="ts">
/**
 * Un `select` que respeta el tema.
 *
 * El `select` nativo de WebKit se dibuja solo: pinta su propio fondo blanco y su
 * propio texto, y las clases de color no lo tocan. En la pantalla de registros eso
 * dejó el selector con texto claro sobre blanco, ilegible. `appearance-none` lo
 * apaga — y entonces la flecha hay que poner a mano, porque desaparece con el
 * resto del dibujo nativo.
 */
import Icono from '@/components/Icono.vue';

// Los atributos van al `select` y no al contenedor: si no, un `@change` o un
// `aria-label` quedan colgados de un `div` y no hacen nada.
defineOptions({ inheritAttrs: false });

const [modelo, modificadores] = defineModel<string | number>({
	required: true,
	set(valor) {
		// `v-model.number` sobre un componente no convierte solo como lo hace sobre
		// un `input`: el modificador llega acá y hay que aplicarlo. Sin esto el
		// intervalo de medición salía como cadena y la validación lo rechazaba.
		return modificadores.number ? Number(valor) : valor;
	},
});
</script>

<template>
	<div class="relative flex min-w-0 items-center">
		<select
			v-model="modelo"
			class="min-w-0 flex-1 appearance-none truncate rounded-corner border border-ui-border bg-ui-surface/60 py-1.5 pr-8 pl-2 text-sm text-tx-main"
			v-bind="$attrs"
		>
			<slot />
		</select>
		<Icono
			nombre="pan-down-symbolic"
			:tamano="14"
			alt=""
			class="pointer-events-none absolute right-2"
		/>
	</div>
</template>
