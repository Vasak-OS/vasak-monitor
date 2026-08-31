/**
 * El búfer de muestras y las cuentas del gráfico.
 *
 * Lo que se prueba es lo que dibuja algo verosímil pero equivocado, que es peor
 * que no dibujar nada: una serie que se escala mal muestra una CPU tranquila
 * cuando está al palo, y nadie lo nota porque el gráfico se ve bien.
 */

import { describe, expect, test } from 'bun:test';
import {
	agregar,
	CAPACIDAD,
	comoArea,
	comoPath,
	maximoDe,
	tramosDe,
} from '../src/tools/historial';

describe('el búfer', () => {
	test('las muestras se van sumando al final', () => {
		let s: (number | null)[] = [];
		s = agregar(s, 10);
		s = agregar(s, 20);
		expect(s).toEqual([10, 20]);
	});

	test('al llenarse se descarta la más vieja', () => {
		let s: (number | null)[] = [];
		for (let i = 0; i < 5; i++) s = agregar(s, i, 3);
		// Quedan las tres últimas, en orden.
		expect(s).toEqual([2, 3, 4]);
	});

	test('nunca pasa de la capacidad', () => {
		let s: (number | null)[] = [];
		for (let i = 0; i < CAPACIDAD * 2; i++) s = agregar(s, i);
		expect(s.length).toBe(CAPACIDAD);
	});

	test('devuelve un arreglo nuevo, no muta el anterior', () => {
		// De esto depende que Vue vea el cambio sin disparar la reactividad a mano.
		const original: (number | null)[] = [1, 2];
		const nuevo = agregar(original, 3);
		expect(original).toEqual([1, 2]);
		expect(nuevo).not.toBe(original);
	});

	test('un hueco de medición se guarda como hueco', () => {
		// Descartarlo juntaría los dos lados con una recta que dice que todo
		// anduvo bien justo cuando no se pudo medir.
		let s: (number | null)[] = [];
		s = agregar(s, 10);
		s = agregar(s, null);
		s = agregar(s, 30);
		expect(s).toEqual([10, null, 30]);
	});
});

describe('el máximo', () => {
	test('sale del mayor valor medido', () => {
		expect(maximoDe([3, 9, 4])).toBe(9);
	});

	test('los huecos no cuentan', () => {
		expect(maximoDe([3, null, 4])).toBe(4);
	});

	test('sin ningún valor medido no hay máximo', () => {
		// Devolver 0 haría que quien escala dividiera por cero.
		expect(maximoDe([])).toBeNull();
		expect(maximoDe([null, null])).toBeNull();
	});
});

describe('los tramos', () => {
	test('una serie sin huecos es un solo tramo', () => {
		const t = tramosDe([0, 50, 100], 100, 200, 40);
		expect(t.length).toBe(1);
		expect(t[0].length).toBe(3);
	});

	test('el valor más alto queda arriba y el más bajo abajo', () => {
		// Invertir el eje es el error clásico: el gráfico se ve bien y dice lo
		// contrario de lo que pasa.
		const [tramo] = tramosDe([0, 100], 100, 200, 40);
		expect(tramo[0].y).toBe(40); // 0% → base
		expect(tramo[1].y).toBe(0); // 100% → techo
	});

	test('los puntos se reparten a lo ancho', () => {
		const [tramo] = tramosDe([1, 2, 3], 10, 200, 40);
		expect(tramo[0].x).toBe(0);
		expect(tramo[2].x).toBe(200);
	});

	test('un hueco parte la serie en dos tramos', () => {
		const t = tramosDe([10, null, 30], 100, 200, 40);
		expect(t.length).toBe(2);
		expect(t[0].length).toBe(1);
		expect(t[1].length).toBe(1);
	});

	test('un valor por encima del techo se acota', () => {
		// Sin acotar, el punto se dibuja fuera del viewBox y la línea se corta
		// sola sin que se entienda por qué.
		const [tramo] = tramosDe([150, 150], 100, 200, 40);
		expect(tramo[0].y).toBe(0);
		expect(tramo[1].y).toBe(0);
	});

	test('un techo de cero no divide por cero', () => {
		// Pasa de verdad: la red arranca en cero bytes y el máximo es cero hasta
		// la primera transferencia. Sin este corte, `y` sale NaN y el path queda
		// vacío sin que nada avise.
		expect(tramosDe([0, 0], 0, 200, 40)).toEqual([]);
	});

	test('con menos de dos muestras todavía no hay gráfico', () => {
		expect(tramosDe([], 100, 200, 40)).toEqual([]);
		expect(tramosDe([50], 100, 200, 40)).toEqual([]);
	});
});

describe('los caminos SVG', () => {
	test('el primer punto es un salto y el resto líneas', () => {
		const d = comoPath([
			{ x: 0, y: 10 },
			{ x: 5, y: 20 },
		]);
		expect(d).toBe('M0.00,10.00 L5.00,20.00');
	});

	test('el área cierra contra la base', () => {
		const d = comoArea(
			[
				{ x: 0, y: 10 },
				{ x: 5, y: 20 },
			],
			40
		);
		// Baja hasta la base en el último x, vuelve por la base y cierra.
		expect(d.endsWith('L5.00,40 L0.00,40 Z')).toBe(true);
	});

	test('un punto solo no dibuja área', () => {
		// Cerrar un único punto contra la base pinta una tira de ancho cero que
		// en algunos motores se ve como una raya vertical.
		expect(comoArea([{ x: 0, y: 10 }], 40)).toBe('');
	});

	test('sin puntos no hay camino', () => {
		expect(comoPath([])).toBe('');
		expect(comoArea([], 40)).toBe('');
	});
});
