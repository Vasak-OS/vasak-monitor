/**
 * Lo que el monitor dejó de dibujar por su cuenta.
 *
 * Tenía su propio `ThemeIcon` y su propio `useReactiveIcon` —ciento cuarenta y
 * cinco líneas para lo que la librería ya hace mejor: memoriza lo resuelto y
 * usa un solo oyente para todas las instancias, no uno por icono—, su propia
 * barra de carga, sus buscadores y seis cajas de error idénticas.
 *
 * Lo que se comprueba es lo que **no se ve** y por eso no se nota si se rompe:
 * que la barra diga cuánto y de qué, que los buscadores tengan nombre, que un
 * error interrumpa y que «midiendo» se anuncie. Nada de eso existía: las barras
 * eran cajas de colores sin nombre, los campos sólo tenían `placeholder` —que
 * se va con la primera letra y que un lector de pantalla no está obligado a
 * leer—, y las seis cajas de error no tenían rol.
 */

import { afterEach, beforeAll, describe, expect, test } from 'bun:test';
import {
	ActionButton,
	AlertMessage,
	Badge,
	Checkbox,
	ConfigSection,
	Disclosure,
	EmptyState,
	ListGroup,
	ListRow,
	LoadingState,
	olvidarLosIconosDelTema,
	ProgressBar,
	StatTile,
} from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import CleanableProjects from '@/components/CleanableProjects.vue';
import AppsView from '@/views/AppsView.vue';
import CleanupView from '@/views/CleanupView.vue';
import LogsView from '@/views/LogsView.vue';
import ResourcesView from '@/views/ResourcesView.vue';
import ServicesView from '@/views/ServicesView.vue';
import { invoke, olvidarTodo, responderInvoke } from './dobles';

const ROOT = new URL('..', import.meta.url).pathname;

const mounted = new Set<VueWrapper>();

function track<T extends VueWrapper>(wrapper: T): T {
	mounted.add(wrapper);
	return wrapper;
}

/**
 * El primer montaje del archivo, fuera de toda prueba.
 *
 * Bun compila cada `.vue` al importarlo, y ese costo lo pagaba entero la
 * primera prueba que montara: se pasaba del límite de cinco segundos y fallaba
 * por tiempo sin tener nada roto.
 */
beforeAll(async () => {
	const warmUp = mount(ResourcesView, { props: { interval: 100000, active: false } });
	warmUp.unmount();
}, 60_000);

afterEach(() => {
	for (const wrapper of mounted) wrapper.unmount();
	mounted.clear();
	olvidarTodo();
	olvidarLosIconosDelTema();
});

/** Un equipo de mentira, con el disco casi lleno para ver el tono. */
const RESOURCES = {
	cpu: 12,
	nucleos: 8,
	ram_usada: 8_000_000_000,
	ram_total: 16_000_000_000,
	ram_cache: 0,
	swap: 0,
	bajada: 0,
	subida: 0,
	discos: [{ punto: '/', tipo: 'ext4', total: 100, usado: 95 }],
};

async function mountResources(data: unknown = RESOURCES) {
	responderInvoke('recursos', data);
	const wrapper = track(mount(ResourcesView, { props: { interval: 100000, active: true } }));
	// `useSondeo` mide desde `onMounted` y la medición es asíncrona.
	await new Promise((done) => setTimeout(done, 0));
	await nextTick();
	return wrapper;
}

describe('las barras de carga', () => {
	test('dicen cuánto y de qué, que antes eran una caja de colores', async () => {
		// Sin `role="progressbar"` y sin nombre, el uso de CPU es un rectángulo
		// que cambia de ancho: para quien no lo ve, nada.
		const wrapper = await mountResources();

		const bars = wrapper.findAllComponents(ProgressBar);
		expect(bars.length).toBeGreaterThan(2);

		const cpu = bars[0];
		expect(cpu.attributes('role')).toBe('progressbar');
		expect(cpu.attributes('aria-label')).toBe('recursos.cpu');
		expect(cpu.attributes('aria-valuenow')).toBe('12');
	});

	test('y el disco casi lleno se pinta distinto, que es para lo que sirve un monitor', async () => {
		// El disco de la muestra está al 95 %. Es lo único que la copia propia
		// tenía y la de la librería no, así que si el tono no llegara el color
		// se perdería sin que nada falle.
		const wrapper = await mountResources();

		const disk = wrapper.findAllComponents(ProgressBar).at(-1);
		expect(disk?.props('tone')).toBe('critical');
		expect(disk?.attributes('aria-label')).toBe('/');
	});

	test('mientras no midió, lo dice en voz alta', async () => {
		// Antes era un párrafo gris sin rol: la ventana quedaba en blanco y sin
		// explicación hasta que el backend contestara.
		const wrapper = track(mount(ResourcesView, { props: { interval: 100000, active: false } }));
		await nextTick();

		const loading = wrapper.findComponent(LoadingState);
		expect(loading.exists()).toBe(true);
		expect(loading.attributes('role')).toBe('status');
	});

	test('y lo que falla interrumpe en vez de esperar turno', async () => {
		const wrapper = await mountResources(new Error('no se pudo medir'));

		const alert = wrapper.findComponent(AlertMessage);
		expect(alert.exists()).toBe(true);
		expect(alert.attributes('role')).toBe('alert');
		expect(wrapper.text()).toContain('no se pudo medir');
	});
});

