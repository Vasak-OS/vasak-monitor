<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { SelectField, SideBar, SideButton, ThemeIcon } from '@vasakgroup/vue-libvasak';
import { onMounted, onUnmounted, ref } from 'vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';
import { esEnVivo, INTERVALO_POR_OMISION, INTERVALOS, intervaloValido } from '@/tools/sondeo';
import AplicacionesView from '@/views/AplicacionesView.vue';
import LimpiezaView from '@/views/LimpiezaView.vue';
import RecursosView from '@/views/RecursosView.vue';
import RegistrosView from '@/views/RegistrosView.vue';
import ServiciosView from '@/views/ServiciosView.vue';

const { t } = useI18n();
const configStore = useConfigStore() as any;

const PANTALLAS = ['recursos', 'aplicaciones', 'servicios', 'limpieza', 'registros'] as const;
type Pantalla = (typeof PANTALLAS)[number];

const pantalla = ref<Pantalla>('recursos');

/** El icono de cada pantalla. Todas las áreas del escritorio llevan uno. */
const ICONOS: Record<Pantalla, string> = {
	recursos: 'utilities-system-monitor',
	aplicaciones: 'applications-other',
	servicios: 'system-run',
	limpieza: 'user-trash',
	registros: 'text-x-generic',
};
const intervalo = ref(INTERVALO_POR_OMISION);

let soltarConfig: UnlistenFn | null = null;

onMounted(async () => {
	try {
		await configStore.loadConfig();
		// El intervalo se valida al leerlo: un valor escrito a mano en la
		// configuración no debe dejar el monitor midiendo cada milisegundo.
		intervalo.value = intervaloValido(
			configStore.config?.monitor?.intervalo ?? INTERVALO_POR_OMISION
		);
		soltarConfig = await listen('config-changed', () => void configStore.loadConfig());
	} catch {
		// Sin configuración se usa el intervalo por omisión: no arrancar por no
		// poder leer una preferencia sería peor que ignorarla.
	}
});

onUnmounted(() => soltarConfig?.());
</script>

<template>
	<WindowAppLayout>
		<!-- `gap-1 p-1` como en Configuración y en la tienda: la barra es una
		     tarjeta con borde y esquina redondeada, y pegada al borde de la ventana
		     se le come el redondeo. Antes no hacía falta porque era un `<nav>` con
		     un borde derecho, que sí quería llegar hasta el filo. -->
		<div class="flex h-full min-h-0 w-full gap-1 p-1">
			<!-- La barra es la de `@vasakgroup/vue-libvasak`, que es la de
			     Configuración: acá había una escrita a mano que se parecía pero no
			     era, y que no se podía plegar —sólo se angostaba sola por debajo de
			     `sm`—. Sin área de título: el nombre de la ventana ya está en la
			     barra superior y repetirlo gastaría la mitad del alto.
			
			     Con botones sueltos y no con `categories`: las cinco pantallas no
			     están agrupadas, y un grupo con título sería un encabezado que hoy
			     no existe. -->
			<SideBar
				:collapse-label="t('barraLateral.plegar')"
				:expand-label="t('barraLateral.desplegar')"
			>
				<template #default="{ collapsed }">
					<SideButton
						v-for="p in PANTALLAS"
						:key="p"
						:label="t(`pantallas.${p}`)"
						:icon="ICONOS[p]"
						:active="pantalla === p"
						:collapsed="collapsed"
						@click="pantalla = p"
					/>

					<!-- El intervalo, al pie. Plegada no entra ni la etiqueta ni el
					     desplegable, así que se esconde. -->
					<div v-if="!collapsed" class="mt-auto flex flex-col gap-1 pt-3">
						<!-- La etiqueta la pone el propio selector y queda atada al control:
						     suelta acá al lado no estaba asociada a nada, así que un lector
						     de pantalla anunciaba un desplegable sin nombre y hacer clic en
						     el texto no abría la lista. -->
						<!-- Sin `.number`: las opciones atan el número con `:value="i"`, no
						     una cadena, así que lo que vuelve ya es un número y el
						     modificador no convierte nada. Lo pedía cuando el `value` era
						     texto. -->
						<SelectField v-model="intervalo" :label="t('ajustes.intervalo')">
							<option v-for="i in INTERVALOS" :key="i" :value="i">{{ i / 1000 }} s</option>
						</SelectField>
						<!-- Se dice que la medición se pausa: sin eso, alguien que abre el
						     monitor y lo deja de fondo supone que sigue gastando. -->
						<p class="text-tx-muted text-[11px]">{{ t('ajustes.pausaExplicada') }}</p>
					</div>
				</template>
			</SideBar>

			<!-- `@container`: lo que decide si algo cabe es el ancho de **esta** área, no
			     el de la ventana. Con cortes por viewport, la barra lateral de 208 px
			     no se descuenta y una ventana de 800 px pone dos columnas en 570 px de
			     espacio real. -->
			<main class="@container min-h-0 flex-1 overflow-y-auto p-3 sm:p-4">
				<h1 class="mb-4 flex items-center gap-2 font-medium text-tx-main text-xl">
					<ThemeIcon :name="ICONOS[pantalla]" :size="22" />
					{{ t(`pantallas.${pantalla}`) }}
				</h1>
				<RecursosView
					v-if="pantalla === 'recursos'"
					:intervalo="intervalo"
					:activa="esEnVivo(pantalla)"
				/>
				<AplicacionesView
					v-else-if="pantalla === 'aplicaciones'"
					:intervalo="intervalo"
					:activa="esEnVivo(pantalla)"
				/>
				<ServiciosView v-else-if="pantalla === 'servicios'" />
				<LimpiezaView v-else-if="pantalla === 'limpieza'" />
				<RegistrosView v-else-if="pantalla === 'registros'" />
			</main>
		</div>
	</WindowAppLayout>
</template>
