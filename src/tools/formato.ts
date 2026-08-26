/**
 * Cómo se muestran los números.
 *
 * Aparte para poder probarlo: un tamaño mal formateado no falla, se lee mal — y
 * «1438291 B» en una tabla de discos es peor que no mostrar nada.
 */

/** Un tamaño en la unidad más grande que deje una o dos cifras enteras. */
export function tamano(bytes: number): string {
	const unidades = ['B', 'kB', 'MB', 'GB', 'TB'];
	let valor = Math.max(0, bytes);
	let i = 0;
	while (valor >= 1000 && i + 1 < unidades.length) {
		valor /= 1000;
		i += 1;
	}
	// Sin decimales para bytes: «512,0 B» no aporta nada.
	const decimales = i === 0 ? 0 : 1;
	return `${valor.toFixed(decimales)} ${unidades[i]}`;
}

/** Un caudal de red. */
export function caudal(bytesPorSegundo: number | null): string {
	if (bytesPorSegundo === null) return '—';
	return `${tamano(bytesPorSegundo)}/s`;
}

/**
 * Un porcentaje, o un guion cuando todavía no se sabe.
 *
 * `null` en la primera muestra: la CPU y la red sólo existen como diferencia, y
 * mostrar 0% ahí dibuja una caída a cero que no ocurrió.
 */
export function porcentaje(valor: number | null): string {
	return valor === null ? '—' : `${valor.toFixed(1)} %`;
}

/** El color de una barra según lo llena que esté. */
export function tonoDeCarga(porciento: number): 'normal' | 'atencion' | 'critico' {
	if (porciento >= 90) return 'critico';
	if (porciento >= 75) return 'atencion';
	return 'normal';
}