describe('los buscadores', () => {
	test('tienen nombre, y no sólo un texto de relleno que se va al escribir', async () => {
		// El `placeholder` no es un nombre: desaparece con la primera letra, y
		// un lector de pantalla no tiene obligación de leerlo.
		responderInvoke('aplicaciones', []);
		const wrapper = track(mount(AppsView, { props: { interval: 100000, active: true } }));
		await new Promise((done) => setTimeout(done, 0));
		await nextTick();

		const field = wrapper.find('input[type="search"]');
		expect(field.exists()).toBe(true);
		expect(field.attributes('aria-label')).toBe('aplicaciones.buscar');
	});
});

/** Monta una vista y espera a que conteste el `invoke` del `onMounted`. */
async function mountView<T>(component: T, props: Record<string, unknown> = {}) {
	const wrapper = track(mount(component as never, { props } as never));
	for (let i = 0; i < 4; i++) {
		await new Promise((done) => setTimeout(done, 0));
		await nextTick();
	}
	return wrapper;
}

const APPS = [
	{ pid: 1, nombre: 'firefox', memoria: 1_000_000, cpu: 1, con_ventana: true },
	{ pid: 2, nombre: 'pipewire', memoria: 1_000, cpu: 0, con_ventana: false },
];

const SERVICES = [
	{ unidad: 'a.service', estado: 'active', detalle: '', descripcion: 'A', del_usuario: true, de_vasakos: true },
	{ unidad: 'b.service', estado: 'failed', detalle: '', descripcion: 'B', del_usuario: false, de_vasakos: true },
];

