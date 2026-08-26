/**
 * Las dos reglas de la pantalla de limpieza que se equivocaban en silencio.
 *
 * Viven acá y no dentro del componente para poder probarlas: una limpieza que se
 * pisa con otra y un fallo que se borra a sí mismo no dejan rastro en la interfaz.
 */

/**
 * Si hay alguna limpieza en curso, sea de una tarea o de un grupo entero.
 *
 * Los botones de cada fila miraban sólo `ocupada`, así que durante «limpiar todo»
 * seguían habilitados y se podía lanzar `paccache` dos veces sobre la misma caché.
 */
export function hayLimpiezaEnCurso(limpiandoTodo: boolean, ocupada: string | null): boolean {
	return limpiandoTodo || ocupada !== null;
}

/**
 * El error que queda después de limpiar un grupo.
 *
 * Volver a medir limpia el error anterior, así que si los fallos se anotaban antes
 * de recargar desaparecían: la limpieza decía «Listo» aunque la mitad no se
 * hubiera hecho. Se juntan los dos, el de la recarga y los del grupo.
 */
export function errorTrasLimpiarGrupo(errorDeRecarga: string, fallos: string[]): string {
	return [errorDeRecarga, ...fallos].filter(Boolean).join(' · ');
}
