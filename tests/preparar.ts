/**
 * Lo que tiene que estar listo antes de la primera prueba.
 *
 * El DOM, el complemento que compila los `.vue` —Bun los trata como un archivo
 * suelto y lo que se importa sin él es la ruta— y los dobles de Tauri, que los
 * componentes llaman al importarse.
 */

import { GlobalRegistrator } from '@happy-dom/global-registrator';
import { mock } from 'bun:test';
import './complemento-vue';
import {
	getIconSource,
	getSymbolSource,
	invoke,
	listen,
	useConfigStore,
	useI18n,
} from './dobles';

GlobalRegistrator.register();

// El módulo de verdad primero, para quedarse con lo que no se dobla. Al
// reemplazarlo entero desaparecían exportaciones que otros módulos del propio
// Tauri importan —`SERIALIZE_TO_IPC_FN`, por ejemplo— y la suite no arrancaba.
const core = await import('@tauri-apps/api/core');
const evento = await import('@tauri-apps/api/event');

mock.module('@tauri-apps/api/core', () => ({ ...core, invoke }));
mock.module('@tauri-apps/api/event', () => ({ ...evento, listen }));
mock.module('@vasakgroup/tauri-plugin-i18n', () => ({ useI18n }));
mock.module('@vasakgroup/plugin-vicons', () => ({ getIconSource, getSymbolSource }));
mock.module('@vasakgroup/plugin-config-manager', () => ({ useConfigStore }));

/**
 * Lo que Tauri le inyecta a la ventana al abrirla.
 *
 * `getCurrentWindow()` lo lee para saber de qué ventana se trata, y los
 * controles de la barra superior lo llaman en su `setup`. Sin esto, montar
 * cualquier cosa que incluya el marco de la ventana revienta antes de dibujar.
 */
(globalThis as Record<string, unknown>).__TAURI_INTERNALS__ = {
	metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
	invoke,
	transformCallback: (callback: unknown) => callback,
};
