import { describe, expect, test } from 'bun:test';
import { caudal, porcentaje, tamano, tonoDeCarga } from '@/tools/formato';

describe('tamano', () => {
	test('se elige la unidad que se lee de un vistazo', () => {
		// «1438291 B» en una tabla de discos es peor que no mostrar nada.
		expect(tamano(0)).toBe('0 B');
		expect(tamano(512)).toBe('512 B');
		expect(tamano(1500)).toBe('1.5 kB');
		expect(tamano(11_811_160_064)).toBe('11.8 GB');
	});

	test('los bytes van sin decimales', () => {
		// «512,0 B» no aporta nada.
		expect(tamano(512)).not.toContain('.');
	});

	test('no se pasa de la unidad más grande', () => {
		expect(tamano(9e18)).toContain('TB');
	});

	test('un valor negativo no muestra un tamaño negativo', () => {
		// Puede llegar de una resta protegida mal en otra capa.
		expect(tamano(-100)).toBe('0 B');
	});
});

describe('porcentaje y caudal', () => {
	test('sin dato se muestra un guion, no un cero', () => {
		// La CPU y la red sólo existen como diferencia: mostrar 0% en la primera
		// muestra dibuja una caída a cero que no ocurrió.
		expect(porcentaje(null)).toBe('—');
		expect(caudal(null)).toBe('—');
	});

	test('con dato se muestra con una decimal', () => {
		expect(porcentaje(43.75)).toBe('43.8 %');
		expect(caudal(1_500_000)).toBe('1.5 MB/s');
	});
});

describe('tonoDeCarga', () => {
	test('los cortes son 75 y 90', () => {
		expect(tonoDeCarga(0)).toBe('normal');
		expect(tonoDeCarga(74.9)).toBe('normal');
		expect(tonoDeCarga(75)).toBe('atencion');
		expect(tonoDeCarga(89.9)).toBe('atencion');
		expect(tonoDeCarga(90)).toBe('critico');
		expect(tonoDeCarga(100)).toBe('critico');
	});
});
