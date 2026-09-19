/**
 * Los dobles de lo que sólo existe adentro de la ventana de Tauri.
 *
 * Montar `App.vue` es lo único que comprueba de verdad que la barra lateral
 * cambia de pantalla; sin estos, importarla falla en la primera línea.
 */

const iconos = new Map<string, string>();
const oyentes = new Map<string, Set<(evento: { payload: unknown }) => unknown>>();

export const invocaciones: string[] = [];

export async function invoke(comando: string) {
	invocaciones.push(comando);
	return undefined;
}

export async function listen(nombre: string, manejador: (evento: { payload: unknown }) => unknown) {
	const suyos = oyentes.get(nombre) ?? new Set<(evento: { payload: unknown }) => unknown>();
	suyos.add(manejador);
	oyentes.set(nombre, suyos);
	return () => {
		suyos.delete(manejador);
	};
}

export function ponerEnElTema(nombre: string, fuente: string) {
	iconos.set(nombre, fuente);
}

export async function getIconSource(nombre: string) {
	return iconos.get(nombre) ?? '';
}

export async function getSymbolSource(_nombre: string) {
	return '';
}

/** El `t()` devuelve la clave: una prueba que mire el texto mira la clave. */
export function useI18n() {
	return { t: (clave: string) => clave, locale: { value: 'es' } };
}

/** La configuración de la ventana, con lo mínimo que el monitor le pide. */
export function useConfigStore() {
	return {
		config: { monitor: { intervalo: 2000 } },
		loadConfig: async () => {},
	};
}

export function olvidarTodo() {
	invocaciones.length = 0;
	iconos.clear();
	oyentes.clear();
}
