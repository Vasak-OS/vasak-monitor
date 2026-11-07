//! Que el afinado del WebKit siga cableado de punta a punta.
//!
//! El glue de GTK (`commands/webkit.rs`) baja al `WebKitWebView` por el árbol de
//! widgets y no se puede ejercitar sin un display ni un WebKit reales, así que
//! esta prueba lee el fuente y comprueba el cableado: que el comando exista con
//! sus tres palancas, que los extras de desarrollo se apaguen sólo en release,
//! que no se toque lo que haría tironear al monitor, que el comando esté
//! registrado y que el frontend lo invoque al montar. No corre GTK.
//!
//! Si algo de esto se cae, el afinado deja de aplicarse sin que nada falle al
//! compilar: por eso la guarda.

use std::path::PathBuf;

fn leer(relativa: &str) -> String {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relativa);
    std::fs::read_to_string(&ruta)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", ruta.display()))
}

#[test]
fn el_comando_existe_y_afina_las_tres_palancas() {
    let fuente = leer("src/commands/webkit.rs");
    assert!(
        fuente.contains("pub fn tune_webview"),
        "falta el comando `tune_webview` en commands/webkit.rs: sin él el \
         frontend invoca un comando que no existe y el afinado no se aplica"
    );
    for palanca in [
        "set_enable_developer_extras(false)",
        "set_enable_back_forward_navigation_gestures(false)",
        "CacheModel::DocumentViewer",
    ] {
        assert!(
            fuente.contains(palanca),
            "commands/webkit.rs no aplica «{palanca}»: el webview vuelve a \
             arrancar con el perfil de un navegador y el consumo de memoria sube"
        );
    }
}

#[test]
fn los_extras_de_desarrollo_solo_se_apagan_en_release() {
    // En debug hay que poder inspeccionar; apagarlos siempre los pisaría.
    let fuente = leer("src/commands/webkit.rs");
    let lineas: Vec<&str> = fuente.lines().collect();
    let indice = lineas
        .iter()
        .position(|linea| linea.contains("set_enable_developer_extras(false)"))
        .expect("commands/webkit.rs tiene que apagar los extras de desarrollo en release");
    let anterior = lineas[..indice]
        .iter()
        .rev()
        .find(|linea| !linea.trim().is_empty())
        .expect("la llamada no puede ser la primera línea del archivo");
    assert_eq!(
        anterior.trim(),
        "#[cfg(not(debug_assertions))]",
        "los extras de desarrollo tienen que apagarse sólo con \
         #[cfg(not(debug_assertions))]: en debug se inspecciona, y apagarlos \
         ahí rompe esa inspección"
    );
}

#[test]
fn la_aceleracion_y_el_webgl_no_se_tocan() {
    // El monitor anima sus gráficos: apagarlos los haría tironear.
    let fuente = leer("src/commands/webkit.rs");
    for palanca in ["set_hardware_acceleration_policy", "set_enable_webgl"] {
        assert!(
            !fuente.contains(palanca),
            "commands/webkit.rs no debe llamar a «{palanca}»: el monitor anima \
             sus gráficos y apagarlos los haría tironear"
        );
    }
}

#[test]
fn el_comando_aplica_el_afinado_al_webview() {
    // Con las palancas puestas pero sin esta llamada, `tune_webkit` nunca corre
    // y el afinado no llega al webview.
    assert!(
        leer("src/commands/webkit.rs").contains("tune_webkit(&webview)"),
        "commands/webkit.rs no llama a `tune_webkit(&webview)` en tune_webview: \
         las palancas quedan definidas pero nunca se aplican y el consumo de \
         memoria no baja"
    );
}

#[test]
fn el_comando_esta_registrado_en_lib() {
    assert!(
        leer("src/lib.rs").contains("commands::webkit::tune_webview"),
        "lib.rs no registra commands::webkit::tune_webview en generate_handler: \
         la invocación del frontend vuelve como «command not found»"
    );
}

#[test]
fn el_frontend_lo_invoca_al_montar() {
    assert!(
        leer("../src/App.vue").contains("invoke('tune_webview')"),
        "App.vue no invoca tune_webview al montar: el comando queda registrado \
         pero nadie lo llama y el afinado nunca se aplica"
    );
}
