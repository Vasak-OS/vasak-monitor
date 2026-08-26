import { describe, expect, test } from 'bun:test';
import {
	debeMedir,
	esEnVivo,
	INTERVALO_POR_OMISION,
	INTERVALOS,
	intervaloValido,
} from '@/tools/sondeo';

describe('debeMedir', () => {
	test('con la ventana tapada no se mide', () => {
		// Es la aplicación que muestra el consumo: medir con nadie mirando gasta
		// exactamente lo que la pantalla dice que hay que cuidar.
		expect(debeMedir(true, true, false)).toBe(false);
	});

	test('en una pantalla que no cambia sola tampoco', () => {
		// Servicios, Limpieza y Registros no se mueven por su cuenta.
		expect(debeMedir(false, false, false)).toBe(false);
	});

	test('con una consulta en vuelo se espera', () => {
		// `setInterval` no espera a la anterior: si el backend tarda más que el
		// intervalo, una respuesta vieja llega después de una nueva y pisa datos
		// más recientes.
		expect(debeMedir(false, true, true)).toBe(false);
	});

	test('a la vista, en vivo y sin nada en vuelo, se mide', () => {
		expect(debeMedir(false, true, false)).toBe(true);
	});
});

describe('esEnVivo', () => {
	test('recursos y aplicaciones se actualizan solas', () => {
		expect(esEnVivo('recursos')).toBe(true);
		expect(esEnVivo('aplicaciones')).toBe(true);
	});

	test('el resto no', () => {
		for (const pantalla of ['servicios', 'limpieza', 'registros']) {
			expect(esEnVivo(pantalla)).toBe(false);
		}
	});

	test('una pantalla desconocida no se mide sola', () => {
		// Mejor no medir que medir de más: lo primero se nota y se arregla, lo
		// segundo pasa inadvertido.
		expect(esEnVivo('lo-que-sea')).toBe(false);
	});
});

describe('intervaloValido', () => {
	test('los ofrecidos se aceptan tal cual', () => {
		for (const i of INTERVALOS) expect(intervaloValido(i)).toBe(i);
	});

	test('un valor inventado cae en el de omisión', () => {
		// Un valor guardado por una versión anterior, o escrito a mano en la
		// configuración, no debe dejar el monitor midiendo cada milisegundo.
		expect(intervaloValido(1)).toBe(INTERVALO_POR_OMISION);
		expect(intervaloValido(0)).toBe(INTERVALO_POR_OMISION);
		expect(intervaloValido(-5000)).toBe(INTERVALO_POR_OMISION);
		expect(intervaloValido(Number.NaN)).toBe(INTERVALO_POR_OMISION);
		expect(intervaloValido(Number.POSITIVE_INFINITY)).toBe(INTERVALO_POR_OMISION);
	});
});
