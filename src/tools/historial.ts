/**
 * El búfer de muestras que alimenta los gráficos.
 *
 * Vive aparte del componente para poder probarlo: son cuentas chicas donde un
 * error no se ve —un gráfico que dibuja algo verosímil pero equivocado es peor
 * que uno vacío— y se rompen por separado.
 *
 * La historia es **de la sesión y en memoria**. Es lo que hacen todos los
 * monitores del sistema: no escribe nada al disco, no agrega un formato que
 * después haya que migrar, y un monitor escribiendo constantemente es justo lo
 * que un monitor no debería hacer.
 */

/** Cuántas muestras se guardan. Con el sondeo por defecto son varios minutos. */
export const CAPACIDAD = 120;

/**
 * Suma una muestra al final, descartando la más vieja si ya está lleno.
 *
 * Devuelve un arreglo nuevo en lugar de mutar: es lo que hace que Vue vea el
 * cambio sin tener que acordarse de disparar la reactividad a mano.
 *
 * `null` es una muestra válida y **no se descarta**: significa «acá no se pudo
 * medir», y saltearla juntaría los dos lados del hueco con una línea recta que
 * dice que todo anduvo bien.
 */
export function agregar(
	serie: readonly (number | null)[],
	muestra: number | null,
	capacidad = CAPACIDAD
): (number | null)[] {
	const siguiente = [...serie, muestra];
	return siguiente.length > capacidad ? siguiente.slice(siguiente.length - capacidad) : siguiente;
}

/**
 * El máximo de la serie, para escalar el eje vertical.
 *
 * `null` si no hay ningún valor medido. Quien dibuja decide qué hacer con eso —un
 * gráfico de porcentaje usa 100 fijo, uno de red no tiene techo conocido—.
 */
export function maximoDe(serie: readonly (number | null)[]): number | null {
	let max: number | null = null;
	for (const v of serie) {
		if (v === null) continue;
		if (max === null || v > max) max = v;
	}
	return max;
}

/**
 * Los puntos del gráfico, en coordenadas del `viewBox`.
 *
 * `techo` es el valor que llega arriba. Se ignora si es cero o negativo: dividir
 * por cero daría `Infinity` y el `path` saldría vacío sin que nada avise.
 *
 * Los `null` no se interpolan: cortan la línea. Por eso esto devuelve **tramos**
 * y no una lista de puntos suelta — un hueco de medición tiene que verse como un
 * hueco.
 */
export function tramosDe(
	serie: readonly (number | null)[],
	techo: number,
	ancho: number,
	alto: number
): { x: number; y: number }[][] {
	if (serie.length < 2 || techo <= 0) return [];

	const paso = ancho / (serie.length - 1);
	const tramos: { x: number; y: number }[][] = [];
	let actual: { x: number; y: number }[] = [];

	serie.forEach((valor, i) => {
		if (valor === null) {
			if (actual.length) tramos.push(actual);
			actual = [];
			return;
		}
		const acotado = Math.min(Math.max(valor, 0), techo);
		actual.push({ x: i * paso, y: alto - (acotado / techo) * alto });
	});

	if (actual.length) tramos.push(actual);
	// Un tramo de un solo punto no dibuja línea; se queda igual porque el
	// componente le pone un círculo, que es lo que corresponde a una muestra sola
	// entre dos huecos.
	return tramos;
}

/** Un tramo como atributo `d` de un `path`. */
export function comoPath(tramo: readonly { x: number; y: number }[]): string {
	if (!tramo.length) return '';
	return tramo.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' ');
}

/** El mismo tramo cerrado contra la base, para el relleno. */
export function comoArea(tramo: readonly { x: number; y: number }[], alto: number): string {
	if (tramo.length < 2) return '';
	const primero = tramo[0];
	const ultimo = tramo[tramo.length - 1];
	return `${comoPath(tramo)} L${ultimo.x.toFixed(2)},${alto} L${primero.x.toFixed(2)},${alto} Z`;
}
