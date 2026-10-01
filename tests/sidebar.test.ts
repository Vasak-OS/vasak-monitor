/**
 * La barra lateral del monitor, montada.
 *
 * Era un `<nav>` escrito a mano adentro de `App.vue` que se parecía a la de
 * Configuración sin serlo: `w-14` que crecía a `sm:w-52` según el ancho de la
 * ventana, sin forma de plegarla a voluntad y sin los grupos. Ahora es la de
 * `@vasakgroup/vue-libvasak`, que es la misma que usan las demás ventanas del
 * escritorio.
 *
 * Lo que se comprueba acá es lo del monitor —que las cinco pantallas estén y
 * cambien, y que el intervalo siga a mano—; cómo se pliega y cómo se ve es de
 * la librería y se prueba allá.
 */

import { beforeEach, describe, expect, test } from 'bun:test';
import { mount, type VueWrapper } from '@vue/test-utils';
import { ListRow, SideBar, SideButton } from '@vasakgroup/vue-libvasak';
import { nextTick } from 'vue';
import App from '@/App.vue';
import { olvidarTodo } from './dobles';

const SCREENS = ['recursos', 'aplicaciones', 'servicios', 'limpieza', 'registros'];

/** Las vistas se reemplazan: lo que se mira es a cuál se llega, no qué dibuja. */
const VIEWS = ['ResourcesView', 'AppsView', 'ServicesView', 'CleanupView', 'LogsView'];

async function openMonitor() {
	const wrapper = mount(App, {
		global: { stubs: Object.fromEntries(VIEWS.map((v) => [v, { template: `<div class="${v}" />` }])) },
	});
	for (let i = 0; i < 6; i++) {
		await nextTick();
	}
	return wrapper;
}

/** El botón de la barra cuyo texto es esa pantalla. */
function buttonFor(wrapper: Awaited<ReturnType<typeof openMonitor>>, screen: string) {
	return wrapper
		.findAllComponents(SideButton)
		.find((button) => button.text().includes(`pantallas.${screen}`));
}

beforeEach(() => {
	olvidarTodo();
});

describe('la barra', () => {
	test('es la compartida y no una escrita acá', async () => {
		// El punto de todo el cambio: que las ventanas del escritorio se lean
		// como partes de lo mismo porque **son** lo mismo, no porque alguien se
		// acordó de copiar las clases.
		const wrapper = await openMonitor();

		expect(wrapper.findComponent(SideBar).exists()).toBe(true);
		// El único `<nav>` es la lista de la ventana angosta, que también es de
		// la librería: filas de `ListRow`, no botones escritos acá.
		const navs = wrapper.findAll('nav');
		expect(navs).toHaveLength(1);
		expect(navs[0]?.attributes('data-narrow-list')).toBeDefined();
		expect(navs[0]?.findAll('button')).toHaveLength(0);
	});

	test('sin área de título, que ya está arriba', async () => {
		// El nombre de la ventana lo pone la barra superior; repetirlo acá
		// gastaría la mitad del alto de la sidebar.
		const wrapper = await openMonitor();

		// Sin título ni ranura de cabecera, la barra no dibuja su `<header>`:
		// queda sólo el botón de plegar, que necesita su lugar igual.
		const sidebar = wrapper.findComponent(SideBar);
		expect(sidebar.find('header').exists()).toBe(false);
		expect(sidebar.find('button[aria-label="barraLateral.plegar"]').exists()).toBe(true);
	});

	test('están las cinco pantallas', async () => {
		const wrapper = await openMonitor();

		const texts = wrapper.findAllComponents(SideButton).map((button) => button.text());
		for (const screen of SCREENS) {
			expect(texts.some((text) => text.includes(`pantallas.${screen}`))).toBe(true);
		}
	});
});