describe('las piezas de la 2.2 (vue-libvasak#74)', () => {
	test('las tarjetas de Recursos son ConfigSection, con la medida al costado', async () => {
		// Cinco `article` escritos a mano con el mismo borde. La medida tiene
		// que seguir a la vista: es lo que se mira primero.
		const wrapper = await mountResources();

		const cards = wrapper.findAllComponents(ConfigSection);
		expect(cards.length).toBe(5);
		expect(cards[0]?.props('title')).toBe('recursos.cpu');
		expect(cards[0]?.text()).toContain('12.0 %');
		expect(wrapper.findAll('article')).toHaveLength(0);
	});

	test('Aplicaciones usa la lista de la librería y el desplegable dice que se abre', async () => {
		responderInvoke('aplicaciones', APPS);
		const wrapper = await mountView(AppsView, { interval: 100000, active: true });

		const [withWindow] = wrapper.findAllComponents(ListGroup);
		expect(withWindow?.findAllComponents(ListRow).map((row) => row.text())).toEqual([
			expect.stringContaining('firefox'),
		]);
		expect(wrapper.findAll('ul')).toHaveLength(0);

		// Lo de segundo plano sigue escondido por omisión, y el botón lo dice
		// con `aria-expanded`, que el botón escrito a mano no decía.
		const disclosure = wrapper.findComponent(Disclosure);
		const toggle = disclosure.get('button');
		const region = disclosure.get(`#${toggle.attributes('aria-controls')}`);
		expect(toggle.attributes('aria-expanded')).toBe('false');
		expect(region.attributes('style') ?? '').toContain('display: none');
		expect(region.text()).toContain('pipewire');

		await toggle.trigger('click');
		expect(toggle.attributes('aria-expanded')).toBe('true');
		expect(region.attributes('style') ?? '').not.toContain('display: none');
	});

	test('cerrar es un botón de la librería, con su nombre', async () => {
		responderInvoke('aplicaciones', APPS);
		const wrapper = await mountView(AppsView, { interval: 100000, active: true });

		const close = wrapper.findAllComponents(ActionButton).find((b) => b.props('label') === 'aplicaciones.cerrar');
		expect(close?.props('variant')).toBe('secondary');
	});

	test('Servicios: casilla de la librería, insignia del sistema y botones de la librería', async () => {
		responderInvoke('lista_de_servicios', SERVICES);
		const wrapper = await mountView(ServicesView);

		// Casilla donde había casilla: es la misma decisión, con otra forma.
		const checkbox = wrapper.findComponent(Checkbox);
		expect(checkbox.props('label')).toBe('servicios.soloVasakOS');
		expect(checkbox.get('input').attributes('type')).toBe('checkbox');

		const badges = wrapper.findAllComponents(Badge);
		expect(badges.filter((b) => b.text() === 'servicios.delSistema')).toHaveLength(1);
		// El estado es una insignia con tono y no texto de color: el verde y el
		// rojo del esquema como letra sobre la superficie no llegan a 4,5:1.
		const states = Object.fromEntries(
			badges.filter((b) => b.text() !== 'servicios.delSistema').map((b) => [b.text(), b.props('tone')])
		);
		expect(states).toEqual({ active: 'success', failed: 'error' });

		// Tres acciones por servicio y el de actualizar.
		expect(wrapper.findAllComponents(ActionButton)).toHaveLength(SERVICES.length * 3 + 1);
	});

	test('Registros: casilla, botón, filas de la librería y cortes de contenedor', async () => {
		responderInvoke('desplazamiento_horario', 0);
		responderInvoke('apps_del_diario', []);
		responderInvoke('registros_de_vasakos', [
			{ microsegundos: 3_600_000_000, origen: 'vasak-desktop', nivel: 3, mensaje: 'falló algo' },
		]);
		const wrapper = await mountView(LogsView);

		expect(wrapper.findComponent(Checkbox).props('label')).toBe('registros.soloProblemas');
		const rows = wrapper.findAllComponents(ListRow);
		expect(rows).toHaveLength(1);
		expect(rows[0]?.text()).toContain('01:00:00');
		expect(rows[0]?.text()).toContain('falló algo');

		// La fila pasa a una línea por el ancho del área (`@lg`) y no por el de
		// la pantalla (`sm:`), que no descontaba la barra lateral.
		// El nivel va en el canto, con el texto en `tx-main`: el rojo como color
		// de letra sobre la superficie no llega a 4,5:1.
		const level = rows[0]?.get('[data-level]');
		expect(level?.classes()).toContain('border-status-error');
		expect(level?.find('.text-status-error').exists()).toBe(false);

		const html = rows[0]?.html() ?? '';
		expect(html).toContain('@lg:flex-row');
		expect(html).not.toMatch(/(?<![\w@-])sm:/);
	});

	test('una respuesta vieja de Registros no pisa a la nueva', async () => {
		// El selector, la casilla y el botón piden cada uno; si el primer pedido
		// contesta último, la lista quedaba con el filtro anterior (lo marcó la
		// revisión). `invoke` devuelve la promesa registrada, así que acá se
		// decide a mano en qué orden contestan.
		responderInvoke('desplazamiento_horario', 0);
		responderInvoke('apps_del_diario', []);
		let answerFirst: (value: unknown) => void = () => {};
		responderInvoke('registros_de_vasakos', new Promise((resolve) => (answerFirst = resolve)));
		const wrapper = await mountView(LogsView);

		responderInvoke('registros_de_vasakos', [
			{ microsegundos: 0, origen: 'nuevo', nivel: 6, mensaje: 'la respuesta nueva' },
		]);
		await wrapper.findComponent(Checkbox).get('input').setValue(true);
		for (let i = 0; i < 4; i++) {
			await new Promise((done) => setTimeout(done, 0));
			await nextTick();
		}
		answerFirst([{ microsegundos: 0, origen: 'viejo', nivel: 6, mensaje: 'la respuesta vieja' }]);
		for (let i = 0; i < 4; i++) {
			await new Promise((done) => setTimeout(done, 0));
			await nextTick();
		}

		expect(wrapper.text()).toContain('la respuesta nueva');
		expect(wrapper.text()).not.toContain('la respuesta vieja');
	});

	test('y cuando una app no tiene nada, lo explica con el estado vacío', async () => {
		responderInvoke('desplazamiento_horario', 0);
		responderInvoke('apps_del_diario', [{ id: 'vasak-mail', icono: 'vasak-mail', presente: false }]);
		responderInvoke('registros_de_vasakos', []);
		const wrapper = await mountView(LogsView);

		await wrapper.get('select').setValue('vasak-mail');
		for (let i = 0; i < 4; i++) {
			await new Promise((done) => setTimeout(done, 0));
			await nextTick();
		}

		const empty = wrapper.findComponent(EmptyState);
		expect(empty.exists()).toBe(true);
		expect(empty.props('title')).toBe('registros.nadaQueMostrar');
		expect(empty.props('note')).toBe('registros.dondeEscriben');
	});

	test('Limpieza: el total es un StatTile y el «listo» se anuncia', async () => {
		responderInvoke('recuperable', [
			{ tarea: 'papelera', bytes: 2048, necesita_autenticar: false },
			{ tarea: 'cache-del-kernel', bytes: null, necesita_autenticar: true },
		]);
		const wrapper = await mountView(CleanupView);

		const total = wrapper.findComponent(StatTile);
		expect(total.props('label')).toBe('limpieza.totalEtiqueta');
		expect(total.props('value')).toBe('2.0 kB');

		// La advertencia de la memoria es un aviso de la librería, no un párrafo
		// con fondo a mano.
		const alerts = wrapper.findAllComponents(AlertMessage);
		expect(alerts.some((a) => a.props('tone') === 'info' && a.text() === 'limpieza.advertenciaMemoria')).toBe(true);

		// «Limpiar todo» del disco es la acción principal: la única con el acento.
		const primary = wrapper.findAllComponents(ActionButton).filter((b) => b.props('variant') === 'primary');
		expect(primary).toHaveLength(1);
		expect(primary[0]?.props('label')).toBe('limpieza.limpiarTodo');

		responderInvoke('limpiar', null);
		await wrapper.findAllComponents(ActionButton).find((b) => b.props('label') === 'limpieza.limpiar')?.trigger('click');
		for (let i = 0; i < 4; i++) {
			await new Promise((done) => setTimeout(done, 0));
			await nextTick();
		}
		const done = wrapper.findAllComponents(AlertMessage).find((a) => a.props('tone') === 'success');
		expect(done?.text()).toBe('limpieza.hecho');
		expect(done?.attributes('role')).toBe('status');
	});

	test('las carpetas de proyectos: casillas de la librería atadas a su nombre', async () => {
		responderInvoke('proyectos_limpiables', [
			{ ruta: '/home/ana/web/node_modules', clase: 'node', proyecto: 'web', bytes: null },
		]);
		responderInvoke('medir_proyecto', 4096);
		const wrapper = await mountView(CleanableProjects);

		await wrapper.findAllComponents(ActionButton).find((b) => b.props('label') === 'limpieza.proyectos.buscar')?.trigger('click');
		for (let i = 0; i < 6; i++) {
			await new Promise((done) => setTimeout(done, 0));
			await nextTick();
		}

		const checkbox = wrapper.findComponent(Checkbox);
		expect(checkbox.props('label')).toBe('web');
		const id = checkbox.get('input').attributes('id');
		// La etiqueta de al lado —nombre, ruta y clase— apunta a la misma casilla.
		const labels = wrapper.findAll(`label[for="${id}"]`).filter((l) => !l.find('input').exists());
		expect(labels).toHaveLength(1);
		expect(labels[0]?.text()).toContain('web');
		expect(labels[0]?.text()).toContain('/home/ana/web/node_modules');

		await checkbox.get('input').setValue(true);
		const remove = wrapper.findAllComponents(ActionButton).find((b) => b.props('variant') === 'primary');
		expect(remove?.props('label')).toBe('limpieza.proyectos.borrarSeleccion');
	});
});

