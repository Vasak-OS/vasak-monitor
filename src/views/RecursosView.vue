<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import BarraDeCarga from '@/components/BarraDeCarga.vue';
import GraficoDeUso from '@/components/GraficoDeUso.vue';
import ThemeIcon from '@/components/ThemeIcon.vue';
import { useSondeo } from '@/composables/useSondeo';
import { caudal, porcentaje, tamano, tonoDeCarga } from '@/tools/formato';
import { agregar } from '@/tools/historial';
import { interpolar } from '@/tools/interpolar';

interface Disco {
	punto: string;
	tipo: string;
	total: number;
	usado: number;
}
interface Recursos {
	cpu: number | null;
	nucleos: number;
	ram_usada: number;
	ram_total: number;
	ram_cache: number;
	swap: number | null;
	bajada: number | null;
	subida: number | null;
	discos: Disco[];
}

const props = defineProps<{ intervalo: number; activa: boolean }>();
const { t } = useI18n();
const datos = ref<Recursos | null>(null);
const error = ref('');

// Declaradas antes de `useSondeo` porque su callback las usa. Hoy funciona igual
// —`useSondeo` mide desde `onMounted`, cuando el setup ya terminó— pero depender
// de ese orden para que un `const` esté inicializado es esperar un ReferenceError.
const usoDeRam = (d: Recursos) => (d.ram_total === 0 ? 0 : (d.ram_usada / d.ram_total) * 100);
const usoDeDisco = (d: Disco) => (d.total === 0 ? 0 : (d.usado / d.total) * 100);

/**
 * La historia de cada medida, para los gráficos.
 *
 * En memoria y de la sesión: es lo que hacen todos los monitores del sistema, no
 * escribe nada al disco y no agrega un formato que después haya que migrar.
 *
 * Se guarda una serie por medida y no un arreglo de muestras completas porque cada
 * gráfico dibuja una sola, y así el componente recibe exactamente lo que necesita.
 */
const serieCpu = ref<(number | null)[]>([]);
const serieRam = ref<(number | null)[]>([]);
const serieSwap = ref<(number | null)[]>([]);
const serieBajada = ref<(number | null)[]>([]);
const serieSubida = ref<(number | null)[]>([]);

useSondeo(
	async () => {
		try {
			const d = await invoke<Recursos>('recursos');
			datos.value = d;
			serieCpu.value = agregar(serieCpu.value, d.cpu);
			serieRam.value = agregar(serieRam.value, usoDeRam(d));
			serieSwap.value = agregar(serieSwap.value, d.swap);
			serieBajada.value = agregar(serieBajada.value, d.bajada);
			serieSubida.value = agregar(serieSubida.value, d.subida);
			error.value = '';
		} catch (e) {
			error.value = String(e);
			// El error también es historia: se anota el hueco en lugar de dejar la
			// serie quieta, que dibujaría una línea recta diciendo que todo siguió
			// igual mientras no se pudo medir.
			serieCpu.value = agregar(serieCpu.value, null);
			serieRam.value = agregar(serieRam.value, null);
			serieSwap.value = agregar(serieSwap.value, null);
			serieBajada.value = agregar(serieBajada.value, null);
			serieSubida.value = agregar(serieSubida.value, null);
		}
	},
	() => props.intervalo,
	() => props.activa
);
</script>