describe('cambiar de pantalla', () => {
	test('se abre en recursos', async () => {
		const wrapper = await openMonitor();

		expect(wrapper.find('.ResourcesView').exists()).toBe(true);
		expect(buttonFor(wrapper, 'recursos')?.props('active')).toBe(true);
	});

	test('apretar una lleva a la suya', async () => {
		const wrapper = await openMonitor();

		await buttonFor(wrapper, 'servicios')?.trigger('click');
		await nextTick();

		expect(wrapper.find('.ServicesView').exists()).toBe(true);
		expect(wrapper.find('.ResourcesView').exists()).toBe(false);
	});

	test('y la que está se marca, para quien no ve el color', async () => {
		// Sobre el botón dibujado y no sobre su propiedad: un borde distinto no
		// lo anuncia un lector de pantalla, y comprobar `active` pasaría igual el
		// día que el botón dejara de traducirlo a `aria-current`.
		const wrapper = await openMonitor();

		await buttonFor(wrapper, 'limpieza')?.trigger('click');
		await nextTick();

		expect(buttonFor(wrapper, 'limpieza')?.get('button').attributes('aria-current')).toBe('page');
		expect(buttonFor(wrapper, 'recursos')?.get('button').attributes('aria-current')).toBeUndefined();
	});

	test('las cinco llevan a algún lado', async () => {
		// El control de los de arriba: si una quedara sin vista, la ventana se
		// vacía al apretarla y nadie lo dice.
		const wrapper = await openMonitor();

		for (const [i, screen] of SCREENS.entries()) {
			await buttonFor(wrapper, screen)?.trigger('click');
			await nextTick();
			expect(wrapper.find(`.${VIEWS[i]}`).exists(), screen).toBe(true);
		}
	});
});

describe('el intervalo de medición', () => {
	test('sigue estando, dentro de la barra', async () => {
		// Vivía al pie del `<nav>` viejo. Al cambiar la barra es lo más fácil de
		// dejarse afuera, y sin él no hay forma de bajar el muestreo.
		const wrapper = await openMonitor();

		const sidebar = wrapper.findComponent(SideBar);
		const select = sidebar.find('select');
		expect(select.exists()).toBe(true);

		// Y su nombre queda atado al control. La etiqueta la pone el propio
		// selector: suelta al lado no estaba asociada a nada, así que un lector
		// de pantalla anunciaba un desplegable sin nombre y hacer clic en el
		// texto no abría la lista.
		const label = sidebar.findAll('label').find((l) => l.text() === 'ajustes.intervalo');
		expect(label).toBeDefined();
		expect(label?.attributes('for')).toBe(select.attributes('id'));
	});

	test('y dice que la medición se pausa con la ventana tapada', async () => {
		// Sin eso, alguien que abre el monitor y lo deja de fondo supone que
		// sigue gastando.
		const wrapper = await openMonitor();

		expect(wrapper.findComponent(SideBar).text()).toContain('ajustes.pausaExplicada');
	});
});

describe('la versión de la librería', () => {
	test('trae el arreglo de la barra que abría plegada', async () => {
		// En WebKitGTK el `change` de `matchMedia` no llega cuando la ventana
		// pasa de angosta a ancha al terminar de abrirse: la barra se montaba
		// con el WebView todavía sin tamaño y se quedaba plegada para siempre
		// en una ventana de 1280 que nadie había plegado. Se arregló en la
		// 0.3.5 de la librería —y del todo en la 0.3.6, porque el `resize` de la
		// ventana tampoco llega y hizo falta un `ResizeObserver`—, así que
		// volver atrás de ahí lo trae de vuelta.
		const manifest = (await Bun.file(
			new URL('../package.json', import.meta.url)
		).json()) as { dependencies: Record<string, string> };
		const requested = manifest.dependencies['@vasakgroup/vue-libvasak'];
		expect(requested).toBeDefined();

		const [major, minor, patch] = requested
			.replace(/^[^\d]*/, '')
			.split('.')
			.map(Number);
		const number = major * 1_000_000 + minor * 1_000 + patch;
		expect(number).toBeGreaterThanOrEqual(0 * 1_000_000 + 3 * 1_000 + 6);
	});
});

