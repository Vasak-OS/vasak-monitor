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
import { mount } from '@vue/test-utils';
import { SideBar, SideButton } from '@vasakgroup/vue-libvasak';
import { nextTick } from 'vue';
import App from '@/App.vue';
import { olvidarTodo } from './dobles';

const PANTALLAS = ['recursos', 'aplicaciones', 'servicios', 'limpieza', 'registros'];

/** Las vistas se reemplazan: lo que se mira es a cuál se llega, no qué dibuja. */
const VISTAS = ['RecursosView', 'AplicacionesView', 'ServiciosView', 'LimpiezaView', 'RegistrosView'];

async function abrirElMonitor() {
	const vista = mount(App, {
		global: { stubs: Object.fromEntries(VISTAS.map((v) => [v, { template: `<div class="${v}" />` }])) },
	});
	for (let i = 0; i < 6; i++) {
		await nextTick();
	}
	return vista;
}

/** El botón de la barra cuyo texto es esa pantalla. */
function botonDe(vista: Awaited<ReturnType<typeof abrirElMonitor>>, pantalla: string) {
	return vista
		.findAllComponents(SideButton)
		.find((boton) => boton.text().includes(`pantallas.${pantalla}`));
}

beforeEach(() => {
	olvidarTodo();
});

describe('la barra', () => {
	test('es la compartida y no una escrita acá', async () => {
		// El punto de todo el cambio: que las ventanas del escritorio se lean
		// como partes de lo mismo porque **son** lo mismo, no porque alguien se
		// acordó de copiar las clases.
		const vista = await abrirElMonitor();

		expect(vista.findComponent(SideBar).exists()).toBe(true);
		expect(vista.findAll('nav')).toHaveLength(0);
	});

	test('sin área de título, que ya está arriba', async () => {
		// El nombre de la ventana lo pone la barra superior; repetirlo acá
		// gastaría la mitad del alto de la barra.
		const vista = await abrirElMonitor();

		// Sin título ni ranura de cabecera, la barra no dibuja su `<header>`:
		// queda sólo el botón de plegar, que necesita su lugar igual.
		const barra = vista.findComponent(SideBar);
		expect(barra.find('header').exists()).toBe(false);
		expect(barra.find('button[aria-label="barraLateral.plegar"]').exists()).toBe(true);
	});

	test('están las cinco pantallas', async () => {
		const vista = await abrirElMonitor();

		const textos = vista.findAllComponents(SideButton).map((boton) => boton.text());
		for (const pantalla of PANTALLAS) {
			expect(textos.some((texto) => texto.includes(`pantallas.${pantalla}`))).toBe(true);
		}
	});
});

describe('cambiar de pantalla', () => {
	test('se abre en recursos', async () => {
		const vista = await abrirElMonitor();

		expect(vista.find('.RecursosView').exists()).toBe(true);
		expect(botonDe(vista, 'recursos')?.props('active')).toBe(true);
	});

	test('apretar una lleva a la suya', async () => {
		const vista = await abrirElMonitor();

		await botonDe(vista, 'servicios')?.trigger('click');
		await nextTick();

		expect(vista.find('.ServiciosView').exists()).toBe(true);
		expect(vista.find('.RecursosView').exists()).toBe(false);
	});

	test('y la que está se marca, para quien no ve el color', async () => {
		// Sobre el botón dibujado y no sobre su propiedad: un borde distinto no
		// lo anuncia un lector de pantalla, y comprobar `active` pasaría igual el
		// día que el botón dejara de traducirlo a `aria-current`.
		const vista = await abrirElMonitor();

		await botonDe(vista, 'limpieza')?.trigger('click');
		await nextTick();

		expect(botonDe(vista, 'limpieza')?.get('button').attributes('aria-current')).toBe('page');
		expect(botonDe(vista, 'recursos')?.get('button').attributes('aria-current')).toBeUndefined();
	});

	test('las cinco llevan a algún lado', async () => {
		// El control de los de arriba: si una quedara sin vista, la ventana se
		// vacía al apretarla y nadie lo dice.
		const vista = await abrirElMonitor();

		for (const [i, pantalla] of PANTALLAS.entries()) {
			await botonDe(vista, pantalla)?.trigger('click');
			await nextTick();
			expect(vista.find(`.${VISTAS[i]}`).exists(), pantalla).toBe(true);
		}
	});
});

describe('el intervalo de medición', () => {
	test('sigue estando, dentro de la barra', async () => {
		// Vivía al pie del `<nav>` viejo. Al cambiar la barra es lo más fácil de
		// dejarse afuera, y sin él no hay forma de bajar el muestreo.
		const vista = await abrirElMonitor();

		const barra = vista.findComponent(SideBar);
		const select = barra.find('select');
		expect(select.exists()).toBe(true);

		// Y su nombre queda atado al control. La etiqueta la pone el propio
		// selector: suelta al lado no estaba asociada a nada, así que un lector
		// de pantalla anunciaba un desplegable sin nombre y hacer clic en el
		// texto no abría la lista.
		const etiqueta = barra.findAll('label').find((l) => l.text() === 'ajustes.intervalo');
		expect(etiqueta).toBeDefined();
		expect(etiqueta?.attributes('for')).toBe(select.attributes('id'));
	});

	test('y dice que la medición se pausa con la ventana tapada', async () => {
		// Sin eso, alguien que abre el monitor y lo deja de fondo supone que
		// sigue gastando.
		const vista = await abrirElMonitor();

		expect(vista.findComponent(SideBar).text()).toContain('ajustes.pausaExplicada');
	});
});

describe('la versión de la librería', () => {
	test('trae el arreglo de la barra que abría plegada', async () => {
		// En WebKitGTK el `change` de `matchMedia` no llega cuando la ventana
		// pasa de angosta a ancha al terminar de abrirse: la barra se montaba
		// con el WebView todavía sin tamaño y se quedaba plegada para siempre
		// en una ventana de 1280 que nadie había plegado. Se arregló en la
		// 0.3.5 de la librería, así que volver atrás de ahí lo trae de vuelta.
		const manifiesto = (await Bun.file(
			new URL('../package.json', import.meta.url)
		).json()) as { dependencies: Record<string, string> };
		const pedido = manifiesto.dependencies['@vasakgroup/vue-libvasak'];
		expect(pedido).toBeDefined();

		const [mayor, menor, parche] = pedido
			.replace(/^[^\d]*/, '')
			.split('.')
			.map(Number);
		const numero = mayor * 1_000_000 + menor * 1_000 + parche;
		expect(numero).toBeGreaterThanOrEqual(0 * 1_000_000 + 3 * 1_000 + 5);
	});
});
