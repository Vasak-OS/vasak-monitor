<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	ListGroup,
	ListRow,
	PageHeader,
	SelectField,
	SideBar,
	SideButton,
} from '@vasakgroup/vue-libvasak';
import { onMounted, onUnmounted, ref } from 'vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { esEnVivo, INTERVALO_POR_OMISION, INTERVALOS, intervaloValido } from '@/tools/sondeo';
import AppsView from '@/views/AppsView.vue';
import CleanupView from '@/views/CleanupView.vue';
import LogsView from '@/views/LogsView.vue';
import ResourcesView from '@/views/ResourcesView.vue';
import ServicesView from '@/views/ServicesView.vue';

const { t } = useI18n();
const configStore = useConfigStore() as any;

/** Los identificadores son también las claves del catálogo (`pantallas.*`). */
const SCREENS = ['recursos', 'aplicaciones', 'servicios', 'limpieza', 'registros'] as const;
type Screen = (typeof SCREENS)[number];

const screen = ref<Screen>('recursos');

/**
 * Qué se ve en una ventana angosta: la lista de pantallas o la pantalla.
 *
 * En una ventana angosta va una columna por vez, como en una aplicación de
 * teléfono: la barra lateral plegada dejaba 120 píxeles para el contenido a
 * 240 de ancho, y el buscador de Registros se salía del área. Por debajo de
 * `30rem` de la fila de la ventana se ve **una** de las dos —la lista o la
 * pantalla elegida, con un botón para volver—; lo decide una consulta de
 * contenedor, así que en el ancho habitual esto no cambia nada y las dos se
 * ven a la vez como siempre.
 *
 * Arranca en la lista, que es lo que abre una aplicación de teléfono: es la
 * única vista angosta donde se ve qué hay.
 */
const narrowPane = ref<'list' | 'screen'>('list');

/** El icono de cada pantalla. Todas las áreas del escritorio llevan uno. */
const ICONS: Record<Screen, string> = {
	recursos: 'utilities-system-monitor',
	aplicaciones: 'applications-other',
	servicios: 'system-run',
	limpieza: 'user-trash',
	registros: 'text-x-generic',
};
const interval = ref(INTERVALO_POR_OMISION);

let stopConfigListener: UnlistenFn | null = null;

function open(next: Screen) {
	screen.value = next;
	narrowPane.value = 'screen';
}

onMounted(async () => {
	try {
		await configStore.loadConfig();
		// El intervalo se valida al leerlo: un valor escrito a mano en la
		// configuración no debe dejar el monitor midiendo cada milisegundo.
		interval.value = intervaloValido(
			configStore.config?.monitor?.intervalo ?? INTERVALO_POR_OMISION
		);
		stopConfigListener = await listen('config-changed', () => void configStore.loadConfig());
	} catch {
		// Sin configuración se usa el intervalo por omisión: no arrancar por no
		// poder leer una preferencia sería peor que ignorarla.
	}
});

onUnmounted(() => stopConfigListener?.());
</script>

