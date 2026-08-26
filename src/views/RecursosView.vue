<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { ref } from 'vue';
import BarraDeCarga from '@/components/BarraDeCarga.vue';
import { useSondeo } from '@/composables/useSondeo';
import { caudal, porcentaje, tamano } from '@/tools/formato';
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

useSondeo(
	async () => {
		try {
			datos.value = await invoke<Recursos>('recursos');
			error.value = '';
		} catch (e) {
			error.value = String(e);
		}
	},
	() => props.intervalo,
	() => props.activa
);

const usoDeRam = (d: Recursos) => (d.ram_total === 0 ? 0 : (d.ram_usada / d.ram_total) * 100);
const usoDeDisco = (d: Disco) => (d.total === 0 ? 0 : (d.usado / d.total) * 100);
</script>

<template>
	<section class="flex flex-col gap-4">
		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>
		<p v-if="!datos" class="text-sm text-tx-muted">{{ t('common.midiendo') }}</p>

		<template v-else>
			<div class="grid gap-3 sm:grid-cols-2">
				<article class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
					<header class="flex items-baseline justify-between">
						<h2 class="font-medium text-tx-primary">{{ t('recursos.cpu') }}</h2>
						<span class="font-mono text-lg text-tx-primary">{{ porcentaje(datos.cpu) }}</span>
					</header>
					<BarraDeCarga :porciento="datos.cpu ?? 0" />
					<p class="text-tx-muted text-xs">
						{{ interpolar(t('recursos.nucleos'), datos.nucleos) }}
					</p>
				</article>

				<article class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
					<header class="flex items-baseline justify-between">
						<h2 class="font-medium text-tx-primary">{{ t('recursos.memoria') }}</h2>
						<span class="font-mono text-lg text-tx-primary">{{ porcentaje(usoDeRam(datos)) }}</span>
					</header>
					<BarraDeCarga :porciento="usoDeRam(datos)" />
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
					<header class="flex items-baseline justify-between">
						<h2 class="font-medium text-tx-primary">{{ t('recursos.swap') }}</h2>
						<span class="font-mono text-lg text-tx-primary">{{ porcentaje(datos.swap) }}</span>
					</header>
					<BarraDeCarga :porciento="datos.swap" />
					<p class="text-tx-muted text-xs">{{ t('recursos.swapExplicado') }}</p>
				</article>

				<article class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
					<h2 class="font-medium text-tx-primary">{{ t('recursos.red') }}</h2>
					<div class="flex gap-6 font-mono text-sm">
						<span class="text-tx-primary">↓ {{ caudal(datos.bajada) }}</span>
						<span class="text-tx-primary">↑ {{ caudal(datos.subida) }}</span>
					</div>
					<p class="text-tx-muted text-xs">{{ t('recursos.redExplicada') }}</p>
				</article>
			</div>

			<article class="flex flex-col gap-3 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
				<h2 class="font-medium text-tx-primary">{{ t('recursos.discos') }}</h2>
				<div v-for="d in datos.discos" :key="d.punto" class="flex flex-col gap-1">
					<div class="flex justify-between text-sm">
						<span class="text-tx-primary">{{ d.punto }}</span>
						<span class="font-mono text-tx-muted text-xs">
							{{ interpolar(t('recursos.deTotal'), tamano(d.usado), tamano(d.total)) }}
						</span>
					</div>
					<BarraDeCarga :porciento="usoDeDisco(d)" />
				</div>
			</article>
		</template>
	</section>
</template>
