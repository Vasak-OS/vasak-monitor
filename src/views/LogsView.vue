<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Checkbox,
	EmptyState,
	ListGroup,
	ListRow,
	LoadingState,
	SearchField,
	SelectField,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { interpolar } from '@/tools/interpolar';
import {
	type AppDelDiario,
	ECOSISTEMA,
	etiquetaDeApp,
	iconoDeSeleccion,
	pideExplicacionDelVacio,
	SISTEMA,
} from '@/tools/registros';

/** Los nombres de los campos son los del backend: son el formato del cable. */
interface LogEntry {
	microsegundos: number;
	origen: string;
	nivel: number;
	mensaje: string;
}

const { t } = useI18n();
const entries = ref<LogEntry[]>([]);
const journalApps = ref<AppDelDiario[]>([]);
const utcOffset = ref(0);
const loading = ref(true);
const error = ref('');
const onlyProblems = ref(false);
const filter = ref('');
const app = ref<string>(ECOSISTEMA);

/**
 * El número del último pedido.
 *
 * El selector, la casilla y el botón llaman a `load()` cada uno, y dos pedidos
 * pueden cruzarse: si el de antes contesta último, pisaría la lista con
 * entradas de un filtro que ya no está elegido. Sólo el último escribe.
 */
let latestRequest = 0;

async function load() {
	const request = ++latestRequest;
	loading.value = true;
	try {
		const offset = await invoke<number>('desplazamiento_horario');
		const result = await invoke<LogEntry[]>('registros_de_vasakos', {
			soloProblemas: onlyProblems.value,
			cantidad: 500,
			app: app.value,
		});
		if (request !== latestRequest) return;
		utcOffset.value = offset;
		entries.value = result;
		error.value = '';
	} catch (e) {
		if (request !== latestRequest) return;
		error.value = String(e);
	} finally {
		if (request === latestRequest) loading.value = false;
	}
}

/** El catálogo se pide una sola vez: enumerar los campos del diario recorre su
 *  índice, y no cambia entre dos actualizaciones de la lista. */
async function loadApps() {
	try {
		journalApps.value = await invoke<AppDelDiario[]>('apps_del_diario');
	} catch {
		// Sin catálogo el selector queda con las dos opciones amplias, que es
		// mejor que no poder ver nada.
		journalApps.value = [];
	}
}

/** El icono de lo que está seleccionado. Va al lado del selector porque `option`
 *  no admite contenido: es la única forma de que esta área tenga icono. */
const selectedIcon = computed(() => iconoDeSeleccion(app.value, journalApps.value));

const visible = computed(() => {
	const q = filter.value.trim().toLowerCase();
	if (!q) return entries.value;
	return entries.value.filter(
		(e) => e.mensaje.toLowerCase().includes(q) || e.origen.toLowerCase().includes(q)
	);
});

/** Cuando se eligió una app y no hay nada, la explicación importa: puede que la
 *  app no haya fallado, o que escriba en el diario de la sesión. */
const emptyForOneApp = computed(() =>
	pideExplicacionDelVacio(app.value, entries.value.length, loading.value)
);

/** La hora en local. El diario informa en UTC y el reloj del panel muestra local:
 *  sin convertir, las horas no coinciden con nada de lo que la persona vio. */
function clock(microseconds: number): string {
	const seconds = Math.floor(microseconds / 1_000_000) + utcOffset.value;
	const rest = ((seconds % 86_400) + 86_400) % 86_400;
	const two = (n: number) => String(n).padStart(2, '0');
	return `${two(Math.floor(rest / 3600))}:${two(Math.floor((rest % 3600) / 60))}:${two(rest % 60)}`;
}

/**
 * El nivel se marca con el canto izquierdo de la fila, como las líneas de
 * `CodeBlock`, y el texto queda en `tx-main`: el ámbar y el rojo del esquema
 * sobre la superficie no llegan a 4,5:1 como color de letra, y como canto sólo
 * necesitan 3:1. Hasta la 0.8 el tono no se veía nunca —el `body *` de
 * `main.css` pisaba toda utilidad de color de texto—, así que esto es lo
 * primero que lo muestra.
 */
