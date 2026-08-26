/**
 * Medir cada tanto, y sólo cuando corresponde.
 *
 * El intervalo se reagenda con `setTimeout` y no con `setInterval` a propósito: así
 * la próxima medición se cuenta desde que la anterior **terminó**, y una consulta
 * lenta no se solapa con la siguiente.
 */
import { onMounted, onUnmounted, ref, watch } from 'vue';
import { debeMedir } from '@/tools/sondeo';

export function useSondeo(
	medir: () => Promise<void>,
	intervalo: () => number,
	enVivo: () => boolean
) {
	const enVuelo = ref(false);
	let temporizador: ReturnType<typeof setTimeout> | null = null;

	const oculto = () => typeof document !== 'undefined' && document.hidden;

	async function vuelta() {
		if (debeMedir(oculto(), enVivo(), enVuelo.value)) {
			enVuelo.value = true;
			try {
				await medir();
			} finally {
				enVuelo.value = false;
			}
		}
		agendar();
	}

	function agendar() {
		if (temporizador !== null) clearTimeout(temporizador);
		// Se sigue agendando aunque esta vuelta no midiera: es lo que hace que al
		// volver la ventana a la vista se retome sin depender de ningún evento.
		temporizador = setTimeout(() => void vuelta(), intervalo());
	}

	function detener() {
		if (temporizador !== null) {
			clearTimeout(temporizador);
			temporizador = null;
		}
	}

	// Al volver a la vista se mide enseguida en lugar de esperar el intervalo: si
	// estuvo tapada un rato, lo que se muestra es viejo.
	const alCambiarVisibilidad = () => {
		if (!oculto()) void vuelta();
	};

	onMounted(() => {
		document.addEventListener('visibilitychange', alCambiarVisibilidad);
		void vuelta();
	});

	onUnmounted(() => {
		document.removeEventListener('visibilitychange', alCambiarVisibilidad);
		detener();
	});

	// Cambiar de pantalla o de intervalo reagenda con el valor nuevo, sin esperar a
	// que venza el anterior.
	watch([intervalo, enVivo], () => agendar());

	return { enVuelo, medirAhora: vuelta };
}
