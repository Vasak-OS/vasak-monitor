<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Disclosure,
	ListGroup,
	ListRow,
	SearchField,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import { useSondeo } from '@/composables/useSondeo';
import { tamano } from '@/tools/formato';
import { interpolar } from '@/tools/interpolar';

/** Los nombres de los campos son los del backend: son el formato del cable. */
interface RunningApp {
	pid: number;
	nombre: string;
	memoria: number;
	cpu: number | null;
	con_ventana: boolean;
}

const props = defineProps<{ interval: number; active: boolean }>();
const { t } = useI18n();
const apps = ref<RunningApp[]>([]);
const error = ref('');
const filter = ref('');
const showBackground = ref(false);
const closing = ref<number | null>(null);

useSondeo(
	async () => {
		try {
			apps.value = await invoke<RunningApp[]>('aplicaciones');
			error.value = '';
		} catch (e) {
			error.value = String(e);
		}
	},
	() => props.interval,
	() => props.active
);

const matches = (a: RunningApp) => {
	const q = filter.value.trim().toLowerCase();
	return !q || a.nombre.toLowerCase().includes(q);
};

const withWindow = computed(() => apps.value.filter((a) => a.con_ventana && matches(a)));
const inBackground = computed(() => apps.value.filter((a) => !a.con_ventana && matches(a)));

async function close(a: RunningApp) {
	closing.value = a.pid;
	error.value = '';
	try {
		await invoke('cerrar', { pid: a.pid });
	} catch (e) {
		error.value = String(e);
	} finally {
		closing.value = null;
	}
}
</script>

<template>
	<section class="flex flex-col gap-3">
		<!-- `flex-wrap` y anchos mínimos: en una ventana angosta el buscador y el
		     contador se apilan en lugar de comprimirse hasta ser ilegibles. -->
		<header class="flex flex-wrap items-center gap-2">
			<!-- El campo del sistema: trae la lupa, la cruz para vaciarlo y —lo
			     que no tenía— un nombre. El `placeholder` no es un nombre: se
			     va en cuanto se escribe la primera letra, y un lector de
			     pantalla no tiene obligación de leerlo.

			     `basis-40` y no `min-w-40`: en el applet más angosto un mínimo
			     de 160 px se salía del área y obligaba a desplazar de costado. -->
			<SearchField
				v-model="filter"
				class="min-w-0 flex-1 basis-40"
				:placeholder="t('aplicaciones.buscar')"
				:label="t('aplicaciones.buscar')" />
			<span class="shrink-0 text-tx-muted text-xs">
				{{ interpolar(t('aplicaciones.cuantas'), withWindow.length) }}
			</span>
		</header>

		<AlertMessage v-if="error" tone="error" icon="dialog-error">
			{{ error }}
		</AlertMessage>

		<h2 class="flex items-center gap-2 font-medium text-sm text-tx-main">
			<ThemeIcon name="applications-other" :size="16" />
			{{ t('aplicaciones.conVentana') }}
		</h2>
		<p class="text-tx-muted text-xs">{{ t('aplicaciones.agrupadas') }}</p>

		<!-- La lista y sus filas son las de la librería: el canto, la superficie
		     y los divisores eran una copia de `ListGroup` escrita acá.
		
		     Todo va en la ranura principal y no en `trailing` porque la fila tiene
		     que poder partirse: con su mínimo el nombre se queda con su línea y,
		     cuando no entra, el tamaño y el botón bajan a la siguiente en vez de
		     salirse del área. `trailing` no se parte. -->
		<ListGroup>
			<ListRow v-for="a in withWindow" :key="a.pid" :icon="a.nombre">
				<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
					<!-- `basis-0` con 8rem de mínimo: el nombre se lleva el espacio que sobra
					     pero no empuja el tamaño y el botón fuera de la ventana; y nunca más
					     que la fila, que en el applet más angosto no llega a 8rem. -->
					<span class="min-w-[min(8rem,100%)] flex-1 basis-0 truncate text-sm text-tx-main" :title="a.nombre">{{
						a.nombre
					}}</span>
					<span class="shrink-0 font-mono text-sm text-tx-muted">{{ tamano(a.memoria) }}</span>
					<ActionButton
						:label="t('aplicaciones.cerrar')"
						variant="secondary"
						size="sm"
						class="shrink-0"
						:disabled="closing === a.pid"
						@click="close(a)" />
				</div>
			</ListRow>
			<ListRow v-if="withWindow.length === 0">
				<span class="text-sm text-tx-muted">{{ t('aplicaciones.ningunaConVentana') }}</span>
			</ListRow>
		</ListGroup>

		<p class="text-tx-muted text-xs">{{ t('aplicaciones.cerrarExplicado') }}</p>

		<!-- Lo de segundo plano queda escondido por omisión: son cien filas de
		     ayudantes y servicios, y quien abre esta pantalla busca lo que abrió.
		     `Disclosure` dice además que se abre (`aria-expanded`), que el botón
		     escrito a mano no decía. -->
		<Disclosure
			v-model:open="showBackground"
			:title="
				interpolar(
					showBackground ? t('aplicaciones.ocultarFondo') : t('aplicaciones.verFondo'),
					inBackground.length
				)
			"
		>
			<div class="flex flex-col gap-3">
				<p class="text-tx-muted text-xs">{{ t('aplicaciones.fondoExplicado') }}</p>
				<ListGroup>
					<ListRow v-for="a in inBackground" :key="a.pid">
						<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
							<span class="min-w-[min(8rem,100%)] flex-1 basis-0 truncate text-sm text-tx-muted" :title="a.nombre">{{
								a.nombre
							}}</span>
							<span class="shrink-0 font-mono text-tx-muted text-xs">{{ tamano(a.memoria) }}</span>
							<ActionButton
								:label="t('aplicaciones.cerrar')"
								variant="secondary"
								size="sm"
								class="shrink-0"
								:disabled="closing === a.pid"
								@click="close(a)" />
						</div>
					</ListRow>
				</ListGroup>
			</div>
		</Disclosure>
	</section>
</template>
