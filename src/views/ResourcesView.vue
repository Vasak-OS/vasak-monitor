<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { AlertMessage, ConfigSection, LoadingState, ProgressBar } from '@vasakgroup/vue-libvasak';
import { ref } from 'vue';
import UsageChart from '@/components/UsageChart.vue';
import { useSondeo } from '@/composables/useSondeo';
import { caudal, porcentaje, tamano, tonoDeCarga } from '@/tools/formato';
import { agregar } from '@/tools/historial';
import { interpolar } from '@/tools/interpolar';

/** Los nombres de los campos son los del backend: son el formato del cable. */
interface Disk {
	punto: string;
	tipo: string;
	total: number;
	usado: number;
}
interface Resources {
	cpu: number | null;
	nucleos: number;
	ram_usada: number;
	ram_total: number;
	ram_cache: number;
	swap: number | null;
	bajada: number | null;
	subida: number | null;
	discos: Disk[];
}

const props = defineProps<{ interval: number; active: boolean }>();
const { t } = useI18n();
const data = ref<Resources | null>(null);
const error = ref('');

// Declaradas antes de `useSondeo` porque su callback las usa. Hoy funciona igual
// —`useSondeo` mide desde `onMounted`, cuando el setup ya terminó— pero depender
// de ese orden para que un `const` esté inicializado es esperar un ReferenceError.
const ramUsage = (d: Resources) => (d.ram_total === 0 ? 0 : (d.ram_usada / d.ram_total) * 100);
const diskUsage = (d: Disk) => (d.total === 0 ? 0 : (d.usado / d.total) * 100);

/**
 * La historia de cada medida, para los gráficos.
 *
 * En memoria y de la sesión: es lo que hacen todos los monitores del sistema, no
 * escribe nada al disco y no agrega un formato que después haya que migrar.
 *
 * Se guarda una serie por medida y no un arreglo de muestras completas porque cada
 * gráfico dibuja una sola, y así el componente recibe exactamente lo que necesita.
 */
const cpuSeries = ref<(number | null)[]>([]);
const ramSeries = ref<(number | null)[]>([]);
const swapSeries = ref<(number | null)[]>([]);
const downloadSeries = ref<(number | null)[]>([]);
const uploadSeries = ref<(number | null)[]>([]);

useSondeo(
	async () => {
		try {
			const d = await invoke<Resources>('recursos');
			data.value = d;
			cpuSeries.value = agregar(cpuSeries.value, d.cpu);
			ramSeries.value = agregar(ramSeries.value, ramUsage(d));
			swapSeries.value = agregar(swapSeries.value, d.swap);
			downloadSeries.value = agregar(downloadSeries.value, d.bajada);
			uploadSeries.value = agregar(uploadSeries.value, d.subida);
			error.value = '';
		} catch (e) {
			error.value = String(e);
			// El error también es historia: se anota el hueco en lugar de dejar la
			// serie quieta, que dibujaría una línea recta diciendo que todo siguió
			// igual mientras no se pudo medir.
			cpuSeries.value = agregar(cpuSeries.value, null);
			ramSeries.value = agregar(ramSeries.value, null);
			swapSeries.value = agregar(swapSeries.value, null);
			downloadSeries.value = agregar(downloadSeries.value, null);
			uploadSeries.value = agregar(uploadSeries.value, null);
		}
	},
	() => props.interval,
	() => props.active
);
</script>

