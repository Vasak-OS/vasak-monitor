//! Punto de entrada de una aplicación de VasakOS.
//!
//! Lo que hay acá no es decoración: cada pieza resuelve algo que en las
//! aplicaciones reales del escritorio se rompió al menos una vez.

mod locales;
pub mod muestreo;
pub mod comandos;
pub mod limpieza;
pub mod registros;
pub mod servicios;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // El idioma de la sesión. **Con la ruta explícita de los catálogos**:
        // el plugin sólo prueba rutas relativas al ejecutable y al directorio
        // de trabajo, y ninguna existe cuando el binario está en /usr/bin. Sin
        // esto, un paquete instalado muestra las claves crudas
        // («views.home.title») en lugar de los textos. Ver `locales.rs`.
        .plugin(tauri_plugin_i18n_vsk::init_with_path(
            Some(locales::idioma_del_sistema()),
            locales::directorio(),
        ))
        // El clic derecho abre el menú de VasakOS y no el del motor del
        // navegador, que ofrece «Recargar» e «Inspeccionar elemento».
        .plugin(tauri_plugin_vsk_contextual_menu::init())
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            comandos::recursos,
            comandos::aplicaciones,
            comandos::cerrar,
            comandos::lista_de_servicios,
            comandos::accion_de_servicio,
            comandos::recuperable,
            comandos::limpiar,
            comandos::limpiar_todo,
            comandos::apps_del_diario,
            comandos::registros_de_vasakos,
            comandos::desplazamiento_horario,
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación");
}
