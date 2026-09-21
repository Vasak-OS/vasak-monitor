<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	AlertMessage,
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

interface Entrada {
	microsegundos: number;
	origen: string;
	nivel: number;
	mensaje: string;
}

const { t } = useI18n();
const lista = ref<Entrada[]>([]);
const apps = ref<AppDelDiario[]>([]);
const desplazamiento = ref(0);
const cargando = ref(true);
const error = ref('');
const soloProblemas = ref(false);
const filtro = ref('');
const app = ref<string>(ECOSISTEMA);

async function cargar() {
	cargando.value = true;
	try {
		desplazamiento.value = await invoke<number>('desplazamiento_horario');
		lista.value = await invoke<Entrada[]>('registros_de_vasakos', {
			soloProblemas: soloProblemas.value,
			cantidad: 500,
			app: app.value,
		});
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		cargando.value = false;
	}
}

/** El catálogo se pide una sola vez: enumerar los campos del diario recorre su
 *  índice, y no cambia entre dos actualizaciones de la lista. */
async function cargarApps() {
	try {
		apps.value = await invoke<AppDelDiario[]>('apps_del_diario');
	} catch {
		// Sin catálogo el selector queda con las dos opciones amplias, que es
		// mejor que no poder ver nada.
		apps.value = [];
	}
}

/** El icono de lo que está seleccionado. Va al lado del selector porque `option`
 *  no admite contenido: es la única forma de que esta área tenga icono. */
const iconoElegido = computed(() => iconoDeSeleccion(app.value, apps.value));

const visibles = computed(() => {
	const q = filtro.value.trim().toLowerCase();
	if (!q) return lista.value;
	return lista.value.filter(
		(e) => e.mensaje.toLowerCase().includes(q) || e.origen.toLowerCase().includes(q)
	);
});

/** Cuando se eligió una app y no hay nada, la explicación importa: puede que la
 *  app no haya fallado, o que escriba en el diario de la sesión. */
const vacioDeUnaApp = computed(() =>
	pideExplicacionDelVacio(app.value, lista.value.length, cargando.value)
);

/** La hora en local. El diario informa en UTC y el reloj del panel muestra local:
 *  sin convertir, las horas no coinciden con nada de lo que la persona vio. */
function hora(microsegundos: number): string {
	const segundos = Math.floor(microsegundos / 1_000_000) + desplazamiento.value;
	const resto = ((segundos % 86_400) + 86_400) % 86_400;
	const dos = (n: number) => String(n).padStart(2, '0');
	return `${dos(Math.floor(resto / 3600))}:${dos(Math.floor((resto % 3600) / 60))}:${dos(resto % 60)}`;
}

const tonoDeNivel = (nivel: number) =>
	nivel <= 3 ? 'text-status-error' : nivel === 4 ? 'text-status-warning' : 'text-tx-muted';

onMounted(() => {
	void cargarApps();
	void cargar();
});
</script>

<template>
	<section class="flex min-h-0 flex-col gap-3">
		<!-- El selector primero: es lo que decide qué se está mirando, y el resto
		     de los controles filtran dentro de eso. -->
		<header class="flex flex-wrap items-center gap-2 sm:gap-3">
			<label class="flex min-w-0 items-center gap-2 text-sm text-tx-main">
				<ThemeIcon :name="iconoElegido" :size="18" :alt="t('registros.deQuien')" />
				<span class="sr-only">{{ t('registros.deQuien') }}</span>
				<SelectField v-model="app" class="max-w-56" @change="cargar()">
					<option :value="ECOSISTEMA">{{ t('registros.todoElEcosistema') }}</option>
					<option v-for="a in apps" :key="a.id" :value="a.id">
						{{ etiquetaDeApp(a, t('registros.sinEntradas')) }}
					</option>
					<option :value="SISTEMA">{{ t('registros.todoElSistema') }}</option>
				</SelectField>
			</label>

			<label class="flex items-center gap-2 text-sm text-tx-main">
				<input v-model="soloProblemas" type="checkbox" @change="cargar()" />
				{{ t('registros.soloProblemas') }}
			</label>

			<SearchField
				v-model="filtro"
				class="min-w-40 flex-1"
				:placeholder="t('registros.buscar')"
				:label="t('registros.buscar')" />

			<button
				type="button"
				class="flex items-center gap-2 rounded-corner border border-ui-border px-3 py-1.5 text-sm text-tx-main hover:bg-ui-surface"
				@click="cargar()"
			>
				<ThemeIcon name="view-refresh" :size="16" alt="" />
				{{ t('common.actualizar') }}
			</button>
		</header>

		<AlertMessage v-if="error" tone="error" icon="dialog-error">
			{{ error }}
		</AlertMessage>
		<LoadingState v-if="cargando" :label="t('common.cargando')" />

		<template v-else>
			<div
				v-if="vacioDeUnaApp"
				class="flex flex-col gap-2 rounded-corner border border-ui-border bg-ui-surface/70 px-4 py-4"
			>
				<p class="flex items-center gap-2 text-sm text-tx-main">
					<ThemeIcon name="dialog-information" :size="18" alt="" />
					{{ t('registros.nadaQueMostrar') }}
				</p>
				<p class="text-tx-muted text-xs">{{ t('registros.dondeEscriben') }}</p>
			</div>

			<template v-else>
				<p class="flex items-center gap-2 text-tx-muted text-xs">
					<ThemeIcon name="text-x-generic" :size="14" alt="" />
					{{ interpolar(t('registros.cuantas'), visibles.length) }}
				</p>
				<ul
					class="flex min-h-0 flex-1 flex-col divide-y divide-ui-border overflow-y-auto rounded-corner border border-ui-border"
				>
					<!-- Dos líneas en angosto y una en ancho.
					     Con `w-44` fijo para el origen, en una ventana de 700 px el
					     mensaje quedaba en dos palabras por línea y había que leerlo en
					     vertical. Ahora la hora y el origen van juntos arriba y el
					     mensaje abajo, y a partir de `sm` vuelven a la misma línea. -->
					<li
						v-for="(e, i) in visibles"
						:key="`${e.microsegundos}-${i}`"
						class="flex flex-col gap-0.5 bg-ui-surface/70 px-3 py-2 sm:flex-row sm:gap-3 sm:px-4"
					>
						<div class="flex shrink-0 items-baseline gap-2 sm:gap-3">
							<span class="font-mono text-tx-muted text-xs">{{ hora(e.microsegundos) }}</span>
							<span class="max-w-44 truncate text-tx-muted text-xs sm:w-44">{{ e.origen }}</span>
						</div>
						<span :class="tonoDeNivel(e.nivel)" class="min-w-0 flex-1 break-words text-xs">{{
							e.mensaje
						}}</span>
					</li>
				</ul>
			</template>
		</template>
	</section>
</template>