describe('lo que el monitor ya no dibuja', () => {
	const removed = [
		'src/components/ThemeIcon.vue',
		'src/components/BarraDeCarga.vue',
		'src/composables/useReactiveIcon.ts',
	];

	test('los tres archivos se fueron', async () => {
		for (const path of removed) {
			expect(await Bun.file(`${ROOT}${path}`).exists()).toBe(false);
		}
	});

	test('y nadie los importa', async () => {
		// Un `import` a un archivo que volvió no lo ataja ninguna prueba
		// montada: lo que fallaría es la prueba del componente que ya no está.
		const sources = [...new Bun.Glob('src/**/*.{vue,ts}').scanSync(ROOT)];
		expect(sources.length).toBeGreaterThan(10);

		const offenders: string[] = [];
		for (const path of sources) {
			const text = await Bun.file(`${ROOT}${path}`).text();
			if (/components\/ThemeIcon|BarraDeCarga|useReactiveIcon/.test(text)) offenders.push(path);
		}

		expect(offenders).toEqual([]);
	});
});

describe('el reinicio de los dobles', () => {
	test('olvidarTodo borra también las respuestas registradas', async () => {
		// Es la única que mira los dobles y no la aplicación, y está por una
		// razón: una respuesta que sobrevive al reinicio se la come el `invoke`
		// de otro archivo de prueba, que no registró ninguna y no tiene forma de
		// saber de dónde salió. Falla en silencio y en otro lado. Lo marcó la
		// revisión, sobre el reinicio que había quedado aparte.
		responderInvoke('recursos', { cpu: 1 });
		olvidarTodo();

		expect(await invoke('recursos')).toBeUndefined();
	});
});