<template>
	<section class="flex flex-col gap-4">
		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>
		<p v-if="!datos" class="text-sm text-tx-muted">{{ t('common.midiendo') }}</p>

		<template v-else>
			<div class="grid gap-3 @lg:grid-cols-2">
				<article class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
					<header class="flex flex-wrap items-baseline justify-between gap-x-3">
						<h2 class="flex items-center gap-2 font-medium text-tx-main">
							<ThemeIcon nombre="cpu" :tamano="18" />
							{{ t('recursos.cpu') }}
						</h2>
						<span class="font-mono text-lg text-tx-main">{{ porcentaje(datos.cpu) }}</span>
					</header>
					<BarraDeCarga :porciento="datos.cpu ?? 0" />
					<GraficoDeUso
						:serie="serieCpu"
						:techo="100"
						:tono="tonoDeCarga(datos.cpu ?? 0)"
						:etiqueta="`${t('recursos.cpu')} — ${t('recursos.historial.titulo')}`"
					>
						<template #vacio>{{ t('recursos.historial.sinDatos') }}</template>
					</GraficoDeUso>
					<p class="text-tx-muted text-xs">
						{{ interpolar(t('recursos.nucleos'), datos.nucleos) }}
					</p>
				</article>

				<article class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
					<header class="flex flex-wrap items-baseline justify-between gap-x-3">
						<h2 class="flex items-center gap-2 font-medium text-tx-main">
							<ThemeIcon nombre="memory" :tamano="18" />
							{{ t('recursos.memoria') }}
						</h2>
						<span class="font-mono text-lg text-tx-main">{{ porcentaje(usoDeRam(datos)) }}</span>
					</header>
					<BarraDeCarga :porciento="usoDeRam(datos)" />
					<GraficoDeUso
						:serie="serieRam"
						:techo="100"
						:tono="tonoDeCarga(usoDeRam(datos))"
						:etiqueta="`${t('recursos.memoria')} — ${t('recursos.historial.titulo')}`"
					>
						<template #vacio>{{ t('recursos.historial.sinDatos') }}</template>
					</GraficoDeUso>
					<p class="text-tx-muted text-xs">
						{{ interpolar(t('recursos.deTotal'), tamano(datos.ram_usada), tamano(datos.ram_total)) }}
					</p>
					<!-- La caché explicada, no escondida: es lo que evita que alguien
					     intente «liberar» algo que no le está quitando nada. -->
					<p class="text-tx-muted text-xs">
						{{ interpolar(t('recursos.cacheExplicada'), tamano(datos.ram_cache)) }}
					</p>
				</article>

				<article
					v-if="datos.swap !== null"
					class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4"
				>
					<header class="flex flex-wrap items-baseline justify-between gap-x-3">
						<h2 class="flex items-center gap-2 font-medium text-tx-main">
							<ThemeIcon nombre="drive-harddisk" :tamano="18" />
							{{ t('recursos.swap') }}
						</h2>
						<span class="font-mono text-lg text-tx-main">{{ porcentaje(datos.swap) }}</span>
					</header>
					<BarraDeCarga :porciento="datos.swap" />
					<GraficoDeUso
						:serie="serieSwap"
						:techo="100"
						:tono="tonoDeCarga(datos.swap)"
						:etiqueta="`${t('recursos.swap')} — ${t('recursos.historial.titulo')}`"
					>
						<template #vacio>{{ t('recursos.historial.sinDatos') }}</template>
					</GraficoDeUso>
					<p class="text-tx-muted text-xs">{{ t('recursos.swapExplicado') }}</p>
				</article>

				<article class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
					<h2 class="flex items-center gap-2 font-medium text-tx-main">
							<ThemeIcon nombre="network-wired" :tamano="18" />
							{{ t('recursos.red') }}
						</h2>
					<div class="flex gap-6 font-mono text-sm">
						<span class="text-tx-main">↓ {{ caudal(datos.bajada) }}</span>
						<span class="text-tx-main">↑ {{ caudal(datos.subida) }}</span>
					</div>
					<!-- Sin techo fijo: la red no tiene un máximo conocido, así que cada
					     gráfico se escala contra su propio pico. Van separados porque
					     compartir escala esconde la subida, que suele ser mucho menor. -->
					<GraficoDeUso
						:serie="serieBajada"
						:etiqueta="`${t('recursos.red')} ↓ — ${t('recursos.historial.titulo')}`"
					>
						<template #vacio>{{ t('recursos.historial.sinDatos') }}</template>
					</GraficoDeUso>
					<GraficoDeUso
						:serie="serieSubida"
						:etiqueta="`${t('recursos.red')} ↑ — ${t('recursos.historial.titulo')}`"
					>
						<template #vacio>{{ t('recursos.historial.sinDatos') }}</template>
					</GraficoDeUso>
					<p class="text-tx-muted text-xs">{{ t('recursos.redExplicada') }}</p>
				</article>
			</div>

			<article class="flex flex-col gap-3 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
				<h2 class="flex items-center gap-2 font-medium text-tx-main">
							<ThemeIcon nombre="drive-multidisk" :tamano="18" />
							{{ t('recursos.discos') }}
						</h2>
				<div v-for="d in datos.discos" :key="d.punto" class="flex flex-col gap-1">
					<div class="flex flex-wrap items-baseline justify-between gap-x-3 text-sm">
						<span class="truncate text-tx-main">{{ d.punto }}</span>
						<span class="shrink-0 font-mono text-tx-muted text-xs">
							{{ interpolar(t('recursos.deTotal'), tamano(d.usado), tamano(d.total)) }}
						</span>
					</div>
					<BarraDeCarga :porciento="usoDeDisco(d)" />
				</div>
			</article>
		</template>
	</section>
</template>
