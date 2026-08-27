import { describe, expect, test } from 'bun:test';
import { readdirSync, statSync } from 'node:fs';
import { basename, join } from 'node:path';
import { fileURLToPath } from 'node:url';

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

// `fileURLToPath` y no `.pathname`: el segundo devuelve la ruta tal como va en
// una URL, así que un directorio con un espacio o un acento en el nombre llega
// con `%20` y `readdirSync` no encuentra nada.
const RAIZ = fileURLToPath(new URL('../src', import.meta.url));
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
			encontrados.push(ruta);
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
		const deUnaPalabra = componentes(RAIZ)
			.filter((ruta) => palabras(basename(ruta)) < 2)
			.map((ruta) => ruta.slice(RAIZ.length + 1));

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
