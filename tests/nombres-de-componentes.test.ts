import { describe, expect, test } from 'bun:test';
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Los nombres de un componente de una sola palabra frenan el empaquetado.
 *
 * Biome 2.5.10 sumó `useVueMultiWordComponentNames` a `recommended`, y
 * `check-all.sh` trata cualquier aviso de biome como «este paquete no compila»:
 * `Icono.vue` y `Selector.vue` dejaron a vasak-monitor afuera del repositorio
 * sin que nada fallara al desarrollar. Esto lo hace fallar acá, que es donde se
 * mira, y no doce minutos después en el build de un paquete.
 *
 * `App.vue` es la excepción que hace la propia regla: es la raíz, no se usa como
 * etiqueta y no puede colisionar con un elemento HTML.
 */

const RAIZ = new URL('../src', import.meta.url).pathname;
const EXCEPCIONES = new Set(['App.vue']);

function componentes(dir: string): string[] {
	const encontrados: string[] = [];
	for (const entrada of readdirSync(dir)) {
		const ruta = join(dir, entrada);
		if (statSync(ruta).isDirectory()) {
			encontrados.push(...componentes(ruta));
			continue;
		}
		if (entrada.endsWith('.vue') && !EXCEPCIONES.has(entrada)) {
			encontrados.push(ruta.slice(RAIZ.length + 1));
		}
	}
	return encontrados;
}

/** `ThemeIcon` → 2, `Icono` → 1. Un nombre en minúscula cuenta como una. */
function palabras(nombreDeArchivo: string): number {
	const base = nombreDeArchivo.replace(/\.vue$/, '');
	const partes = base.match(/[A-Z]+(?![a-z])|[A-Z][a-z]*|[a-z]+|\d+/g);
	return partes?.length ?? 0;
}

describe('nombres de componentes', () => {
	test('ninguno tiene una sola palabra', () => {
		const deUnaPalabra = componentes(RAIZ).filter((ruta) => {
			const nombre = ruta.split('/').pop() ?? ruta;
			return palabras(nombre) < 2;
		});

		expect(deUnaPalabra).toEqual([]);
	});

	test('la cuenta de palabras distingue los casos que importan', () => {
		expect(palabras('ThemeIcon.vue')).toBe(2);
		expect(palabras('SelectField.vue')).toBe(2);
		expect(palabras('BarraDeCarga.vue')).toBe(3);
		expect(palabras('Icono.vue')).toBe(1);
		expect(palabras('Selector.vue')).toBe(1);
		// Las siglas no se parten en una letra por palabra.
		expect(palabras('CPUChart.vue')).toBe(2);
	});
});