<template>
	<WindowAppLayout>
		<!-- `gap-1 p-1` como en Configuración y en la tienda: la barra es una
		     tarjeta con borde y esquina redondeada, y pegada al borde de la ventana
		     se le come el redondeo.

		     `@container/window`: lo que decide cuántas columnas entran es el ancho
		     de esta fila, no el de la pantalla. En WebKitGTK ni `matchMedia` ni
		     `resize` avisan, y la ventana puede ser un applet. -->
		<div class="@container/window flex h-full min-h-0 w-full gap-1 p-1" data-window-row>
			<!-- La barra es la de `@vasakgroup/vue-libvasak`, que es la de
			     Configuración. Sin área de título: el nombre de la ventana ya está en
			     la barra superior y repetirlo gastaría la mitad del alto.

			     Con botones sueltos y no con `categories`: las cinco pantallas no
			     están agrupadas, y un grupo con título sería un encabezado que hoy
			     no existe.

			     Angosta no se muestra: ahí la lista de abajo ocupa su lugar. -->
			<div class="hidden h-full min-h-0 @[30rem]/window:flex" data-sidebar>
				<SideBar
					:collapse-label="t('barraLateral.plegar')"
					:expand-label="t('barraLateral.desplegar')"
				>
					<template #default="{ collapsed }">
						<SideButton
							v-for="p in SCREENS"
							:key="p"
							:label="t(`pantallas.${p}`)"
							:icon="ICONS[p]"
							:active="screen === p"
							:collapsed="collapsed"
							@click="open(p)"
						/>

						<!-- El intervalo, al pie. Plegada no entra ni la etiqueta ni el
						     desplegable, así que se esconde. -->
						<div v-if="!collapsed" class="mt-auto flex flex-col gap-1 pt-3">
							<!-- La etiqueta la pone el propio selector y queda atada al control:
							     suelta acá al lado no estaba asociada a nada. Sin `.number`:
							     las opciones atan el número con `:value="i"`. -->
							<SelectField v-model="interval" :label="t('ajustes.intervalo')">
								<option v-for="i in INTERVALOS" :key="i" :value="i">{{ i / 1000 }} s</option>
							</SelectField>
							<!-- Se dice que la medición se pausa: sin eso, alguien que abre el
							     monitor y lo deja de fondo supone que sigue gastando. -->
							<p class="text-tx-muted text-xs">{{ t('ajustes.pausaExplicada') }}</p>
						</div>
					</template>
				</SideBar>
			</div>

			<!-- La lista de pantallas de la ventana angosta: lo mismo que la barra,
			     con el intervalo incluido, que no puede desaparecer por achicar la
			     ventana. -->
			<nav
				v-if="narrowPane === 'list'"
				class="flex min-h-0 min-w-0 flex-1 flex-col gap-3 overflow-y-auto p-3 @[30rem]/window:hidden"
				:aria-label="t('navigation.screens')"
				data-narrow-list
			>
				<ListGroup>
					<ListRow
						v-for="p in SCREENS"
						:key="p"
						role="button"
						:icon="ICONS[p]"
						:title="t(`pantallas.${p}`)"
						:selected="screen === p"
						@click="open(p)"
					/>
				</ListGroup>
				<div class="flex flex-col gap-1">
					<SelectField v-model="interval" :label="t('ajustes.intervalo')">
						<option v-for="i in INTERVALOS" :key="i" :value="i">{{ i / 1000 }} s</option>
					</SelectField>
					<p class="text-tx-muted text-xs">{{ t('ajustes.pausaExplicada') }}</p>
				</div>
			</nav>

			<!-- `@container`: lo que decide si algo cabe es el ancho de **esta** área, no
			     el de la ventana. Con cortes por viewport, la barra lateral de 208 px
			     no se descuenta y una ventana de 800 px pone dos columnas en 570 px de
			     espacio real.

			     El relleno sube a 16 px con la fila de la ventana en `@2xl` (42rem),
			     que es lo que hacía `sm:p-4` en los anchos que se miran: 12 a 600 de
			     ventana, 16 a 1200. -->
			<main
				class="@container min-h-0 min-w-0 flex-1 overflow-y-auto p-3 @2xl/window:p-4"
				:class="narrowPane === 'list' ? 'hidden @[30rem]/window:block' : ''"
				data-screen
			>
				<ActionButton
					:label="t('navigation.back')"
					icon="go-previous"
					variant="ghost"
					size="sm"
					class="mb-2 @[30rem]/window:hidden"
					@click="narrowPane = 'list'"
				/>
				<PageHeader class="mb-4" :title="t(`pantallas.${screen}`)" :icon="ICONS[screen]" />
				<ResourcesView
					v-if="screen === 'recursos'"
					:interval="interval"
					:active="esEnVivo(screen)"
				/>
				<AppsView
					v-else-if="screen === 'aplicaciones'"
					:interval="interval"
					:active="esEnVivo(screen)"
				/>
				<ServicesView v-else-if="screen === 'servicios'" />
				<CleanupView v-else-if="screen === 'limpieza'" />
				<LogsView v-else-if="screen === 'registros'" />
			</main>
		</div>
	</WindowAppLayout>
</template>