describe('la ventana angosta: una columna por vez', () => {
	/**
	 * Por debajo de `30rem` de la fila de la ventana se ve la lista **o** la
	 * pantalla, como en una aplicación de teléfono. Qué se ve lo decide una
	 * consulta de contenedor que `happy-dom` no evalúa, así que lo que se
	 * comprueba es lo que la decide: qué está montado y con qué clases. Las
	 * capturas del banco a 240 y 360 muestran el resultado.
	 */
	const narrowList = (wrapper: Awaited<ReturnType<typeof openMonitor>>) => wrapper.find('[data-narrow-list]');
	const screenPane = (wrapper: Awaited<ReturnType<typeof openMonitor>>) => wrapper.get('[data-screen]');

	test('las dos columnas se separan por el ancho de la fila, no por el de la pantalla', async () => {
		const wrapper = await openMonitor();

		expect(wrapper.get('[data-window-row]').classes()).toContain('@container/window');
		expect(wrapper.get('[data-sidebar]').classes()).toEqual(expect.arrayContaining(['hidden', '@[30rem]/window:flex']));
		expect(narrowList(wrapper).classes()).toContain('@[30rem]/window:hidden');
	});

	test('arranca en la lista, con las cinco pantallas y el intervalo', async () => {
		// El intervalo no puede desaparecer por achicar la ventana: en la barra
		// está al pie, y acá va debajo de la lista.
		const wrapper = await openMonitor();

		const rows = narrowList(wrapper).findAllComponents(ListRow);
		expect(rows.map((row: VueWrapper<any>) => row.props('title'))).toEqual(SCREENS.map((s) => `pantallas.${s}`));
		expect(rows.every((row: VueWrapper<any>) => row.props('role') === 'button')).toBe(true);
		expect(narrowList(wrapper).find('select').exists()).toBe(true);
		expect(narrowList(wrapper).text()).toContain('ajustes.pausaExplicada');

		// La pantalla queda escondida en angosto, pero no en ancho.
		expect(screenPane(wrapper).classes()).toEqual(expect.arrayContaining(['hidden', '@[30rem]/window:block']));
	});

	test('elegir una lleva a la pantalla, y volver regresa a la lista', async () => {
		const wrapper = await openMonitor();

		const services = narrowList(wrapper)
			.findAllComponents(ListRow)
			.find((row: VueWrapper<any>) => row.props('title') === 'pantallas.servicios');
		await services?.trigger('click');
		await nextTick();

		expect(narrowList(wrapper).exists()).toBe(false);
		expect(screenPane(wrapper).classes()).not.toContain('hidden');
		expect(wrapper.find('.ServicesView').exists()).toBe(true);

		// El botón para volver sólo se ve en angosto.
		const back = screenPane(wrapper).get('button');
		expect(back.text()).toBe('navigation.back');
		expect(back.classes()).toContain('@[30rem]/window:hidden');

		await back.trigger('click');
		await nextTick();
		expect(narrowList(wrapper).exists()).toBe(true);
		// Y la pantalla elegida queda marcada en la lista.
		const selected = narrowList(wrapper)
			.findAllComponents(ListRow)
			.filter((row: VueWrapper<any>) => row.props('selected'));
		expect(selected.map((row: VueWrapper<any>) => row.props('title'))).toEqual(['pantallas.servicios']);
	});

	test('el relleno de la pantalla sigue a la fila de la ventana, no a la pantalla', async () => {
		// Era `sm:p-4`. `@2xl` de la fila (42rem) da lo mismo en los anchos que se
		// miran: 12 px a 600 de ventana y 16 a 1200.
		const wrapper = await openMonitor();

		expect(screenPane(wrapper).classes()).toEqual(expect.arrayContaining(['p-3', '@2xl/window:p-4']));
	});
});
