<script setup lang="ts">
/**
 * Las carpetas de dependencias y compilación que los proyectos regeneran.
 *
 * # Por qué el escaneo es a pedido
 *
 * Recorrer `$HOME` tarda de medio segundo a un par —en esta máquina, 54
 * candidatas en 1,4 s— y no cambia de un minuto al otro. Hacerlo al abrir la
 * pantalla gastaría eso cada vez que alguien pasa por acá a mirar la caché, así
 * que hay un botón.
 *
 * # Por qué los tamaños llegan después
 *
 * Medir es lo lento: un `du` sobre un `target` de 40 GB tarda más que todo el
 * escaneo. Se pide de a uno y la lista se va completando, en lugar de quedarse en
 * blanco hasta que estén todos. La lista sin tamaños ya sirve: dice qué hay.
 */
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { computed, ref } from 'vue';
import ThemeIcon from '@/components/ThemeIcon.vue';
import { tamano } from '@/tools/formato';
import { interpolar } from '@/tools/interpolar';

type Clase = 'node' | 'cargo' | 'python' | 'gradle' | 'go' | 'compilacion';

interface Hallazgo {
	ruta: string;
	clase: Clase;
	proyecto: string;
	bytes: number | null;
}

const { t } = useI18n();

const lista = ref<Hallazgo[]>([]);
const elegidas = ref<Set<string>>(new Set());
const buscando = ref(false);
const midiendo = ref(false);
const borrando = ref<string | null>(null);
const borrandoTodo = ref(false);
const error = ref('');
const aviso = ref('');
const yaBusco = ref(false);

const ICONOS: Record<Clase, string> = {
	node: 'application-javascript',
	cargo: 'text-rust',
	python: 'text-x-python',
	gradle: 'application-x-java',
	go: 'text-x-go',
	compilacion: 'package-x-generic',
};

/**
 * Lo elegido, con su tamaño conocido.
 *
 * Sólo cuenta lo medido: sumar los `null` como cero mostraría un total que crece
 * solo a medida que llegan las mediciones, y eso se lee como si el disco estuviera
 * cambiando.
 */
const bytesElegidos = computed(() =>
	lista.value
		.filter((h) => elegidas.value.has(h.ruta))
		.reduce((suma, h) => suma + (h.bytes ?? 0), 0)
);

const totalMedido = computed(() => lista.value.reduce((suma, h) => suma + (h.bytes ?? 0), 0));

const todasElegidas = computed(
	() => lista.value.length > 0 && elegidas.value.size === lista.value.length
);

const ocupada = computed(() => buscando.value || borrandoTodo.value || borrando.value !== null);

function alternar(ruta: string) {
	// Un Set nuevo y no `add`/`delete` sobre el mismo: Vue no ve las mutaciones
	// internas de un Set en un `ref`, y las casillas quedarían sin actualizarse.
	const siguiente = new Set(elegidas.value);
	if (siguiente.has(ruta)) siguiente.delete(ruta);
	else siguiente.add(ruta);
	elegidas.value = siguiente;
}

function elegirTodo() {
	elegidas.value = new Set(lista.value.map((h) => h.ruta));
}

function elegirNada() {
	elegidas.value = new Set();
}

async function buscar() {
	buscando.value = true;
	error.value = '';
	aviso.value = '';
	try {
		lista.value = await invoke<Hallazgo[]>('proyectos_limpiables');
		elegidas.value = new Set();
		yaBusco.value = true;
		void medirTodo();
	} catch (e) {
		error.value = String(e);
	} finally {
		buscando.value = false;
	}
}

/**
 * Pide el tamaño de cada carpeta, una por una.
 *
 * En serie y no en paralelo: son todos `du` sobre el mismo disco, y lanzarlos
 * juntos los hace competir por la cabeza —o por la cola de la NVMe— sin terminar
 * antes. Y de paso el monitor no se convierte en la aplicación que más consume.
 */
async function medirTodo() {
	midiendo.value = true;
	for (const h of lista.value) {
		try {
			const bytes = await invoke<number | null>('medir_proyecto', { ruta: h.ruta });
			// Se busca de nuevo por ruta: si mientras medíamos se borró algo, el
			// índice ya no apunta a la misma fila.
			const actual = lista.value.find((x) => x.ruta === h.ruta);
			if (actual) actual.bytes = bytes;
		} catch {
			// Una carpeta que no se pudo medir se queda sin tamaño. No es un error
			// que valga interrumpir el resto.
		}
	}
	midiendo.value = false;
	ordenar();
}

/** Lo más grande primero, que es lo que alguien vino a buscar. */
function ordenar() {
	lista.value = [...lista.value].sort((a, b) => (b.bytes ?? 0) - (a.bytes ?? 0));
}

async function borrar(ruta: string) {
	borrando.value = ruta;
	error.value = '';
	try {
		const bytes = await invoke<number>('borrar_proyecto', { ruta });
		lista.value = lista.value.filter((h) => h.ruta !== ruta);
		const siguiente = new Set(elegidas.value);
		siguiente.delete(ruta);
		elegidas.value = siguiente;
		aviso.value = interpolar(t('limpieza.proyectos.recuperado'), tamano(bytes));
	} catch (e) {
		error.value = String(e);
	} finally {
		borrando.value = null;
	}
}

