<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	ListGroup,
	ListRow,
	LoadingState,
	StatTile,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import CleanableProjects from '@/components/CleanableProjects.vue';
import { tamano } from '@/tools/formato';
import { errorTrasLimpiarGrupo, hayLimpiezaEnCurso } from '@/tools/limpieza';

/** Los identificadores son los del backend y las claves del catálogo. */
type Task =
	| 'cache-de-usuario'
	| 'papelera'
	| 'cache-de-paquetes'
	| 'paquetes-huerfanos'
	| 'diario-viejo'
	| 'swap-a-la-memoria'
	| 'cache-del-kernel';

interface Reclaimable {
	tarea: Task;
	bytes: number | null;
	necesita_autenticar: boolean;
}

const { t } = useI18n();
const tasks = ref<Reclaimable[]>([]);
const loading = ref(true);
const error = ref('');
const notice = ref('');
const busyTask = ref<Task | null>(null);
const cleaningGroup = ref(false);

/** El icono de cada tarea. Todas las áreas llevan uno. */
const ICONS: Record<Task, string> = {
	'cache-de-usuario': 'folder-temp',
	papelera: 'user-trash-full',
	'cache-de-paquetes': 'package-x-generic',
	'paquetes-huerfanos': 'package-broken',
	'diario-viejo': 'text-x-generic',
	'swap-a-la-memoria': 'drive-harddisk',
	'cache-del-kernel': 'applications-system',
};

async function load() {
	loading.value = true;
	try {
		tasks.value = await invoke<Reclaimable[]>('recuperable');
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		loading.value = false;
	}
}

/** El total recuperable en disco, que es el número que la gente busca. */
const diskTotal = computed(() => tasks.value.reduce((sum, r) => sum + (r.bytes ?? 0), 0));

/** Cualquier limpieza en curso bloquea a todas las demás: dos comandos sobre el
 *  mismo recurso a la vez no se llevan bien. */
const busy = computed(() => hayLimpiezaEnCurso(cleaningGroup.value, busyTask.value));

const onDisk = computed(() => tasks.value.filter((r) => r.bytes !== null));
const inMemory = computed(() => tasks.value.filter((r) => r.bytes === null));

/**
 * Ejecuta un grupo entero de tareas.
 *
 * El backend las ordena poniendo al final las que piden autenticar, así polkit
 * pregunta una sola vez seguida en lugar de intercalar diálogos. Y devuelve **qué
 * falló** en lugar de cortar en la primera: si la caché de paquetes no se puede
 * tocar, la papelera y el diario igual se limpian.
 */
async function cleanGroup(group: Task[]) {
	if (group.length === 0 || busy.value) return;
	cleaningGroup.value = true;
	error.value = '';
	notice.value = '';
	try {
		const failures = await invoke<string[]>('limpiar_todo', { tareas: group });
		// Se recarga **antes** de anotar los fallos: `load` limpia `error` cuando
		// le va bien, así que anotándolos primero se borraban solos y la pantalla
		// decía «Listo» aunque la mitad no se hubiera hecho.
		await load();
		error.value = errorTrasLimpiarGrupo(error.value, failures);
		if (!error.value) {
			notice.value = t('limpieza.hecho');
		}
	} catch (e) {
		error.value = String(e);
	} finally {
		cleaningGroup.value = false;
	}
}

async function clean(r: Reclaimable) {
	// Nada arranca mientras haya otra limpieza: dos comandos sobre el mismo
	// recurso a la vez no se llevan bien, y el botón deshabilitado no alcanza
	// —un doble clic entra antes de que Vue lo pinte.
	if (busy.value) return;
	busyTask.value = r.tarea;
	error.value = '';
	notice.value = '';
	try {
		await invoke('limpiar', { tarea: r.tarea });
		await load();
		if (!error.value) {
			notice.value = t('limpieza.hecho');
		}
	} catch (e) {
		error.value = String(e);
	} finally {
		busyTask.value = null;
	}
}
onMounted(load);
</script>

