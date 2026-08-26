<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import { interpolar } from '@/tools/interpolar';

interface Servicio {
	unidad: string;
	estado: string;
	detalle: string;
	descripcion: string;
	del_usuario: boolean;
	de_vasakos: boolean;
}

const { t } = useI18n();
const lista = ref<Servicio[]>([]);
const error = ref('');
const cargando = ref(true);
const soloVasakOS = ref(true);
const ocupada = ref('');

async function cargar() {
	cargando.value = true;
	try {
		lista.value = await invoke<Servicio[]>('lista_de_servicios');
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		cargando.value = false;
	}
}

const visibles = computed(() =>
	soloVasakOS.value ? lista.value.filter((s) => s.de_vasakos) : lista.value
);

const fallidos = computed(() => lista.value.filter((s) => s.estado === 'failed').length);

async function accion(s: Servicio, cual: 'start' | 'stop' | 'restart') {
	ocupada.value = s.unidad;
	error.value = '';
	try {
		await invoke('accion_de_servicio', {
			unidad: s.unidad,
			accion: cual,
			delUsuario: s.del_usuario,
		});
		await cargar();
	} catch (e) {
		error.value = String(e);
	} finally {
		ocupada.value = '';
	}
}

const tonoDeEstado = (s: Servicio) =>
	s.estado === 'failed'
		? 'text-status-error'
		: s.estado === 'active'
			? 'text-status-success'
			: 'text-tx-muted';

onMounted(cargar);
</script>

<template>
	<section class="flex flex-col gap-3">
		<header class="flex flex-wrap items-center gap-3">
			<label class="flex items-center gap-2 text-sm text-tx-primary">
				<input v-model="soloVasakOS" type="checkbox" />
				{{ t('servicios.soloVasakOS') }}
			</label>
			<span v-if="fallidos > 0" class="text-sm text-status-error">
				{{ interpolar(t('servicios.fallidos'), fallidos) }}
			</span>
			<button
				type="button"
				class="ml-auto rounded-corner border border-ui-border px-3 py-1.5 text-sm text-tx-primary hover:bg-ui-surface"
				@click="cargar()"
			>
				{{ t('common.actualizar') }}
			</button>
		</header>

		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>
		<p v-if="cargando" class="text-sm text-tx-muted">{{ t('common.cargando') }}</p>

		<ul v-else class="divide-y divide-ui-border overflow-hidden rounded-corner border border-ui-border">
			<li v-for="s in visibles" :key="s.unidad" class="flex flex-wrap items-center gap-3 bg-ui-surface/40 px-4 py-2.5">
				<div class="min-w-0 flex-1">
					<div class="flex items-center gap-2">
						<span class="truncate text-sm text-tx-primary">{{ s.unidad }}</span>
						<span :class="tonoDeEstado(s)" class="shrink-0 font-mono text-xs">{{ s.estado }}</span>
						<!-- Se dice de qué instancia es: los del sistema piden
						     autenticar y los del usuario no, y sin decirlo la
						     contraseña aparece sin explicación. -->
						<span v-if="!s.del_usuario" class="shrink-0 rounded bg-ui-surface px-1.5 py-0.5 text-[10px] text-tx-muted">
							{{ t('servicios.delSistema') }}
						</span>
					</div>
					<p v-if="s.descripcion" class="truncate text-tx-muted text-xs">{{ s.descripcion }}</p>
				</div>
				<div class="flex shrink-0 gap-1">
					<button
						v-for="cual in (['start', 'stop', 'restart'] as const)"
						:key="cual"
						type="button"
						:disabled="ocupada === s.unidad"
						class="rounded-corner border border-ui-border px-2.5 py-1 text-tx-muted text-xs hover:bg-ui-surface disabled:opacity-50"
						@click="accion(s, cual)"
					>
						{{ t(`servicios.${cual}`) }}
					</button>
				</div>
			</li>
		</ul>
	</section>
</template>
