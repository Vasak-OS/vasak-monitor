<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, onMounted, ref } from 'vue';
import { interpolar } from '@/tools/interpolar';

interface Entrada {
	microsegundos: number;
	origen: string;
	nivel: number;
	mensaje: string;
}

const { t } = useI18n();
const lista = ref<Entrada[]>([]);
const desplazamiento = ref(0);
const cargando = ref(true);
const error = ref('');
const soloProblemas = ref(false);
const filtro = ref('');

async function cargar() {
	cargando.value = true;
	try {
		desplazamiento.value = await invoke<number>('desplazamiento_horario');
		lista.value = await invoke<Entrada[]>('registros_de_vasakos', {
			soloProblemas: soloProblemas.value,
			cantidad: 500,
		});
		error.value = '';
	} catch (e) {
		error.value = String(e);
	} finally {
		cargando.value = false;
	}
}

const visibles = computed(() => {
	const q = filtro.value.trim().toLowerCase();
	if (!q) return lista.value;
	return lista.value.filter(
		(e) => e.mensaje.toLowerCase().includes(q) || e.origen.toLowerCase().includes(q)
	);
});

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

onMounted(cargar);
</script>

<template>
	<section class="flex min-h-0 flex-col gap-3">
		<header class="flex flex-wrap items-center gap-3">
			<label class="flex items-center gap-2 text-sm text-tx-primary">
				<input v-model="soloProblemas" type="checkbox" @change="cargar()" />
				{{ t('registros.soloProblemas') }}
			</label>
			<input
				v-model="filtro"
				type="search"
				:placeholder="t('registros.buscar')"
				class="min-w-40 flex-1 rounded-corner border border-ui-border bg-ui-surface/40 px-3 py-1.5 text-sm text-tx-primary"
			/>
			<button
				type="button"
				class="rounded-corner border border-ui-border px-3 py-1.5 text-sm text-tx-primary hover:bg-ui-surface"
				@click="cargar()"
			>
				{{ t('common.actualizar') }}
			</button>
		</header>

		<p v-if="error" class="rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error">
			{{ error }}
		</p>
		<p v-if="cargando" class="text-sm text-tx-muted">{{ t('common.cargando') }}</p>

		<template v-else>
			<p class="text-tx-muted text-xs">
				{{ interpolar(t('registros.cuantas'), visibles.length) }}
			</p>
			<ul class="flex min-h-0 flex-1 flex-col divide-y divide-ui-border overflow-y-auto rounded-corner border border-ui-border">
				<li v-for="(e, i) in visibles" :key="`${e.microsegundos}-${i}`" class="flex gap-3 bg-ui-surface/40 px-4 py-2">
					<span class="shrink-0 font-mono text-tx-muted text-xs">{{ hora(e.microsegundos) }}</span>
					<span class="w-44 shrink-0 truncate text-tx-muted text-xs">{{ e.origen }}</span>
					<span :class="tonoDeNivel(e.nivel)" class="min-w-0 flex-1 break-words text-xs">{{ e.mensaje }}</span>
				</li>
			</ul>
		</template>
	</section>
</template>
