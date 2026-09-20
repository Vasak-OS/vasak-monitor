/**
 * El intervalo que se elige sigue siendo un número.
 *
 * El desplegable venía con `v-model.number`, y el modificador no estaba
 * declarado en los tipos del componente: con `strictTemplates` eso pasó a ser un
 * error. Pero el modificador tampoco hacía nada acá — las opciones atan el
 * número con `:value="i"`, no una cadena, así que lo que vuelve ya es un
 * número—, así que se sacó.
 *
 * «No hacía nada» es justo lo que hay que comprobar: si el valor volviera como
 * `'5000'`, el `ref<number>` mentiría, `intervaloValido` lo rechazaría y la
 * medición se quedaría con el intervalo de antes sin decir por qué.
 */

import { afterEach, describe, expect, test } from 'bun:test';
import { SelectField } from '@vasakgroup/vue-libvasak';
import { mount, type VueWrapper } from '@vue/test-utils';
import { h, ref } from 'vue';
import { INTERVALOS } from '@/tools/sondeo';

let vista: VueWrapper | null = null;

afterEach(() => {
	vista?.unmount();
	vista = null;
});

describe('elegir un intervalo', () => {
	test('devuelve un número, no la cadena del `value`', async () => {
		const elegido = ref<number>(INTERVALOS[0]);
		// Se monta el mismo armado que usa la ventana: el desplegable compartido
		// con las opciones atando el número.
		vista = mount({
			setup: () =>
				() =>
					h(
						SelectField,
						{
							modelValue: elegido.value,
							'onUpdate:modelValue': (valor: number) => {
								elegido.value = valor;
							},
						},
						() => INTERVALOS.map((i) => h('option', { value: i }, `${i / 1000} s`))
					),
		});

		await vista.get('select').setValue(String(INTERVALOS[1]));

		expect(elegido.value).toBe(INTERVALOS[1]);
		expect(typeof elegido.value).toBe('number');
	});
});
