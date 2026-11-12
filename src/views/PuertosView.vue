<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Badge,
	EmptyState,
	ListGroup,
	ListRow,
	LoadingState,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { interpolar } from '@/tools/interpolar';

/** Los nombres de los campos son los del backend: son el formato del cable. */
interface Port {
	protocolo: string;
	direccion: string;
	puerto: number;
	proceso: string | null;
	pid: number | null;
}

const { t } = useI18n();
const ports = ref<Port[]>([]);
const error = ref('');
const loading = ref(true);

async function load() {
	loading.value = true;
	try {
		ports.value = await invoke<Port[]>('puertos_en_uso');
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		loading.value = false;
	}
}

/** Cuántos vinieron sin proceso: son los de otro usuario o de root, que `ss` no ve
    sin privilegios. Se avisa una vez si hay alguno, para que la columna vacía no
    parezca un error. */
const sinProceso = computed(() => ports.value.filter((p) => !p.proceso).length);

const protoTone = (p: Port): 'info' | 'neutral' => (p.protocolo === 'tcp' ? 'info' : 'neutral');

onMounted(load);
</script>

<template>
	<section class="flex flex-col gap-3">
		<header class="flex flex-wrap items-center gap-3">
			<span class="text-sm text-tx-muted">{{ interpolar(t('puertos.total'), ports.length) }}</span>
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
		<!-- Se dice una sola vez, no por fila: la columna vacía de un puerto ajeno
		     parece un error si no se explica que hace falta privilegio para verlo. -->
		<AlertMessage v-if="!loading && sinProceso > 0" tone="info" icon="dialog-information">
			{{ interpolar(t('puertos.sinPrivilegio'), sinProceso) }}
		</AlertMessage>

		<LoadingState v-if="loading" :label="t('common.cargando')" />
		<EmptyState v-else-if="ports.length === 0" icon="network-wired" :title="t('puertos.vacio')" />

		<ListGroup v-else>
			<ListRow v-for="p in ports" :key="`${p.protocolo}-${p.direccion}-${p.puerto}-${p.pid}`">
				<div class="flex flex-wrap items-center gap-3">
					<div class="min-w-[min(12rem,100%)] flex-1 basis-0">
						<div class="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
							<Badge :tone="protoTone(p)" class="font-mono uppercase">{{ p.protocolo }}</Badge>
							<span class="min-w-0 truncate font-mono text-sm text-tx-main" :title="`${p.direccion}:${p.puerto}`">
								{{ p.direccion }}:<span class="font-semibold">{{ p.puerto }}</span>
							</span>
						</div>
					</div>
					<span class="flex min-w-0 items-center gap-2">
						<ThemeIcon name="application-x-executable" :size="16" />
						<span v-if="p.proceso" class="min-w-0 truncate text-sm text-tx-main" :title="p.proceso">
							{{ p.proceso }}
						</span>
						<span v-else class="text-sm text-tx-muted">{{ t('puertos.sinProceso') }}</span>
						<Badge v-if="p.pid" class="font-mono">{{ interpolar(t('puertos.pid'), p.pid) }}</Badge>
					</span>
				</div>
			</ListRow>
		</ListGroup>
	</section>
</template>
