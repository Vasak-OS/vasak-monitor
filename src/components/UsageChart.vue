<script setup lang="ts">
/**
 * Un gráfico de área con las últimas muestras.
 *
 * Es la excepción nombrada de la guardia de diseño (`tests/design-guard.test.ts`):
 * un gráfico de datos no es un icono, y ninguno del tema puede dibujar esto.
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
	series: readonly (number | null)[];
	/**
	 * El valor que llega arriba. Para un porcentaje es 100 fijo; si no se pasa, se
	 * usa el máximo de la serie —la red no tiene techo conocido—.
	 */
	ceiling?: number;
	/** Para el `aria-label`, porque un SVG no dice nada por sí solo. */
	label: string;
	/** El tono, que sigue al de las barras para que el color signifique lo mismo. */
	tone?: 'normal' | 'warning' | 'critical';
}>();

const WIDTH = 300;
const HEIGHT = 56;

/**
 * El techo efectivo.
 *
 * Con el máximo de la serie y sin un mínimo, una serie casi plana se dibuja como
 * una montaña: si todo vale 3 y el techo es 3, la línea va por arriba y parece que
 * está al límite. Por eso se le da un poco de aire.
 */
const effectiveCeiling = computed(() => {
	if (props.ceiling !== undefined) return props.ceiling;
	const max = maximoDe(props.series);
	if (max === null || max <= 0) return 0;
	return max * 1.15;
});

const segments = computed(() => tramosDe(props.series, effectiveCeiling.value, WIDTH, HEIGHT));
const hasChart = computed(() => segments.value.length > 0);

const color = computed(() =>
	props.tone === 'critical'
		? 'text-status-error'
		: props.tone === 'warning'
			? 'text-status-warning'
			: 'text-primary'
);

const areas = computed(() => segments.value.map((s) => comoArea(s, HEIGHT)).filter(Boolean));
const lines = computed(() => segments.value.map((s) => comoPath(s)).filter(Boolean));

/**
 * Los tramos de una sola muestra, que no dibujan línea.
 *
 * Pasa cuando hay huecos de medición alrededor: sin el círculo, esa muestra
 * simplemente no aparece y el gráfico miente por omisión.
 */
const loosePoints = computed(() => segments.value.filter((s) => s.length === 1).map((s) => s[0]));
</script>

<template>
	<svg
		v-if="hasChart"
		:viewBox="`0 0 ${WIDTH} ${HEIGHT}`"
		preserveAspectRatio="none"
		class="h-14 w-full"
		:class="color"
		role="img"
		:aria-label="label"
	>
		<!-- El relleno primero, la línea encima: al revés, el relleno del tramo
		     siguiente tapa el final de la línea del anterior. -->
		<path v-for="(d, i) in areas" :key="`a${i}`" :d="d" fill="currentColor" opacity="0.18" />
		<path
			v-for="(d, i) in lines"
			:key="`l${i}`"
			:d="d"
			fill="none"
			stroke="currentColor"
			stroke-width="1.5"
			vector-effect="non-scaling-stroke"
			stroke-linejoin="round"
		/>
		<circle
			v-for="(p, i) in loosePoints"
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
		<slot name="empty"></slot>
	</div>
</template>