async function borrarElegidas() {
	borrandoTodo.value = true;
	error.value = '';
	let recuperado = 0;
	const fallos: string[] = [];

	// Sobre una copia: `borrar` modifica la lista y el Set mientras esto recorre.
	for (const ruta of [...elegidas.value]) {
		try {
			recuperado += await invoke<number>('borrar_proyecto', { ruta });
			lista.value = lista.value.filter((h) => h.ruta !== ruta);
		} catch (e) {
			fallos.push(String(e));
		}
	}

	elegidas.value = new Set();
	borrandoTodo.value = false;
	// El total recuperado se informa aunque alguna haya fallado: lo que se borró,
	// se borró, y decir sólo el error escondería lo que sí pasó.
	aviso.value = interpolar(t('limpieza.proyectos.recuperado'), tamano(recuperado));
	if (fallos.length) error.value = fallos.join('\n');
}
</script>

<template>
	<article class="flex flex-col gap-3 rounded-corner border border-ui-border bg-ui-surface/40 p-4">
		<header class="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
			<h2 class="flex items-center gap-2 font-medium text-tx-main">
				<ThemeIcon nombre="applications-development" :tamano="18" />
				{{ t('limpieza.proyectos.titulo') }}
			</h2>
			<span v-if="lista.length" class="font-mono text-lg text-tx-main">
				{{ tamano(totalMedido) }}
			</span>
		</header>

		<p class="text-tx-muted text-xs">{{ t('limpieza.proyectos.intro') }}</p>
		<!-- La regla de seguridad se dice, no se esconde: es lo que explica por qué
		     una carpeta que alguien esperaba ver no aparece en la lista. -->
		<p class="text-tx-muted text-xs">{{ t('limpieza.proyectos.seguridad') }}</p>

		<p
			v-if="error"
			class="whitespace-pre-line rounded-corner bg-status-error/10 px-3 py-2 text-sm text-status-error"
		>
			{{ error }}
		</p>
		<p v-if="aviso" class="text-sm text-status-success">{{ aviso }}</p>

		<div class="flex flex-wrap items-center gap-2">
			<button
				type="button"
				class="rounded-corner border border-ui-border-strong px-3 py-1.5 text-sm text-tx-main transition-colors hover:bg-ui-surface disabled:opacity-50"
				:disabled="ocupada"
				@click="buscar()"
			>
				{{ buscando ? t('limpieza.proyectos.buscando') : t('limpieza.proyectos.buscar') }}
			</button>

			<template v-if="lista.length">
				<button
					type="button"
					class="rounded-corner px-2 py-1.5 text-tx-muted text-xs hover:bg-ui-surface"
					:disabled="ocupada"
					@click="todasElegidas ? elegirNada() : elegirTodo()"
				>
					{{
						todasElegidas
							? t('limpieza.proyectos.limpiarSeleccion')
							: t('limpieza.proyectos.seleccionarTodo')
					}}
				</button>
				<span class="text-tx-muted text-xs">
					{{ interpolar(t('limpieza.proyectos.carpetas'), lista.length) }}
					<template v-if="midiendo"> — {{ t('limpieza.proyectos.midiendo') }}</template>
				</span>
				<button
					v-if="elegidas.size"
					type="button"
					class="ml-auto rounded-corner bg-primary px-3 py-1.5 font-medium text-sm text-tx-on-primary transition-all hover:brightness-110 disabled:opacity-50"
					:disabled="ocupada"
					@click="borrarElegidas()"
				>
					{{ interpolar(t('limpieza.proyectos.borrarSeleccion'), tamano(bytesElegidos)) }}
				</button>
			</template>
		</div>

		<p v-if="yaBusco && !lista.length && !buscando" class="text-sm text-tx-muted">
			{{ t('limpieza.proyectos.vacio') }}
		</p>

		<ul v-if="lista.length" class="flex flex-col divide-y divide-ui-border">
			<li v-for="h in lista" :key="h.ruta" class="flex items-center gap-3 py-2">
				<input
					:id="`proy-${h.ruta}`"
					type="checkbox"
					class="size-4 shrink-0 accent-primary"
					:checked="elegidas.has(h.ruta)"
					:disabled="ocupada"
					@change="alternar(h.ruta)"
				/>
				<ThemeIcon :nombre="ICONOS[h.clase]" :tamano="20" class="shrink-0" />
				<label :for="`proy-${h.ruta}`" class="flex min-w-0 flex-col">
					<span class="truncate text-sm text-tx-main">{{ h.proyecto }}</span>
					<!-- La ruta completa en chico: el nombre del proyecto solo no alcanza
					     cuando hay tres `target` de paquetes distintos del mismo repo. -->
					<span class="truncate font-mono text-tx-muted text-xs" :title="h.ruta">
						{{ h.ruta }}
					</span>
					<span class="text-tx-muted text-xs">
						{{ t(`limpieza.proyectos.clases.${h.clase}`) }}
					</span>
				</label>
				<span class="ml-auto shrink-0 font-mono text-sm text-tx-main tabular-nums">
					{{ h.bytes === null ? '…' : tamano(h.bytes) }}
				</span>
				<button
					type="button"
					class="shrink-0 rounded-corner border border-ui-border-strong px-2 py-1 text-xs text-tx-main transition-colors hover:bg-ui-surface disabled:opacity-50"
					:disabled="ocupada"
					@click="borrar(h.ruta)"
				>
					{{ borrando === h.ruta ? '…' : t('limpieza.proyectos.borrar') }}
				</button>
			</li>
		</ul>
	</article>
</template>