<template>
	<!-- Cada tarjeta es un `ConfigSection`: el icono del tema, el título y la
	     medida en la ranura `aside`, que se va abajo del título cuando la
	     tarjeta es angosta. Antes eran cinco `article` escritos a mano con el
	     mismo borde y el mismo encabezado. -->
	<section class="flex flex-col gap-4">
		<AlertMessage v-if="error" tone="error" icon="dialog-error">
			{{ error }}
		</AlertMessage>
		<LoadingState v-if="!data" :label="t('common.midiendo')" />

		<template v-else>
			<div class="grid gap-3 @lg:grid-cols-2">
				<ConfigSection :title="t('recursos.cpu')" icon="cpu" icon-type="icon" as="h2">
					<template #aside>
						<span class="font-mono text-lg text-tx-main">{{ porcentaje(data.cpu) }}</span>
					</template>
					<div class="flex min-w-0 flex-col gap-2">
						<ProgressBar
							:value="data.cpu ?? 0"
							:label="t('recursos.cpu')"
							:tone="tonoDeCarga(data.cpu ?? 0)" />
						<UsageChart
							:series="cpuSeries"
							:ceiling="100"
							:tone="tonoDeCarga(data.cpu ?? 0)"
							:label="`${t('recursos.cpu')} — ${t('recursos.historial.titulo')}`"
						>
							<template #empty>{{ t('recursos.historial.sinDatos') }}</template>
						</UsageChart>
						<p class="text-tx-muted text-xs">
							{{ interpolar(t('recursos.nucleos'), data.nucleos) }}
						</p>
					</div>
				</ConfigSection>

				<ConfigSection :title="t('recursos.memoria')" icon="memory" icon-type="icon" as="h2">
					<template #aside>
						<span class="font-mono text-lg text-tx-main">{{ porcentaje(ramUsage(data)) }}</span>
					</template>
					<div class="flex min-w-0 flex-col gap-2">
						<ProgressBar
							:value="ramUsage(data)"
							:label="t('recursos.memoria')"
							:tone="tonoDeCarga(ramUsage(data))" />
						<UsageChart
							:series="ramSeries"
							:ceiling="100"
							:tone="tonoDeCarga(ramUsage(data))"
							:label="`${t('recursos.memoria')} — ${t('recursos.historial.titulo')}`"
						>
							<template #empty>{{ t('recursos.historial.sinDatos') }}</template>
						</UsageChart>
						<p class="text-tx-muted text-xs">
							{{ interpolar(t('recursos.deTotal'), tamano(data.ram_usada), tamano(data.ram_total)) }}
						</p>
						<!-- La caché explicada, no escondida: es lo que evita que alguien
						     intente «liberar» algo que no le está quitando nada. -->
						<p class="text-tx-muted text-xs">
							{{ interpolar(t('recursos.cacheExplicada'), tamano(data.ram_cache)) }}
						</p>
					</div>
				</ConfigSection>

				<ConfigSection
					v-if="data.swap !== null"
					:title="t('recursos.swap')"
					icon="drive-harddisk"
					icon-type="icon"
					as="h2"
				>
					<template #aside>
						<span class="font-mono text-lg text-tx-main">{{ porcentaje(data.swap) }}</span>
					</template>
					<div class="flex min-w-0 flex-col gap-2">
						<ProgressBar
							:value="data.swap"
							:label="t('recursos.swap')"
							:tone="tonoDeCarga(data.swap)" />
						<UsageChart
							:series="swapSeries"
							:ceiling="100"
							:tone="tonoDeCarga(data.swap)"
							:label="`${t('recursos.swap')} — ${t('recursos.historial.titulo')}`"
						>
							<template #empty>{{ t('recursos.historial.sinDatos') }}</template>
						</UsageChart>
						<p class="text-tx-muted text-xs">{{ t('recursos.swapExplicado') }}</p>
					</div>
				</ConfigSection>

				<ConfigSection :title="t('recursos.red')" icon="network-wired" icon-type="icon" as="h2">
					<div class="flex min-w-0 flex-col gap-2">
						<div class="flex gap-6 font-mono text-sm">
							<span class="text-tx-main">↓ {{ caudal(data.bajada) }}</span>
							<span class="text-tx-main">↑ {{ caudal(data.subida) }}</span>
						</div>
						<!-- Sin techo fijo: la red no tiene un máximo conocido, así que cada
						     gráfico se escala contra su propio pico. Van separados porque
						     compartir escala esconde la subida, que suele ser mucho menor. -->
						<UsageChart
							:series="downloadSeries"
							:label="`${t('recursos.red')} ↓ — ${t('recursos.historial.titulo')}`"
						>
							<template #empty>{{ t('recursos.historial.sinDatos') }}</template>
						</UsageChart>
						<UsageChart
							:series="uploadSeries"
							:label="`${t('recursos.red')} ↑ — ${t('recursos.historial.titulo')}`"
						>
							<template #empty>{{ t('recursos.historial.sinDatos') }}</template>
						</UsageChart>
						<p class="text-tx-muted text-xs">{{ t('recursos.redExplicada') }}</p>
					</div>
				</ConfigSection>
			</div>

			<ConfigSection :title="t('recursos.discos')" icon="drive-multidisk" icon-type="icon" as="h2">
				<div class="flex min-w-0 flex-col gap-3">
					<div v-for="d in data.discos" :key="d.punto" class="flex flex-col gap-1">
						<div class="flex flex-wrap items-baseline justify-between gap-x-3 text-sm">
							<span class="min-w-0 truncate text-tx-main">{{ d.punto }}</span>
							<span class="shrink-0 font-mono text-tx-muted text-xs">
								{{ interpolar(t('recursos.deTotal'), tamano(d.usado), tamano(d.total)) }}
							</span>
						</div>
						<ProgressBar
							:value="diskUsage(d)"
							:label="d.punto"
							:tone="tonoDeCarga(diskUsage(d))" />
					</div>
				</div>
			</ConfigSection>
		</template>
	</section>
</template>