const levelTone = (level: number) =>
	level <= 3 ? 'border-status-error' : level === 4 ? 'border-status-warning' : 'border-transparent';

onMounted(() => {
	void loadApps();
	void load();
});
</script>

<template>
	<!-- Los cortes de esta pantalla son de **contenedor** (`@lg`, 32rem del área
	     de la pantalla) y no de la ventana: eran `sm:`, 640 píxeles de pantalla,
	     que no descontaban la barra lateral. Medido en el banco, el área mide 478
	     a 600 de ventana y 866 a 1200, así que `@lg` da lo mismo que daba `sm:` en
	     los anchos que se miran: dos líneas por entrada hasta 600, una a 1200. -->
	<section class="flex min-h-0 flex-col gap-3">
		<!-- El selector primero: es lo que decide qué se está mirando, y el resto
		     de los controles filtran dentro de eso. -->
		<header class="flex flex-wrap items-center gap-2 @lg:gap-3">
			<label class="flex min-w-0 items-center gap-2 text-sm text-tx-main">
				<ThemeIcon :name="selectedIcon" :size="18" :alt="t('registros.deQuien')" />
				<span class="sr-only">{{ t('registros.deQuien') }}</span>
				<SelectField v-model="app" class="min-w-0 max-w-56" @change="load()">
					<option :value="ECOSISTEMA">{{ t('registros.todoElEcosistema') }}</option>
					<option v-for="a in journalApps" :key="a.id" :value="a.id">
						{{ etiquetaDeApp(a, t('registros.sinEntradas')) }}
					</option>
					<option :value="SISTEMA">{{ t('registros.todoElSistema') }}</option>
				</SelectField>
			</label>

			<Checkbox v-model="onlyProblems" :label="t('registros.soloProblemas')" @change="load()" />

			<!-- `basis-40` y no `min-w-40`: en el applet más angosto un mínimo de
			     160 px se salía del área y obligaba a desplazar de costado. -->
			<SearchField
				v-model="filter"
				class="min-w-0 flex-1 basis-40"
				:placeholder="t('registros.buscar')"
				:label="t('registros.buscar')" />

			<ActionButton
				:label="t('common.actualizar')"
				icon="view-refresh"
				variant="secondary"
				@click="load()" />
		</header>

		<AlertMessage v-if="error" tone="error" icon="dialog-error">
			{{ error }}
		</AlertMessage>
		<LoadingState v-if="loading" :label="t('common.cargando')" />

		<template v-else>
			<EmptyState
				v-if="emptyForOneApp"
				:title="t('registros.nadaQueMostrar')"
				:note="t('registros.dondeEscriben')"
				size="sm"
				bordered
			/>

			<template v-else>
				<p class="flex items-center gap-2 text-tx-muted text-xs">
					<ThemeIcon name="text-x-generic" :size="14" alt="" />
					{{ interpolar(t('registros.cuantas'), visible.length) }}
				</p>
				<!-- Dos líneas en angosto y una en ancho.
				     Con `w-44` fijo para el origen, en una ventana de 700 px el mensaje
				     quedaba en dos palabras por línea y había que leerlo en vertical.
				     Ahora la hora y el origen van juntos arriba y el mensaje abajo, y a
				     partir de `@lg` vuelven a la misma línea. -->
				<ListGroup class="min-h-0 flex-1 overflow-y-auto">
					<ListRow v-for="(e, i) in visible" :key="`${e.microsegundos}-${i}`">
						<div
							:class="levelTone(e.nivel)"
							:data-level="e.nivel"
							class="-ml-3 flex min-w-0 flex-col gap-0.5 border-l-2 pl-2.5 @lg:flex-row @lg:gap-3"
						>
							<div class="flex shrink-0 items-baseline gap-2 @lg:gap-3">
								<span class="font-mono text-tx-muted text-xs">{{ clock(e.microsegundos) }}</span>
								<span class="max-w-44 truncate text-tx-muted text-xs @lg:w-44" :title="e.origen">{{
									e.origen
								}}</span>
							</div>
							<span class="min-w-0 flex-1 break-words text-tx-main text-xs">{{
								e.mensaje
							}}</span>
						</div>
					</ListRow>
				</ListGroup>
			</template>
		</template>
	</section>
</template>
