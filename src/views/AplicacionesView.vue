<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref } from 'vue';
import Icono from '@/components/Icono.vue';
import { useSondeo } from '@/composables/useSondeo';
import { tamano } from '@/tools/formato';
import { interpolar } from '@/tools/interpolar';

interface Aplicacion {
	pid: number;
	nombre: string;
	memoria: number;
	cpu: number | null;
	con_ventana: boolean;
}

const props = defineProps<{ intervalo: number; activa: boolean }>();
const { t } = useI18n();
const lista = ref<Aplicacion[]>([]);
const error = ref('');
const filtro = ref('');
const mostrarSinVentana = ref(false);
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

const coincide = (a: Aplicacion) => {
	const q = filtro.value.trim().toLowerCase();
	return !q || a.nombre.toLowerCase().includes(q);
};

const conVentana = computed(() => lista.value.filter((a) => a.con_ventana && coincide(a)));
const sinVentana = computed(() => lista.value.filter((a) => !a.con_ventana && coincide(a)));

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
		<!-- `flex-wrap` y anchos mínimos: en una ventana angosta el buscador y el
		     contador se apilan en lugar de comprimirse hasta ser ilegibles. -->
		<header class="flex flex-wrap items-center gap-2">
			<input
				v-model="filtro"
				type="search"
				:placeholder="t('aplicaciones.buscar')"
				class="min-w-40 flex-1 rounded-corner border border-ui-border bg-ui-surface/40 px-3 py-1.5 text-sm text-tx-main"
			/>
			<span class="shrink-0 text-tx-muted text-xs">
				{{ interpolar(t('aplicaciones.cuantas'), conVentana.length) }}
			</span>
		</header>

		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>

		<h2 class="flex items-center gap-2 font-medium text-sm text-tx-main">
			<Icono nombre="applications-other" :tamano="16" />
			{{ t('aplicaciones.conVentana') }}
		</h2>
		<p class="text-tx-muted text-xs">{{ t('aplicaciones.agrupadas') }}</p>

		<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
			<li
				v-for="a in conVentana"
				:key="a.pid"
				class="flex flex-wrap items-center gap-x-3 gap-y-1 bg-ui-surface/40 px-3 py-2.5 sm:px-4"
			>
				<Icono :nombre="a.nombre" :tamano="20" />
				<!-- `basis-0` con `min-w-32`: el nombre se lleva el espacio que sobra
				     pero no empuja el tamaño y el botón fuera de la ventana. -->
				<span class="min-w-32 flex-1 basis-0 truncate text-sm text-tx-main">{{ a.nombre }}</span>
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
			<li v-if="conVentana.length === 0" class="bg-ui-surface/40 px-4 py-3 text-sm text-tx-muted">
				{{ t('aplicaciones.ningunaConVentana') }}
			</li>
		</ul>

		<p class="text-tx-muted text-xs">{{ t('aplicaciones.cerrarExplicado') }}</p>

		<!-- Lo de segundo plano queda escondido por omisión: son cien filas de
		     ayudantes y servicios, y quien abre esta pantalla busca lo que abrió. -->
		<button
			type="button"
			class="flex items-center gap-2 self-start rounded-corner border border-ui-border px-3 py-1.5 text-sm text-tx-main hover:bg-ui-surface"
			@click="mostrarSinVentana = !mostrarSinVentana"
		>
			<Icono :nombre="mostrarSinVentana ? 'go-up' : 'go-down'" :tamano="14" />
			{{
				interpolar(
					mostrarSinVentana ? t('aplicaciones.ocultarFondo') : t('aplicaciones.verFondo'),
					sinVentana.length
				)
			}}
		</button>

		<template v-if="mostrarSinVentana">
			<p class="text-tx-muted text-xs">{{ t('aplicaciones.fondoExplicado') }}</p>
			<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
				<li
					v-for="a in sinVentana"
					:key="a.pid"
					class="flex flex-wrap items-center gap-x-3 gap-y-1 bg-ui-surface/40 px-3 py-2 sm:px-4"
				>
					<span class="min-w-32 flex-1 basis-0 truncate text-sm text-tx-muted">{{ a.nombre }}</span>
					<span class="shrink-0 font-mono text-tx-muted text-xs">{{ tamano(a.memoria) }}</span>
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
		</template>
	</section>
</template>
