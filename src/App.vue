<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { listen } from '@tauri-apps/api/event';
import { useConfigStore } from '@vasakgroup/plugin-config-manager';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
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
		<div class="flex h-full min-h-0">
			<nav class="flex w-52 shrink-0 flex-col gap-1 border-ui-border border-r p-3">
				<button
					v-for="p in PANTALLAS"
					:key="p"
					type="button"
					class="flex items-center gap-2 rounded-corner px-3 py-2 text-left text-sm transition-colors"
					:class="
						pantalla === p
							? 'bg-primary/15 font-medium text-primary'
							: 'text-tx-primary hover:bg-ui-surface'
					"
					@click="pantalla = p"
				>
					{{ t(`pantallas.${p}`) }}
				</button>

				<div class="mt-auto flex flex-col gap-1 pt-3">
					<label class="text-tx-muted text-xs">{{ t('ajustes.intervalo') }}</label>
					<select
						v-model.number="intervalo"
						class="rounded-corner border border-ui-border bg-ui-surface/40 px-2 py-1 text-sm text-tx-primary"
					>
						<option v-for="i in INTERVALOS" :key="i" :value="i">{{ i / 1000 }} s</option>
					</select>
					<!-- Se dice que la medición se pausa: sin eso, alguien que abre el
					     monitor y lo deja de fondo supone que sigue gastando. -->
					<p class="text-tx-muted text-[11px]">{{ t('ajustes.pausaExplicada') }}</p>
				</div>
			</nav>

			<main class="min-h-0 flex-1 overflow-y-auto p-4">
				<h1 class="mb-4 font-medium text-tx-primary text-xl">{{ t(`pantallas.${pantalla}`) }}</h1>
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
