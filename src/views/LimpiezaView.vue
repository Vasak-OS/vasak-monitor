<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import Icono from '@/components/Icono.vue';
import { tamano } from '@/tools/formato';
import { errorTrasLimpiarGrupo, hayLimpiezaEnCurso } from '@/tools/limpieza';

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
const limpiandoTodo = ref(false);

/** El icono de cada tarea. Todas las áreas llevan uno. */
const ICONOS: Record<Tarea, string> = {
	'cache-de-usuario': 'folder-temp',
	papelera: 'user-trash-full',
	'cache-de-paquetes': 'package-x-generic',
	'paquetes-huerfanos': 'package-broken',
	'diario-viejo': 'text-x-generic',
	'swap-a-la-memoria': 'drive-harddisk',
	'cache-del-kernel': 'applications-system',
};

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

/** Cualquier limpieza en curso bloquea a todas las demás: dos comandos sobre el
 *  mismo recurso a la vez no se llevan bien. */
const ocupado = computed(() => hayLimpiezaEnCurso(limpiandoTodo.value, ocupada.value));

const deDisco = computed(() => lista.value.filter((r) => r.bytes !== null));
const deMemoria = computed(() => lista.value.filter((r) => r.bytes === null));

/**
 * Ejecuta un grupo entero de tareas.
 *
 * El backend las ordena poniendo al final las que piden autenticar, así polkit
 * pregunta una sola vez seguida en lugar de intercalar diálogos. Y devuelve **qué
 * falló** en lugar de cortar en la primera: si la caché de paquetes no se puede
 * tocar, la papelera y el diario igual se limpian.
 */
async function limpiarGrupo(tareas: Tarea[]) {
	if (tareas.length === 0 || ocupado.value) return;
	limpiandoTodo.value = true;
	error.value = '';
	aviso.value = '';
	try {
		const fallos = await invoke<string[]>('limpiar_todo', { tareas });
		// Se recarga **antes** de anotar los fallos: `cargar` limpia `error` cuando
		// le va bien, así que anotándolos primero se borraban solos y la pantalla
		// decía «Listo» aunque la mitad no se hubiera hecho.
		await cargar();
		error.value = errorTrasLimpiarGrupo(error.value, fallos);
		if (!error.value) {
			aviso.value = t('limpieza.hecho');
		}
	} catch (e) {
		error.value = String(e);
	} finally {
		limpiandoTodo.value = false;
	}
}

async function limpiar(r: Recuperable) {
	// Nada arranca mientras haya otra limpieza: dos comandos sobre el mismo
	// recurso a la vez no se llevan bien, y el botón deshabilitado no alcanza
	// —un doble clic entra antes de que Vue lo pinte.
	if (ocupado.value) return;
	ocupada.value = r.tarea;
	error.value = '';
	aviso.value = '';
	try {
		await invoke('limpiar', { tarea: r.tarea });
		await cargar();
		if (!error.value) {
			aviso.value = t('limpieza.hecho');
		}
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
				<p class="flex items-center gap-2 text-tx-muted text-xs">
					<Icono nombre="drive-harddisk" :tamano="14" />
					{{ t('limpieza.totalEtiqueta') }}
				</p>
				<p class="font-mono text-2xl text-tx-main">{{ tamano(totalEnDisco) }}</p>
			</article>

			<div class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center gap-2">
					<h2 class="flex items-center gap-2 font-medium text-tx-main">
						<Icono nombre="drive-harddisk" :tamano="18" />
						{{ t('limpieza.enDisco') }}
					</h2>
					<button
						type="button"
						:disabled="ocupado || deDisco.length === 0"
						class="ml-auto flex items-center gap-1.5 rounded-corner border border-primary/30 bg-primary/10 px-3 py-1.5 font-medium text-primary text-sm hover:bg-primary/15 disabled:opacity-50"
						@click="limpiarGrupo(deDisco.map((r) => r.tarea))"
					>
						<Icono nombre="edit-clear-all" :tamano="14" />
						{{ t('limpieza.limpiarTodo') }}
					</button>
				</div>
				<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
					<li
						v-for="r in deDisco"
						:key="r.tarea"
						class="flex flex-wrap items-center gap-x-3 gap-y-1 bg-ui-surface/40 px-3 py-3 sm:px-4"
					>
						<Icono :nombre="ICONOS[r.tarea]" :tamano="20" />
						<div class="min-w-40 flex-1 basis-0">
							<p class="text-sm text-tx-main">{{ t(`limpieza.tareas.${r.tarea}.titulo`) }}</p>
							<p class="text-tx-muted text-xs">{{ t(`limpieza.tareas.${r.tarea}.detalle`) }}</p>
						</div>
						<span class="shrink-0 font-mono text-sm text-tx-main">{{ tamano(r.bytes ?? 0) }}</span>
						<button
							type="button"
							:disabled="ocupado"
							class="shrink-0 rounded-corner border border-ui-border px-3 py-1 text-sm text-tx-main hover:bg-ui-surface disabled:opacity-50"
							@click="limpiar(r)"
						>
							{{ r.necesita_autenticar ? t('limpieza.limpiarConClave') : t('limpieza.limpiar') }}
						</button>
					</li>
				</ul>
			</div>

			<div class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center gap-2">
					<h2 class="flex items-center gap-2 font-medium text-tx-main">
						<Icono nombre="applications-system" :tamano="18" />
						{{ t('limpieza.enMemoria') }}
					</h2>
					<button
						type="button"
						:disabled="ocupado || deMemoria.length === 0"
						class="ml-auto flex items-center gap-1.5 rounded-corner border border-ui-border px-3 py-1.5 text-sm text-tx-main hover:bg-ui-surface disabled:opacity-50"
						@click="limpiarGrupo(deMemoria.map((r) => r.tarea))"
					>
						<Icono nombre="edit-clear-all" :tamano="14" />
						{{ t('limpieza.limpiarTodo') }}
					</button>
				</div>
				<!-- La parte incómoda, dicha de frente: casi todo lo que un botón de
				     «liberar RAM» hace en Linux es inútil o contraproducente, y no
				     decirlo sería vender humo. -->
				<p class="rounded-corner bg-ui-surface/60 px-3 py-2 text-tx-muted text-xs">
					{{ t('limpieza.advertenciaMemoria') }}
				</p>
				<ul class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
					<li
						v-for="r in deMemoria"
						:key="r.tarea"
						class="flex flex-wrap items-center gap-x-3 gap-y-1 bg-ui-surface/40 px-3 py-3 sm:px-4"
					>
						<Icono :nombre="ICONOS[r.tarea]" :tamano="20" />
						<div class="min-w-40 flex-1 basis-0">
							<p class="text-sm text-tx-main">{{ t(`limpieza.tareas.${r.tarea}.titulo`) }}</p>
							<p class="text-tx-muted text-xs">{{ t(`limpieza.tareas.${r.tarea}.detalle`) }}</p>
						</div>
						<button
							type="button"
							:disabled="ocupado"
							class="shrink-0 rounded-corner border border-ui-border px-3 py-1 text-sm text-tx-main hover:bg-ui-surface disabled:opacity-50"
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
