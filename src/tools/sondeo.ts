/**
 * Cuándo hay que volver a medir.
 *
 * Vive aparte para poder probarlo, y porque en un monitor esta decisión es la más
 * importante que hay: es la aplicación que muestra el consumo, así que no puede ser
 * la primera de su propia lista. Un `setInterval` que sigue midiendo con la ventana
 * tapada gasta exactamente lo que la pantalla dice que hay que cuidar.
 */

/** Los intervalos que se ofrecen, en milisegundos. */
export const INTERVALOS = [1000, 2000, 5000, 10_000] as const;

/** El de arranque: un segundo se siente vivo sin costar nada medible. */
export const INTERVALO_POR_OMISION = 2000;

/**
 * Si corresponde medir ahora.
 *
 * Tres condiciones, y las tres se rompieron por separado en otras aplicaciones del
 * escritorio:
 *
 *  - **Ventana tapada**: nadie mira los números que se actualizan.
 *  - **Pantalla que no mide**: Servicios, Limpieza y Registros no cambian solos;
 *    volver a consultarlos cada dos segundos es trabajo que nadie pidió.
 *  - **Ya hay una consulta en vuelo**: `setInterval` no espera a la anterior, así
 *    que si el backend tarda más que el intervalo, una respuesta vieja llega
 *    después de una nueva y pisa datos más recientes.
 */
export function debeMedir(oculto: boolean, pantallaEnVivo: boolean, enVuelo: boolean): boolean {
	return !oculto && pantallaEnVivo && !enVuelo;
}

/** Las pantallas que se actualizan solas. */
export const EN_VIVO = new Set(['recursos', 'aplicaciones']);

/** Si una pantalla necesita medir cada tanto. */
export function esEnVivo(pantalla: string): boolean {
	return EN_VIVO.has(pantalla);
}

/**
 * El intervalo válido más cercano a uno pedido.
 *
 * Un valor guardado de una versión anterior, o escrito a mano en la
 * configuración, no debe dejar el monitor midiendo cada milisegundo.
 */
export function intervaloValido(pedido: number): number {
	if (!Number.isFinite(pedido)) return INTERVALO_POR_OMISION;
	return INTERVALOS.includes(pedido as (typeof INTERVALOS)[number])
		? pedido
		: INTERVALO_POR_OMISION;
}
