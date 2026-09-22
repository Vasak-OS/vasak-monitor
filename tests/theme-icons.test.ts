/**
 * Los iconos del monitor siguen al tema, y la recarga va por el planificador.
 *
 * El monitor no resuelve ningún icono: los pide por nombre a `ThemeIcon`, y los
 * cinco de la barra lateral van por el `icon` de `SideButton`, que hace lo
 * mismo por dentro. Lo que se comprueba acá es que la recarga llegue por el
 * planificador de la librería y no en el acto.
 *
 * Importa porque hasta este cambio no era así, y nada lo decía: el manifiesto
 * pedía `^1.0.0`, que admite la 1.4.0, pero `bun.lock` había quedado en la
 * 1.0.0 y se empaquetaba ésa. Un rango corregido no mueve el candado, así que
 * el monitor venía con la librería de antes del planificador sin que ninguna
 * prueba ni el CI dijeran nada — y es el repositorio del taller con más iconos
 * a la vez de los que estaban atrás.
 */

import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { olvidarLosIconosDelTema } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { nextTick } from 'vue';
import App from '@/App.vue';
import { emit, olvidarTodo, ponerEnElTema } from './dobles';

/** Deja que terminen las promesas encadenadas del pedido del icono. */
async function settle(rounds = 8) {
	for (let i = 0; i < rounds; i++) {
		await nextTick();
		await new Promise((done) => setTimeout(done, 0));
	}
}

/**
 * Lo mismo, esperando además a que el planificador recargue.
 *
 * Desde la 1.3.0 el cambio de tema no vuelve a pedir en el acto: vacía la
 * memoria y **agenda** la recarga, para que el anuncio del tema de iconos y el
 * de GTK no disparen dos barridos.
 */
async function settleWithReload() {
	await new Promise((done) => setTimeout(done, 150));
	await settle();
}

/** El icono de la primera pantalla, que es la que abre el monitor. */
const RESOURCES = 'utilities-system-monitor';

let mounted: VueWrapper | null = null;

function openMonitor() {
	mounted = mount(App);
	return mounted;
}

function iconSources(vista: VueWrapper): string[] {
	return vista.findAll('img').map((una) => una.attributes('src') ?? '');
}

beforeEach(() => {
	olvidarTodo();
	// La memoria de la librería vive en su módulo y sobrevive entre archivos de
	// prueba: sin vaciarla, esto ve el icono que dejó otra.
	olvidarLosIconosDelTema();
});

afterEach(() => {
	mounted?.unmount();
	mounted = null;
	olvidarLosIconosDelTema();
});

describe('el monitor dibuja sus iconos con el tema', () => {
	test('la barra lateral los pide por nombre', async () => {
		ponerEnElTema(RESOURCES, 'data:image/svg+xml,recursos-claro');

		const vista = openMonitor();
		await settle();

		expect(iconSources(vista)).toContain('data:image/svg+xml,recursos-claro');
	});

	test('la recarga se agenda, no pasa en el acto', async () => {
		// Es lo que separa la 1.0.0 —la que se venía empaquetando— de la 1.4.0:
		// sin planificador el dibujo nuevo ya estaría acá. Con él, todavía no, y
		// por eso una ráfaga de anuncios —el tema de iconos y el de GTK llegan
		// juntos— no dispara dos barridos. Con veintiséis iconos a la vez, ese
		// barrido de más es la diferencia que se ve.
		ponerEnElTema(RESOURCES, 'data:image/svg+xml,recursos-claro');

		const vista = openMonitor();
		await settle();

		ponerEnElTema(RESOURCES, 'data:image/svg+xml,recursos-oscuro');
		await emit('vicons:theme-changed');
		await settle();

		expect(iconSources(vista)).not.toContain('data:image/svg+xml,recursos-oscuro');

		await settleWithReload();
		expect(iconSources(vista)).toContain('data:image/svg+xml,recursos-oscuro');
	});
});
