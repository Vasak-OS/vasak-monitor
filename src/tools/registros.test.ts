import { describe, expect, it } from 'bun:test';
import {
	type AppDelDiario,
	ECOSISTEMA,
	etiquetaDeApp,
	ICONO_DESCONOCIDO,
	ICONO_ECOSISTEMA,
	ICONO_SISTEMA,
	iconoDeSeleccion,
	pideExplicacionDelVacio,
	SISTEMA,
} from '@/tools/registros';

const apps: AppDelDiario[] = [
	{ id: 'vasak-keyring', icono: 'dialog-password', presente: true },
	{ id: 'vasak-terminal', icono: 'vasak-terminal', presente: false },
];

describe('iconoDeSeleccion', () => {
	it('distingue las dos opciones amplias', () => {
		expect(iconoDeSeleccion(ECOSISTEMA, apps)).toBe(ICONO_ECOSISTEMA);
		expect(iconoDeSeleccion(SISTEMA, apps)).toBe(ICONO_SISTEMA);
	});

	it('usa el icono que informó el backend', () => {
		expect(iconoDeSeleccion('vasak-keyring', apps)).toBe('dialog-password');
	});

	it('no deja un hueco cuando no conoce la app', () => {
		// Puede pasar: el catálogo se pide una vez y una app nueva del diario
		// llegaría sin icono.
		expect(iconoDeSeleccion('vasak-flamante', apps)).toBe(ICONO_DESCONOCIDO);
		expect(iconoDeSeleccion('vasak-keyring', [])).toBe(ICONO_DESCONOCIDO);
	});

	it('nunca devuelve una ruta', () => {
		// El tema cambia en caliente y sus rutas no son estables: el nombre lo
		// resuelve el plugin.
		for (const app of [ECOSISTEMA, SISTEMA, 'vasak-keyring', 'lo-que-sea']) {
			const icono = iconoDeSeleccion(app, apps);
			expect(icono).not.toContain('/');
			expect(icono).not.toMatch(/\.(svg|png)$/);
		}
	});
});

describe('etiquetaDeApp', () => {
	it('marca la que no escribió nada', () => {
		expect(etiquetaDeApp(apps[1], 'sin entradas')).toBe('vasak-terminal — sin entradas');
	});

	it('deja limpia la que sí escribió', () => {
		expect(etiquetaDeApp(apps[0], 'sin entradas')).toBe('vasak-keyring');
	});
});

describe('pideExplicacionDelVacio', () => {
	it('explica el vacío de una app elegida', () => {
		expect(pideExplicacionDelVacio('vasak-terminal', 0, false)).toBe(true);
	});

	it('no explica nada mientras carga', () => {
		// Si no, el mensaje aparece y desaparece en cada actualización.
		expect(pideExplicacionDelVacio('vasak-terminal', 0, true)).toBe(false);
	});

	it('no explica el vacío de las opciones amplias', () => {
		expect(pideExplicacionDelVacio(ECOSISTEMA, 0, false)).toBe(false);
		expect(pideExplicacionDelVacio(SISTEMA, 0, false)).toBe(false);
	});

	it('no explica nada si hay entradas', () => {
		expect(pideExplicacionDelVacio('vasak-terminal', 3, false)).toBe(false);
	});
});
