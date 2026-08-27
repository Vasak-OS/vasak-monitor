<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { onMounted, onUnmounted, ref } from 'vue';
import SelectField from '@/components/SelectField.vue';
import ThemeIcon from '@/components/ThemeIcon.vue';
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
		<div class="flex h-full min-h-0 w-full">
			<nav
				class="flex w-14 shrink-0 flex-col gap-1 overflow-hidden border-ui-border border-r p-2 transition-all sm:w-52 sm:p-3"
			>
				<button
					v-for="p in PANTALLAS"
					:key="p"
					type="button"
					class="flex items-center justify-center gap-2 rounded-corner px-2 py-2 text-left text-sm transition-colors sm:justify-start sm:px-3"
					:class="
						pantalla === p
							? 'bg-primary/15 font-medium text-primary'
							: 'text-tx-main hover:bg-ui-surface'
					"
					@click="pantalla = p"
				>
					<ThemeIcon :nombre="ICONOS[p]" :tamano="18" />
					<span class="hidden truncate sm:inline">{{ t(`pantallas.${p}`) }}</span>
				</button>

				<div class="mt-auto hidden flex-col gap-1 pt-3 sm:flex">
					<label class="text-tx-muted text-xs">{{ t('ajustes.intervalo') }}</label>
					<SelectField v-model.number="intervalo">
						<option v-for="i in INTERVALOS" :key="i" :value="i">{{ i / 1000 }} s</option>
					</SelectField>
					<!-- Se dice que la medición se pausa: sin eso, alguien que abre el
					     monitor y lo deja de fondo supone que sigue gastando. -->
					<p class="text-tx-muted text-[11px]">{{ t('ajustes.pausaExplicada') }}</p>
				</div>
			</nav>

			<!-- `@container`: lo que decide si algo cabe es el ancho de **esta** área, no
			     el de la ventana. Con cortes por viewport, la barra lateral de 208 px
			     no se descuenta y una ventana de 800 px pone dos columnas en 570 px de
			     espacio real. -->
			<main class="@container min-h-0 flex-1 overflow-y-auto p-3 sm:p-4">
				<h1 class="mb-4 flex items-center gap-2 font-medium text-tx-main text-xl">
					<ThemeIcon :nombre="ICONOS[pantalla]" :tamano="22" />
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
