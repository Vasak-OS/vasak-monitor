/**
 * El tema se recuerda entre arranques: la ventana abre ya con el esquema del usuario.
 *
 * Sin esto la ventana abre con el esquema por defecto de la hoja de estilos y
 * vira al del usuario con la transición de 300 ms, recién cuando el gestor de
 * configuración termina de leer. Se prueba la lógica con un elemento y un
 * `Storage` de prueba, y el cableado leyendo las fuentes: el script del `<head>`
 * corre antes del bundle y ninguna prueba de componentes lo ejecuta.
 */

import { describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { applyTheme, saveTheme, THEME_KEY } from '@/tools/theme-cache';

/** Un `Storage` mínimo en memoria. */
function makeStore(initial: Record<string, string> = {}): Storage {
	const data = new Map(Object.entries(initial));
	return {
		get length() {
			return data.size;
		},
		clear: () => data.clear(),
		getItem: (key: string) => data.get(key) ?? null,
		key: (index: number) => [...data.keys()][index] ?? null,
		removeItem: (key: string) => void data.delete(key),
		setItem: (key: string, value: string) => void data.set(key, value),
	};
}

/** Un `Storage` que tira en cada acceso, como un `localStorage` bloqueado. */
function makeBrokenStore(): Storage {
	const fail = () => {
		throw new Error('bloqueado');
	};
	return { getItem: fail, setItem: fail } as unknown as Storage;
}

function makeRoot(): HTMLElement {
	return document.createElement('html');
}

describe('theme-cache', () => {
	test('la clave es la que lee el script del <head>', () => {
		expect(THEME_KEY).toBe('vsk-theme');
	});

	test('saveTheme guarda la clase oscura y sólo las variables --*', () => {
		const root = makeRoot();
		root.classList.add('dark');
		root.style.setProperty('--ui-background', '#101010');
		root.style.setProperty('--primary', '#abcdef');
		root.style.setProperty('color', 'red');
		const store = makeStore();

		saveTheme(root, store);

		expect(JSON.parse(store.getItem(THEME_KEY) as string)).toEqual({
			dark: true,
			vars: { '--ui-background': '#101010', '--primary': '#abcdef' },
		});
	});

	test('saveTheme guarda dark en falso con el esquema claro', () => {
		const store = makeStore();
		saveTheme(makeRoot(), store);
		expect(JSON.parse(store.getItem(THEME_KEY) as string)).toEqual({ dark: false, vars: {} });
	});

	test('applyTheme reaplica la clase oscura y las variables', () => {
		const root = makeRoot();
		const store = makeStore({
			[THEME_KEY]: JSON.stringify({ dark: true, vars: { '--primary': '#abcdef' } }),
		});

		applyTheme(root, store);

		expect(root.classList.contains('dark')).toBe(true);
		expect(root.style.getPropertyValue('--primary')).toBe('#abcdef');
	});

	test('ida y vuelta: lo que se guarda en una ventana lo reaplica la siguiente', () => {
		const first = makeRoot();
		first.classList.add('dark');
		first.style.setProperty('--ui-surface', '#222222');
		const store = makeStore();
		saveTheme(first, store);

		const second = makeRoot();
		applyTheme(second, store);

		expect(second.classList.contains('dark')).toBe(true);
		expect(second.style.getPropertyValue('--ui-surface')).toBe('#222222');
	});

	test('sin clave guardada no toca nada', () => {
		const root = makeRoot();
		applyTheme(root, makeStore());
		expect(root.classList.contains('dark')).toBe(false);
		expect(root.getAttribute('style')).toBeNull();
	});

	test('un JSON inválido no lanza ni toca nada', () => {
		const root = makeRoot();
		expect(() => applyTheme(root, makeStore({ [THEME_KEY]: '{no es json' }))).not.toThrow();
		expect(root.classList.contains('dark')).toBe(false);
	});

	test('un Storage que tira no rompe el arranque', () => {
		expect(() => saveTheme(makeRoot(), makeBrokenStore())).not.toThrow();
		expect(() => applyTheme(makeRoot(), makeBrokenStore())).not.toThrow();
	});
});

describe('theme-cache: cableado', () => {
	const read = (path: string) => readFileSync(new URL(path, import.meta.url), 'utf8');

	test('index.html reaplica el tema antes de montar', () => {
		const html = read('../index.html');
		const head = html.slice(0, html.indexOf('</head>'));
		expect(head).toContain('vsk-theme');
		expect(head).toContain("classList.add(\"dark\")");
		expect(head).toContain('setProperty');
	});

	test('App.vue guarda el tema tras cada loadConfig', () => {
		const app = read('../src/App.vue');
		expect(app).toContain("from '@/tools/theme-cache'");
		const calls = app.match(/loadConfig\(\)\.then\(\(\) => saveTheme\(\)\)/g) ?? [];
		// Al montar y en el listener de config-changed.
		expect(calls.length).toBe(2);
	});
});
