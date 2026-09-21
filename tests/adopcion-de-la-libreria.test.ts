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
import { AlertMessage, LoadingState, olvidarLosIconosDelTema, ProgressBar } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import AplicacionesView from '@/views/AplicacionesView.vue';
import RecursosView from '@/views/RecursosView.vue';
import { olvidarLasRespuestas, responderInvoke } from './dobles';

const RAIZ = new URL('..', import.meta.url).pathname;

const vistas = new Set<VueWrapper>();

function anotar<T extends VueWrapper>(vista: T): T {
	vistas.add(vista);
	return vista;
}

/**
 * El primer montaje del archivo, fuera de toda prueba.
 *
 * Bun compila cada `.vue` al importarlo, y ese costo lo pagaba entero la
 * primera prueba que montara: se pasaba del límite de cinco segundos y fallaba
 * por tiempo sin tener nada roto.
 */
beforeAll(async () => {
	const calentar = mount(RecursosView, { props: { intervalo: 100000, activa: false } });
	calentar.unmount();
}, 60_000);

afterEach(() => {
	for (const vista of vistas) vista.unmount();
	vistas.clear();
	olvidarLasRespuestas();
	olvidarLosIconosDelTema();
});

/** Un equipo de mentira, con el disco casi lleno para ver el tono. */
const RECURSOS = {
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

async function recursos(datos: unknown = RECURSOS) {
	responderInvoke('recursos', datos);
	const vista = anotar(mount(RecursosView, { props: { intervalo: 100000, activa: true } }));
	// `useSondeo` mide desde `onMounted` y la medición es asíncrona.
	await new Promise((listo) => setTimeout(listo, 0));
	await nextTick();
	return vista;
}

describe('las barras de carga', () => {
	test('dicen cuánto y de qué, que antes eran una caja de colores', async () => {
		// Sin `role="progressbar"` y sin nombre, el uso de CPU es un rectángulo
		// que cambia de ancho: para quien no lo ve, nada.
		const vista = await recursos();

		const barras = vista.findAllComponents(ProgressBar);
		expect(barras.length).toBeGreaterThan(2);

		const cpu = barras[0];
		expect(cpu.attributes('role')).toBe('progressbar');
		expect(cpu.attributes('aria-label')).toBe('recursos.cpu');
		expect(cpu.attributes('aria-valuenow')).toBe('12');
	});

	test('y el disco casi lleno se pinta distinto, que es para lo que sirve un monitor', async () => {
		// El disco de la muestra está al 95 %. Es lo único que la copia propia
		// tenía y la de la librería no, así que si el tono no llegara el color
		// se perdería sin que nada falle.
		const vista = await recursos();

		const disco = vista.findAllComponents(ProgressBar).at(-1);
		expect(disco?.props('tone')).toBe('critical');
		expect(disco?.attributes('aria-label')).toBe('/');
	});

	test('mientras no midió, lo dice en voz alta', async () => {
		// Antes era un párrafo gris sin rol: la ventana quedaba en blanco y sin
		// explicación hasta que el backend contestara.
		const vista = anotar(mount(RecursosView, { props: { intervalo: 100000, activa: false } }));
		await nextTick();

		const carga = vista.findComponent(LoadingState);
		expect(carga.exists()).toBe(true);
		expect(carga.attributes('role')).toBe('status');
	});

	test('y lo que falla interrumpe en vez de esperar turno', async () => {
		const vista = await recursos(new Error('no se pudo medir'));

		const aviso = vista.findComponent(AlertMessage);
		expect(aviso.exists()).toBe(true);
		expect(aviso.attributes('role')).toBe('alert');
		expect(vista.text()).toContain('no se pudo medir');
	});
});

describe('los buscadores', () => {
	test('tienen nombre, y no sólo un texto de relleno que se va al escribir', async () => {
		// El `placeholder` no es un nombre: desaparece con la primera letra, y
		// un lector de pantalla no tiene obligación de leerlo.
		responderInvoke('aplicaciones', []);
		const vista = anotar(mount(AplicacionesView, { props: { intervalo: 100000, activa: true } }));
		await new Promise((listo) => setTimeout(listo, 0));
		await nextTick();

		const campo = vista.find('input[type="search"]');
		expect(campo.exists()).toBe(true);
		expect(campo.attributes('aria-label')).toBe('aplicaciones.buscar');
	});
});

describe('lo que el monitor ya no dibuja', () => {
	const borrados = [
		'src/components/ThemeIcon.vue',
		'src/components/BarraDeCarga.vue',
		'src/composables/useReactiveIcon.ts',
	];

	test('los tres archivos se fueron', async () => {
		for (const ruta of borrados) {
			expect(await Bun.file(`${RAIZ}${ruta}`).exists()).toBe(false);
		}
	});

	test('y nadie los importa', async () => {
		// Un `import` a un archivo que volvió no lo ataja ninguna prueba
		// montada: lo que fallaría es la prueba del componente que ya no está.
		const fuentes = [...new Bun.Glob('src/**/*.{vue,ts}').scanSync(RAIZ)];
		expect(fuentes.length).toBeGreaterThan(10);

		const culpables: string[] = [];
		for (const ruta of fuentes) {
			const texto = await Bun.file(`${RAIZ}${ruta}`).text();
			if (/components\/ThemeIcon|BarraDeCarga|useReactiveIcon/.test(texto)) culpables.push(ruta);
		}

		expect(culpables).toEqual([]);
	});
});
