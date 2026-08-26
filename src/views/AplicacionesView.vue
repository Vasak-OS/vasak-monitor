<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref } from 'vue';
import { useSondeo } from '@/composables/useSondeo';
import { tamano } from '@/tools/formato';
import { interpolar } from '@/tools/interpolar';

interface Aplicacion {
	pid: number;
	nombre: string;
	memoria: number;
	cpu: number | null;
}

const props = defineProps<{ intervalo: number; activa: boolean }>();
const { t } = useI18n();
const lista = ref<Aplicacion[]>([]);
const error = ref('');
const filtro = ref('');
const cerrando = ref<number | null>(null);

useSondeo(
	async () => {
		try {
			lista.value = await invoke<Aplicacion[]>('aplicaciones');
			error.value = '';
		} catch (e) {
			error.value = String(e);
		}
	},
	() => props.intervalo,
	() => props.activa
);

const visibles = computed(() => {
	const q = filtro.value.trim().toLowerCase();
	return q ? lista.value.filter((a) => a.nombre.toLowerCase().includes(q)) : lista.value;
});

async function cerrar(a: Aplicacion) {
	cerrando.value = a.pid;
	error.value = '';
	try {
		await invoke('cerrar', { pid: a.pid });
	} catch (e) {
		error.value = String(e);
	} finally {
		cerrando.value = null;
	}
}
</script>

<template>
	<section class="flex flex-col gap-3">
		<header class="flex items-center gap-3">
			<input
				v-model="filtro"
				type="search"
				:placeholder="t('aplicaciones.buscar')"
				class="w-full rounded-corner border border-ui-border bg-ui-surface/40 px-3 py-1.5 text-sm text-tx-primary"
			/>
			<span class="shrink-0 text-tx-muted text-xs">
				{{ interpolar(t('aplicaciones.cuantas'), visibles.length) }}
			</span>
		</header>

		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>

		<!-- Agrupadas por nombre: un navegador son diez procesos y ninguno es «el
		     consumo de Chrome». -->
		<p class="text-tx-muted text-xs">{{ t('aplicaciones.agrupadas') }}</p>

		<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
			<li
				v-for="a in visibles"
				:key="a.pid"
				class="flex items-center gap-3 bg-ui-surface/40 px-4 py-2.5"
			>
				<span class="min-w-0 flex-1 truncate text-sm text-tx-primary">{{ a.nombre }}</span>
				<span class="shrink-0 font-mono text-sm text-tx-muted">{{ tamano(a.memoria) }}</span>
				<button
					type="button"
					:disabled="cerrando === a.pid"
					class="shrink-0 rounded-corner border border-ui-border px-2.5 py-1 text-tx-muted text-xs hover:bg-ui-surface disabled:opacity-50"
					@click="cerrar(a)"
				>
					{{ t('aplicaciones.cerrar') }}
				</button>
			</li>
		</ul>

		<!-- Se dice qué hace el botón: pedir que se cierre, no matar. Alguien que
		     espera lo segundo y ve que el programa sigue abierto cree que falló. -->
		<p class="text-tx-muted text-xs">{{ t('aplicaciones.cerrarExplicado') }}</p>
	</section>
</template>
