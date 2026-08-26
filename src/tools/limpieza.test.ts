import { describe, expect, it } from 'bun:test';
import { errorTrasLimpiarGrupo, hayLimpiezaEnCurso } from '@/tools/limpieza';

describe('hayLimpiezaEnCurso', () => {
	it('lo está mientras corre un grupo', () => {
		// Con esto los botones de cada fila quedan deshabilitados: si no, se podía
		// lanzar la misma limpieza dos veces sobre el mismo recurso.
		expect(hayLimpiezaEnCurso(true, null)).toBe(true);
	});

	it('lo está mientras corre una sola tarea', () => {
		expect(hayLimpiezaEnCurso(false, 'papelera')).toBe(true);
	});

	it('no lo está cuando no hay nada corriendo', () => {
		expect(hayLimpiezaEnCurso(false, null)).toBe(false);
	});
});

describe('errorTrasLimpiarGrupo', () => {
	it('conserva los fallos del grupo', () => {
		// Volver a medir limpia el error anterior; sin juntarlos, la pantalla decía
		// «Listo» aunque la mitad no se hubiera hecho.
		expect(errorTrasLimpiarGrupo('', ['no se pudo tocar la caché'])).toBe(
			'no se pudo tocar la caché'
		);
	});

	it('junta el error de la recarga con los del grupo', () => {
		expect(errorTrasLimpiarGrupo('no se pudo medir', ['falló la papelera'])).toBe(
			'no se pudo medir · falló la papelera'
		);
	});

	it('no deja separadores colgando', () => {
		expect(errorTrasLimpiarGrupo('', [])).toBe('');
		expect(errorTrasLimpiarGrupo('solo la recarga', [])).toBe('solo la recarga');
		expect(errorTrasLimpiarGrupo('', ['a', 'b'])).toBe('a · b');
	});
});
