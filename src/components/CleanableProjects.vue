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
import {
	ActionButton,
	AlertMessage,
	Checkbox,
	ConfigSection,
	ListRow,
	ThemeIcon,
} from '@vasakgroup/vue-libvasak';
import { computed, ref } from 'vue';
import { tamano } from '@/tools/formato';
import { interpolar } from '@/tools/interpolar';

type ProjectKind = 'node' | 'cargo' | 'python' | 'gradle' | 'go' | 'compilacion';

interface Finding {
	ruta: string;
	clase: ProjectKind;
	proyecto: string;
	bytes: number | null;
}

const { t } = useI18n();

const findings = ref<Finding[]>([]);
const chosen = ref<Set<string>>(new Set());
const searching = ref(false);
const measuring = ref(false);
const deleting = ref<string | null>(null);
const deletingChosen = ref(false);
const error = ref('');
const notice = ref('');
const searched = ref(false);

const ICONS: Record<ProjectKind, string> = {
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
const chosenBytes = computed(() =>
	findings.value.filter((h) => chosen.value.has(h.ruta)).reduce((sum, h) => sum + (h.bytes ?? 0), 0)
);

const measuredTotal = computed(() => findings.value.reduce((sum, h) => sum + (h.bytes ?? 0), 0));

const allChosen = computed(
	() => findings.value.length > 0 && chosen.value.size === findings.value.length
);

/**
 * Si hay algo en curso que impida tocar la lista.
 *
 * **`measuring` cuenta.** `search` no espera a `measureAll` —a propósito, para que
 * la lista aparezca enseguida—, así que sin esto el botón de buscar se
 * rehabilitaba mientras el bucle de medición seguía andando. Una segunda pulsación
 * arrancaba un segundo bucle: el primero apagaba el cartel de «midiendo» antes de
 * tiempo y llamaba a `sortBySize()` mientras seguían llegando tamaños, así que las
 * filas se movían debajo del puntero.
 */
const busy = computed(
	() => searching.value || measuring.value || deletingChosen.value || deleting.value !== null
);

function toggle(path: string) {
	// Un Set nuevo y no `add`/`delete` sobre el mismo: Vue no ve las mutaciones
	// internas de un Set en un `ref`, y las casillas quedarían sin actualizarse.
	const next = new Set(chosen.value);
	if (next.has(path)) next.delete(path);
	else next.add(path);
	chosen.value = next;
}

function chooseAll() {
	chosen.value = new Set(findings.value.map((h) => h.ruta));
}

function chooseNone() {
	chosen.value = new Set();
}

async function search() {
	searching.value = true;
	error.value = '';
	notice.value = '';
	try {
		findings.value = await invoke<Finding[]>('proyectos_limpiables');
		chosen.value = new Set();
		searched.value = true;
		void measureAll();
	} catch (e) {
		error.value = String(e);
	} finally {
		searching.value = false;
	}
}

/**
 * Pide el tamaño de cada carpeta, una por una.
 *
 * En serie y no en paralelo: son todos `du` sobre el mismo disco, y lanzarlos
 * juntos los hace competir por la cabeza —o por la cola de la NVMe— sin terminar
 * antes. Y de paso el monitor no se convierte en la aplicación que más consume.
 */
async function measureAll() {
	measuring.value = true;
	for (const h of findings.value) {
		try {
			const bytes = await invoke<number | null>('medir_proyecto', { ruta: h.ruta });
			// Se busca de nuevo por ruta: si mientras medíamos se borró algo, el
			// índice ya no apunta a la misma fila.
			const current = findings.value.find((x) => x.ruta === h.ruta);
			if (current) current.bytes = bytes;
		} catch {
			// Una carpeta que no se pudo medir se queda sin tamaño. No es un error
			// que valga interrumpir el resto.
		}
	}
	measuring.value = false;
	sortBySize();
}

/** Lo más grande primero, que es lo que alguien vino a buscar. */
function sortBySize() {
	findings.value = [...findings.value].sort((a, b) => (b.bytes ?? 0) - (a.bytes ?? 0));
}

async function remove(path: string) {
	deleting.value = path;
	error.value = '';
	try {
		const bytes = await invoke<number>('borrar_proyecto', { ruta: path });
		findings.value = findings.value.filter((h) => h.ruta !== path);
		const next = new Set(chosen.value);
		next.delete(path);
		chosen.value = next;
		notice.value = interpolar(t('limpieza.proyectos.recuperado'), tamano(bytes));
	} catch (e) {
		error.value = String(e);
	} finally {
		deleting.value = null;
	}
}

async function removeChosen() {
	deletingChosen.value = true;
	error.value = '';
	let reclaimed = 0;
	const failures: string[] = [];

	// Sobre una copia: `remove` modifica la lista y el Set mientras esto recorre.
	for (const path of [...chosen.value]) {
		try {
			reclaimed += await invoke<number>('borrar_proyecto', { ruta: path });
			findings.value = findings.value.filter((h) => h.ruta !== path);
		} catch (e) {
			failures.push(String(e));
		}
	}

	chosen.value = new Set();
	deletingChosen.value = false;
	// El total recuperado se informa aunque alguna haya fallado: lo que se borró,
	// se borró, y decir sólo el error escondería lo que sí pasó.
	notice.value = interpolar(t('limpieza.proyectos.recuperado'), tamano(reclaimed));
	if (failures.length) error.value = failures.join('\n');
}
</script>

<template>
	<!-- La tarjeta es un `ConfigSection`, con el total medido en `aside`: era un
	     `article` escrito a mano con el mismo borde que las de Recursos. -->
	<ConfigSection
		:title="t('limpieza.proyectos.titulo')"
		icon="applications-development"
		icon-type="icon"
		as="h2"
	>
		<template v-if="findings.length" #aside>
			<span class="font-mono text-lg text-tx-main">{{ tamano(measuredTotal) }}</span>
		</template>

		<div class="flex min-w-0 flex-col gap-3">
			<p class="text-tx-muted text-xs">{{ t('limpieza.proyectos.intro') }}</p>
			<!-- La regla de seguridad se dice, no se esconde: es lo que explica por qué
			     una carpeta que alguien esperaba ver no aparece en la lista. -->
			<p class="text-tx-muted text-xs">{{ t('limpieza.proyectos.seguridad') }}</p>

			<!-- `whitespace-pre-line` porque el error del backend viene con saltos:
			     es la lista de rutas que no se pudieron borrar, una por línea. -->
			<AlertMessage v-if="error" tone="error" icon="dialog-error" class="whitespace-pre-line">
				{{ error }}
			</AlertMessage>
			<AlertMessage v-if="notice" tone="success" icon="object-select">{{ notice }}</AlertMessage>

			<div class="flex flex-wrap items-center gap-2">
				<ActionButton
					:label="searching ? t('limpieza.proyectos.buscando') : t('limpieza.proyectos.buscar')"
					variant="secondary"
					:disabled="busy"
					@click="search()" />

				<template v-if="findings.length">
					<ActionButton
						:label="
							allChosen
								? t('limpieza.proyectos.limpiarSeleccion')
								: t('limpieza.proyectos.seleccionarTodo')
						"
						variant="ghost"
						size="sm"
						:disabled="busy"
						@click="allChosen ? chooseNone() : chooseAll()" />
					<span class="text-tx-muted text-xs">
						{{ interpolar(t('limpieza.proyectos.carpetas'), findings.length) }}
						<template v-if="measuring"> — {{ t('limpieza.proyectos.midiendo') }}</template>
					</span>
					<ActionButton
						v-if="chosen.size"
						:label="interpolar(t('limpieza.proyectos.borrarSeleccion'), tamano(chosenBytes))"
						variant="primary"
						class="ml-auto"
						:disabled="busy"
						@click="removeChosen()" />
				</template>
			</div>

			<!-- `role="status"` y no un estado vacío entero: es la respuesta a un
			     botón que se acaba de apretar, no la pantalla. Sin el rol, buscar y
			     no encontrar nada es indistinguible de que no haya pasado nada. -->
			<p
				v-if="searched && !findings.length && !searching"
				role="status"
				class="text-sm text-tx-muted">
				{{ t('limpieza.proyectos.vacio') }}
			</p>

			<div v-if="findings.length" class="flex flex-col divide-y divide-ui-line-weak">
				<ListRow v-for="h in findings" :key="h.ruta">
					<template #leading>
						<!-- La casilla de la librería, con el nombre escondido: lo que se lee
						     es la etiqueta de al lado, atada por `for` al mismo `id`, así que
						     tocar el nombre del proyecto también la marca. -->
						<Checkbox
							:id="`project-${h.ruta}`"
							:model-value="chosen.has(h.ruta)"
							:label="h.proyecto"
							hide-label
							:disabled="busy"
							@change="toggle(h.ruta)" />
					</template>
					<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
						<ThemeIcon :name="ICONS[h.clase]" :size="20" class="shrink-0" />
						<label :for="`project-${h.ruta}`" class="flex min-w-0 flex-1 flex-col">
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
						<ActionButton
							:label="deleting === h.ruta ? '…' : t('limpieza.proyectos.borrar')"
							variant="secondary"
							size="sm"
							class="shrink-0"
							:disabled="busy"
							@click="remove(h.ruta)" />
					</div>
				</ListRow>
			</div>
		</div>
	</ConfigSection>
</template>
