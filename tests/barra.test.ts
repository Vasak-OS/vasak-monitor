/**
 * La barra de la ventana del monitor.
 *
 * Tenía su propio marco y su propia barra, con el título centrado a fuerza de
 * un tercer hijo vacío que empujaba contra el `justify-between`. Los dos salen
 * ahora de la librería, y el título va en la ranura `centro`.
 *
 * Lo que se comprueba es dónde queda cada cosa, que es lo que se rompe al
 * mudarla. La cadena es larga —el layout se lo pasa al marco, el marco a la
 * barra— y basta con que uno de los dos no reexponga una ranura para que lo que
 * se le ponga desaparezca sin ningún error.
 */

import { afterEach, describe, expect, test } from 'bun:test';
import { AppBar, WindowControls, WindowFrame } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { h } from 'vue';
import ThemeIcon from '@/components/ThemeIcon.vue';
import WindowAppLayout from '@/layouts/WindowAppLayout.vue';

let vista: VueWrapper | null = null;

function abrir(contenido?: () => unknown) {
	vista = mount(WindowAppLayout, contenido ? { slots: { default: contenido } } : {});
	return vista;
}

/** Lo que se dibuja dentro de una ranura de la barra. */
function ranura(ventana: VueWrapper, nombre: string) {
	const barra = ventana.findComponent(AppBar);
	const dibujar = (barra.vm.$slots as Record<string, (() => unknown) | undefined>)[nombre];
	return dibujar ? mount({ render: () => dibujar() }) : null;
}

afterEach(() => {
	vista?.unmount();
	vista = null;
});

describe('la ventana', () => {
	test('usa el marco compartido', () => {
		expect(abrir().findComponent(WindowFrame).exists()).toBe(true);
	});

	test('y no queda un segundo borde dibujado a mano', () => {
		expect(abrir().findAll('.rounded-corner-window').length).toBe(1);
	});

	test('con los tres botones y su nombre traducido', () => {
		const botones = abrir()
			.findComponent(WindowControls)
			.findAll('button')
			.map((boton) => boton.attributes('aria-label'));

		expect(botones).toEqual(['ventana.minimizar', 'ventana.maximizar', 'ventana.cerrar']);
	});
});

describe('lo que va en la barra', () => {
	test('el icono va en `identidad`', () => {
		// En el contenido de la barra se desplazaría con lo demás cuando queda a
		// un costado: `identidad` es la única zona que no scrollea.
		const dentro = ranura(abrir(), 'identidad');

		expect(dentro?.findComponent(ThemeIcon).props('nombre')).toBe('utilities-system-monitor');
	});

	test('el título va en `centro`, al medio de la ventana entera', () => {
		// Antes lo centraba un tercer hijo vacío tirando contra el
		// `justify-between`, y eso lo deja centrado respecto de lo que sobra
		// entre el icono y los controles: los tres botones ocupan bastante más
		// que el icono, así que se corría.
		const dentro = ranura(abrir(), 'centro');

		expect(dentro?.text()).toBe('app.titulo');
	});

	test('y el molde viejo no dejó nada en el contenido de la barra', () => {
		// El icono, el título y un `<div></div>` vacío eran los tres hijos que
		// el `justify-between` necesitaba para centrar. Los dos primeros tienen
		// su ranura ahora, y el tercero no tiene por qué existir: si alguno
		// sobreviviera quedaría suelto en el contenido de la barra, que es la
		// única zona que crece, empujando al resto.
		// El marco siempre le pasa a la barra su ranura por omisión —`barra` de
		// afuera—, así que lo que se mira es lo que dibuja, no si existe.
		const dentro = ranura(abrir(), 'default');

		expect(dentro?.text()).toBe('');
	});
});

describe('el contenido', () => {
	test('lo que va dentro del layout se dibuja', () => {
		// Sin la ranura, `<WindowAppLayout>…</WindowAppLayout>` descarta en
		// silencio lo que se le ponga dentro y la ventana abre vacía. Pasó, y
		// costó una compilación y una captura darse cuenta.
		const ventana = abrir(() => h('p', { class: 'lo-mio' }, 'datos'));

		expect(ventana.find('.lo-mio').exists()).toBe(true);
	});

	test('y ocupa el ancho entero', () => {
		// El contenedor del marco es una fila, así que un hijo sin crecimiento
		// se encoge al ancho mínimo de su contenido; con `truncate` y `min-w-0`
		// adentro eso es casi cero, y los datos quedaban en una columna de un
		// píxel con barra de desplazamiento.
		const ventana = abrir(() => h('p', { class: 'lo-mio' }, 'datos'));
		const caja = ventana.find('.lo-mio').element.parentElement as HTMLElement;

		expect(caja.className).toContain('w-full');
		expect(caja.className).toContain('flex-1');
	});
});
