<script setup lang="ts">
/**
 * Un gráfico de área con las últimas muestras.
 *
 * SVG a mano y no una biblioteca de gráficos. Traer una acá sería sumar cientos de
 * kilobytes al monitor —la aplicación que muestra el consumo no puede ser la
 * primera de su propia lista— para dibujar una línea y un relleno. Las cuentas
 * están en `tools/historial.ts`, probadas aparte.
 *
 * El `viewBox` es fijo y el SVG se estira con CSS: así el gráfico se adapta al
 * ancho de su tarjeta sin recalcular nada al cambiar de tamaño la ventana.
 */
import { computed } from 'vue';
import { comoArea, comoPath, maximoDe, tramosDe } from '@/tools/historial';

const props = defineProps<{
	serie: readonly (number | null)[];
	/**
	 * El valor que llega arriba. Para un porcentaje es 100 fijo; si no se pasa, se
	 * usa el máximo de la serie —la red no tiene techo conocido—.
	 */
	techo?: number;
	/** Para el `aria-label`, porque un SVG no dice nada por sí solo. */
	etiqueta: string;
	/** El tono, que sigue al de las barras para que el color signifique lo mismo. */
	tono?: 'normal' | 'atencion' | 'critico';
}>();

const ANCHO = 300;
const ALTO = 56;

/**
 * El techo efectivo.
 *
 * Con el máximo de la serie y sin un mínimo, una serie casi plana se dibuja como
 * una montaña: si todo vale 3 y el techo es 3, la línea va por arriba y parece que
 * está al límite. Por eso se le da un poco de aire.
 */
const techoReal = computed(() => {
	if (props.techo !== undefined) return props.techo;
	const max = maximoDe(props.serie);
	if (max === null || max <= 0) return 0;
	return max * 1.15;
});

const tramos = computed(() => tramosDe(props.serie, techoReal.value, ANCHO, ALTO));
const hayGrafico = computed(() => tramos.value.length > 0);

const color = computed(() =>
	props.tono === 'critico'
		? 'text-status-error'
		: props.tono === 'atencion'
			? 'text-status-warning'
			: 'text-primary'
);

const areas = computed(() => tramos.value.map((t) => comoArea(t, ALTO)).filter(Boolean));
const lineas = computed(() => tramos.value.map((t) => comoPath(t)).filter(Boolean));

/**
 * Los tramos de una sola muestra, que no dibujan línea.
 *
 * Pasa cuando hay huecos de medición alrededor: sin el círculo, esa muestra
 * simplemente no aparece y el gráfico miente por omisión.
 */
const puntosSueltos = computed(() => tramos.value.filter((t) => t.length === 1).map((t) => t[0]));
</script>

<template>
	<svg
		v-if="hayGrafico"
		:viewBox="`0 0 ${ANCHO} ${ALTO}`"
		preserveAspectRatio="none"
		class="h-14 w-full"
		:class="color"
		role="img"
		:aria-label="etiqueta"
	>
		<!-- El relleno primero, la línea encima: al revés, el relleno del tramo
		     siguiente tapa el final de la línea del anterior. -->
		<path v-for="(d, i) in areas" :key="`a${i}`" :d="d" fill="currentColor" opacity="0.18" />
		<path
			v-for="(d, i) in lineas"
			:key="`l${i}`"
			:d="d"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			vector-effect="non-scaling-stroke"
			stroke-linejoin="round"
		/>
		<circle
			v-for="(p, i) in puntosSueltos"
			:key="`p${i}`"
			:cx="p.x"
			:cy="p.y"
			r="1.5"
			fill="currentColor"
		/>
	</svg>
	<!-- Sin muestras suficientes no se dibuja un gráfico vacío, que se lee como
	     «no pasa nada» en lugar de «todavía no medí». -->
	<div v-else class="flex h-14 items-center justify-center text-tx-muted text-xs">
		<slot name="vacio"></slot>
	</div>
</template>
