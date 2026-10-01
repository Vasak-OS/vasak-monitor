<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Badge,
	Checkbox,
	ListGroup,
	ListRow,
	LoadingState,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { interpolar } from '@/tools/interpolar';

/** Los nombres de los campos son los del backend: son el formato del cable. */
interface Service {
	unidad: string;
	estado: string;
	detalle: string;
	descripcion: string;
	del_usuario: boolean;
	de_vasakos: boolean;
}

type ServiceAction = 'start' | 'stop' | 'restart';
const ACTIONS: readonly ServiceAction[] = ['start', 'stop', 'restart'];

const { t } = useI18n();
const services = ref<Service[]>([]);
const error = ref('');
const loading = ref(true);
const onlyVasakOS = ref(true);
const busy = ref('');

async function load() {
	loading.value = true;
	try {
		services.value = await invoke<Service[]>('lista_de_servicios');
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		loading.value = false;
	}
}

const visible = computed(() =>
	onlyVasakOS.value ? services.value.filter((s) => s.de_vasakos) : services.value
);

const failed = computed(() => services.value.filter((s) => s.estado === 'failed').length);

async function run(s: Service, action: ServiceAction) {
	busy.value = s.unidad;
	error.value = '';
	try {
		await invoke('accion_de_servicio', {
			unidad: s.unidad,
			accion: action,
			delUsuario: s.del_usuario,
		});
		await load();
	} catch (e) {
		error.value = String(e);
	} finally {
		busy.value = '';
	}
}

const stateTone = (s: Service) =>
	s.estado === 'failed'
		? 'text-status-error'
		: s.estado === 'active'
			? 'text-status-success'
			: 'text-tx-muted';

onMounted(load);
</script>

<template>
	<section class="flex flex-col gap-3">
		<header class="flex flex-wrap items-center gap-3">
			<!-- La casilla de la librería: la nativa se dibujaba con el color del
			     sistema, que en una ventana oscura era un cuadrado blanco. -->
			<Checkbox v-model="onlyVasakOS" :label="t('servicios.soloVasakOS')" />
			<span v-if="failed > 0" class="flex items-center gap-1.5 text-sm text-status-error">
				<ThemeIcon name="dialog-error" :size="16" />
				{{ interpolar(t('servicios.fallidos'), failed) }}
			</span>
			<ActionButton
				:label="t('common.actualizar')"
				icon="view-refresh"
				variant="secondary"
				class="ml-auto"
				@click="load()" />
		</header>

		<AlertMessage v-if="error" tone="error" icon="dialog-error">
			{{ error }}
		</AlertMessage>
		<LoadingState v-if="loading" :label="t('common.cargando')" />

		<ListGroup v-else>
			<ListRow v-for="s in visible" :key="s.unidad">
				<!-- Todo en la ranura principal para que la fila se pueda partir: en
				     angosto los botones bajan a la línea siguiente en vez de salirse. -->
				<div class="flex flex-wrap items-center gap-3">
					<div class="min-w-0 flex-1">
						<div class="flex min-w-0 items-center gap-2">
							<ThemeIcon name="system-run" :size="16" />
							<span class="min-w-0 truncate text-sm text-tx-main" :title="s.unidad">{{ s.unidad }}</span>
							<span :class="stateTone(s)" class="shrink-0 font-mono text-xs">{{ s.estado }}</span>
							<!-- Se dice de qué instancia es: los del sistema piden
							     autenticar y los del usuario no, y sin decirlo la
							     contraseña aparece sin explicación. -->
							<Badge v-if="!s.del_usuario" :label="t('servicios.delSistema')" />
						</div>
						<p v-if="s.descripcion" class="truncate text-tx-muted text-xs" :title="s.descripcion">
							{{ s.descripcion }}
						</p>
					</div>
					<div class="flex shrink-0 flex-wrap gap-1">
						<ActionButton
							v-for="action in ACTIONS"
							:key="action"
							:label="t(`servicios.${action}`)"
							variant="secondary"
							size="sm"
							:disabled="busy === s.unidad"
							@click="run(s, action)" />
					</div>
				</div>
			</ListRow>
		</ListGroup>
	</section>
</template>
