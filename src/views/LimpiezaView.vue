<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import { tamano } from '@/tools/formato';

type Tarea =
	| 'cache-de-usuario'
	| 'papelera'
	| 'cache-de-paquetes'
	| 'paquetes-huerfanos'
	| 'diario-viejo'
	| 'swap-a-la-memoria'
	| 'cache-del-kernel';

interface Recuperable {
	tarea: Tarea;
	bytes: number | null;
	necesita_autenticar: boolean;
}

const { t } = useI18n();
const lista = ref<Recuperable[]>([]);
const cargando = ref(true);
const error = ref('');
const aviso = ref('');
const ocupada = ref<Tarea | null>(null);

async function cargar() {
	cargando.value = true;
	try {
		lista.value = await invoke<Recuperable[]>('recuperable');
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		cargando.value = false;
	}
}

/** El total recuperable en disco, que es el número que la gente busca. */
const totalEnDisco = computed(() => lista.value.reduce((suma, r) => suma + (r.bytes ?? 0), 0));

const deDisco = computed(() => lista.value.filter((r) => r.bytes !== null));
const deMemoria = computed(() => lista.value.filter((r) => r.bytes === null));

async function limpiar(r: Recuperable) {
	ocupada.value = r.tarea;
	error.value = '';
	aviso.value = '';
	try {
		await invoke('limpiar', { tarea: r.tarea });
		aviso.value = t('limpieza.hecho');
		await cargar();
	} catch (e) {
		error.value = String(e);
	} finally {
		ocupada.value = null;
	}
}
onMounted(cargar);
</script>

<template>
	<section class="flex flex-col gap-4">
		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>
		<p v-if="aviso" class="rounded-corner bg-status-success/10 px-3 py-2 text-sm text-status-success">
			{{ aviso }}
		</p>
		<p v-if="cargando" class="text-sm text-tx-muted">{{ t('limpieza.midiendo') }}</p>

		<template v-else>
			<article class="rounded-corner border border-ui-border bg-ui-surface/40 p-4">
				<p class="text-tx-muted text-xs">{{ t('limpieza.totalEtiqueta') }}</p>
				<p class="font-mono text-2xl text-tx-primary">{{ tamano(totalEnDisco) }}</p>
			</article>

			<div class="flex flex-col gap-2">
				<h2 class="font-medium text-tx-primary">{{ t('limpieza.enDisco') }}</h2>
				<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
					<li v-for="r in deDisco" :key="r.tarea" class="flex items-center gap-3 bg-ui-surface/40 px-4 py-3">
						<div class="min-w-0 flex-1">
							<p class="text-sm text-tx-primary">{{ t(`limpieza.tareas.${r.tarea}.titulo`) }}</p>
							<p class="text-tx-muted text-xs">{{ t(`limpieza.tareas.${r.tarea}.detalle`) }}</p>
						</div>
						<span class="shrink-0 font-mono text-sm text-tx-primary">{{ tamano(r.bytes ?? 0) }}</span>
						<button
							type="button"
							:disabled="ocupada === r.tarea"
							class="shrink-0 rounded-corner border border-ui-border px-3 py-1 text-sm text-tx-primary hover:bg-ui-surface disabled:opacity-50"
							@click="limpiar(r)"
						>
							{{ r.necesita_autenticar ? t('limpieza.limpiarConClave') : t('limpieza.limpiar') }}
						</button>
					</li>
				</ul>
			</div>

			<div class="flex flex-col gap-2">
				<h2 class="font-medium text-tx-primary">{{ t('limpieza.enMemoria') }}</h2>
				<!-- La parte incómoda, dicha de frente: casi todo lo que un botón de
				     «liberar RAM» hace en Linux es inútil o contraproducente, y no
				     decirlo sería vender humo. -->
				<p class="rounded-corner bg-ui-surface/60 px-3 py-2 text-tx-muted text-xs">
					{{ t('limpieza.advertenciaMemoria') }}
				</p>
				<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
					<li v-for="r in deMemoria" :key="r.tarea" class="flex items-center gap-3 bg-ui-surface/40 px-4 py-3">
						<div class="min-w-0 flex-1">
							<p class="text-sm text-tx-primary">{{ t(`limpieza.tareas.${r.tarea}.titulo`) }}</p>
							<p class="text-tx-muted text-xs">{{ t(`limpieza.tareas.${r.tarea}.detalle`) }}</p>
						</div>
						<button
							type="button"
							:disabled="ocupada === r.tarea"
							class="shrink-0 rounded-corner border border-ui-border px-3 py-1 text-sm text-tx-primary hover:bg-ui-surface disabled:opacity-50"
							@click="limpiar(r)"
						>
							{{ t('limpieza.limpiarConClave') }}
						</button>
					</li>
				</ul>
			</div>
		</template>
	</section>
</template>
