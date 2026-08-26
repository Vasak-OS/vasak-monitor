/**
 * Lo que decide el selector de la pantalla de registros.
 *
 * Vive acá y no dentro del componente para que se pueda probar: son reglas que se
 * equivocan en silencio —un icono que no resuelve se ve como un hueco, y una app
 * sin entradas que no se distingue de una que falló hace perder tiempo buscando.
 */

/** Una app del selector, como la informa el backend. */
export interface AppDelDiario {
	id: string;
	icono: string;
	presente: boolean;
}

/** Todo el ecosistema. Es el valor por omisión del selector. */
export const ECOSISTEMA = '';
/** El diario completo, VasakOS y lo demás. */
export const SISTEMA = '*';

/** El icono de las dos opciones amplias y el de una app cualquiera. */
export const ICONO_ECOSISTEMA = 'view-list';
export const ICONO_SISTEMA = 'applications-system';
export const ICONO_DESCONOCIDO = 'application-x-executable';

/**
 * El icono de lo que está seleccionado.
 *
 * Siempre un **nombre**, nunca una ruta: lo resuelve el plugin, y el tema puede
 * cambiar en caliente sin que sus rutas sirvan de nada.
 */
export function iconoDeSeleccion(app: string, apps: AppDelDiario[]): string {
	if (app === ECOSISTEMA) {
		return ICONO_ECOSISTEMA;
	}
	if (app === SISTEMA) {
		return ICONO_SISTEMA;
	}
	return apps.find((a) => a.id === app)?.icono ?? ICONO_DESCONOCIDO;
}

/**
 * Lo que se lee en cada opción del selector.
 *
 * Una app sin entradas se marca en lugar de esconderse: si no aparece, parece que
 * el monitor no la conoce.
 */
export function etiquetaDeApp(app: AppDelDiario, sinEntradas: string): string {
	return app.presente ? app.id : `${app.id} — ${sinEntradas}`;
}

/**
 * Si corresponde explicar por qué no hay nada.
 *
 * Sólo cuando se eligió **una** app: con «todo el ecosistema» vacío el problema es
 * otro, y con «todo el sistema» vacío no hay nada que explicar.
 */
export function pideExplicacionDelVacio(app: string, cuantas: number, cargando: boolean): boolean {
	return !cargando && cuantas === 0 && app !== ECOSISTEMA && app !== SISTEMA;
}