<template>
	<section class="flex flex-col gap-4">
		<AlertMessage v-if="error" tone="error" icon="dialog-error">
			{{ error }}
		</AlertMessage>
		<!-- El «listo» era un párrafo verde sin rol: no se anunciaba, y el verde
		     solo no alcanza para quien no lo distingue. -->
		<AlertMessage v-if="notice" tone="success" icon="auto">{{ notice }}</AlertMessage>
		<LoadingState v-if="loading" :label="t('limpieza.midiendo')" />

		<template v-else>
			<StatTile
				:label="t('limpieza.totalEtiqueta')"
				:value="tamano(diskTotal)"
				icon="drive-harddisk"
				icon-type="icon"
			/>

			<div class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center gap-2">
					<h2 class="flex items-center gap-2 font-medium text-tx-main">
						<ThemeIcon name="drive-harddisk" :size="18" />
						{{ t('limpieza.enDisco') }}
					</h2>
					<!-- La acción principal de la pantalla: la única con el acento, que
					     antes iba escrito a mano con el primario al 10 %. -->
					<ActionButton
						:label="t('limpieza.limpiarTodo')"
						icon="edit-clear-all"
						variant="primary"
						class="ml-auto"
						:disabled="busy || onDisk.length === 0"
						@click="cleanGroup(onDisk.map((r) => r.tarea))" />
				</div>
				<ListGroup>
					<ListRow v-for="r in onDisk" :key="r.tarea" :icon="ICONS[r.tarea]">
						<!-- Todo en la ranura principal para que la fila se pueda partir:
						     en angosto el tamaño y el botón bajan a la línea siguiente. -->
						<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
							<div class="min-w-[min(10rem,100%)] flex-1 basis-0">
								<p class="text-sm text-tx-main">{{ t(`limpieza.tareas.${r.tarea}.titulo`) }}</p>
								<p class="text-tx-muted text-xs">{{ t(`limpieza.tareas.${r.tarea}.detalle`) }}</p>
							</div>
							<span class="shrink-0 font-mono text-sm text-tx-main">{{ tamano(r.bytes ?? 0) }}</span>
							<ActionButton
								:label="r.necesita_autenticar ? t('limpieza.limpiarConClave') : t('limpieza.limpiar')"
								variant="secondary"
								size="sm"
								class="shrink-0"
								:disabled="busy"
								@click="clean(r)" />
						</div>
					</ListRow>
				</ListGroup>
			</div>

			<div class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center gap-2">
					<h2 class="flex items-center gap-2 font-medium text-tx-main">
						<ThemeIcon name="applications-system" :size="18" />
						{{ t('limpieza.enMemoria') }}
					</h2>
					<ActionButton
						:label="t('limpieza.limpiarTodo')"
						icon="edit-clear-all"
						variant="secondary"
						class="ml-auto"
						:disabled="busy || inMemory.length === 0"
						@click="cleanGroup(inMemory.map((r) => r.tarea))" />
				</div>
				<!-- La parte incómoda, dicha de frente: casi todo lo que un botón de
				     «liberar RAM» hace en Linux es inútil o contraproducente, y no
				     decirlo sería vender humo. -->
				<AlertMessage tone="info" icon="auto">{{ t('limpieza.advertenciaMemoria') }}</AlertMessage>
				<ListGroup>
					<ListRow v-for="r in inMemory" :key="r.tarea" :icon="ICONS[r.tarea]">
						<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
							<div class="min-w-[min(10rem,100%)] flex-1 basis-0">
								<p class="text-sm text-tx-main">{{ t(`limpieza.tareas.${r.tarea}.titulo`) }}</p>
								<p class="text-tx-muted text-xs">{{ t(`limpieza.tareas.${r.tarea}.detalle`) }}</p>
							</div>
							<ActionButton
								:label="t('limpieza.limpiarConClave')"
								variant="secondary"
								size="sm"
								class="shrink-0"
								:disabled="busy"
								@click="clean(r)" />
						</div>
					</ListRow>
				</ListGroup>
			</div>

			<!-- Las carpetas de proyectos van al final y en su propia tarjeta: el
			     escaneo es a pedido, así que no puede compartir el estado de carga
			     con el resto, que se mide al abrir la pantalla. -->
			<CleanableProjects />
		</template>
	</section>
</template>
