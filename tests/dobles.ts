/**
 * Los dobles de lo que sólo existe adentro de la ventana de Tauri.
 *
 * Montar `App.vue` es lo único que comprueba de verdad que la barra lateral
 * cambia de pantalla; sin estos, importarla falla en la primera línea.
 */

const iconos = new Map<string, string>();
const oyentes = new Map<string, Set<(evento: { payload: unknown }) => unknown>>();

export const invocaciones: string[] = [];

/**
 * Lo que responde cada comando del backend.
 *
 * Sin esto `invoke` devolvía `undefined` para todo, y cualquier vista que
 * dibuje datos se quedaba en «midiendo» para siempre. Con el registro, una
 * prueba puede pedir la vista cargada, la vacía o la del error, que son
 * justamente los tres estados que ahora dibuja la librería.
 */
const respuestas = new Map<string, unknown>();

export function responderInvoke(comando: string, valor: unknown) {
	respuestas.set(comando, valor);
}

export async function invoke(comando: string) {
	invocaciones.push(comando);
	const respuesta = respuestas.get(comando);
	if (respuesta instanceof Error) throw respuesta;
	return respuesta;
}

export async function listen(nombre: string, manejador: (evento: { payload: unknown }) => unknown) {
	const suyos = oyentes.get(nombre) ?? new Set<(evento: { payload: unknown }) => unknown>();
	suyos.add(manejador);
	oyentes.set(nombre, suyos);
	return () => {
		suyos.delete(manejador);
	};
}

/** Dispara un evento del backend y espera a que lo atiendan. */
export async function emit(nombre: string, payload: unknown = null) {
	for (const manejador of [...(oyentes.get(nombre) ?? [])]) {
		await manejador({ payload });
	}
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
	// Las respuestas entran acá y no en un reinicio propio: `olvidarTodo` es el
	// que llaman los demás archivos de prueba, y dos funciones para lo mismo
	// son una que alguien va a olvidarse de llamar. Una respuesta que sobrevive
	// se la come el `invoke` de la prueba siguiente, que no registró ninguna.
	// Lo marcó la revisión.
	respuestas.clear();
}
