//! Qué procesos están hablando con el compositor.
//!
//! # Por qué hace falta distinguirlos
//!
//! Una lista de aplicaciones donde `pipewire`, `akonadi_control` y `dbus-broker`
//! aparecen al lado de Firefox no sirve para lo que la gente abre un monitor:
//! saber qué de lo que *abrió* está pesando. Los servicios importan, pero tienen
//! su propia pantalla.
//!
//! # Cómo se detecta, y qué mide exactamente
//!
//! Un programa con interfaz gráfica mantiene abierta una conexión al socket del
//! compositor. `ss -x` lista esas conexiones, pero **sólo nombra el lado del
//! servidor** —el compositor—; del lado del cliente da el número de inodo del
//! socket. Cruzando esos inodos con los descriptores de cada proceso en
//! `/proc/<pid>/fd` se sabe quién es cada uno.
//!
//! Lo que esto mide es «tiene una conexión gráfica abierta», que **no es
//! exactamente «tiene una ventana visible»**: los agentes de KDE PIM se conectan
//! sin mostrar nada. Saber de ventanas de verdad exige hablar
//! `zwlr_foreign_toplevel_manager` con el compositor, y eso es un protocolo entero
//! para afinar una distinción que ya queda bien con esto. La interfaz nombra el
//! criterio por lo que es y no promete más.

use std::collections::HashSet;

/// Los inodos del lado cliente de cada conexión al compositor, de la salida de
/// `ss -x`.
///
/// Las líneas interesantes terminan en `<ruta> <inodo> * <inodo_par>`; se toma el
/// último campo, que es el del cliente. Tomar el penúltimo daría el del
/// compositor, y entonces ningún proceso coincidiría — la lista quedaría vacía sin
/// dar ningún error.
pub fn inodos_de_clientes(salida: &str) -> HashSet<String> {
    salida
        .lines()
        .filter(|l| l.contains("/wayland-"))
        .filter_map(|l| {
            let campos: Vec<&str> = l.split_whitespace().collect();
            let ultimo = campos.last()?;
            ultimo.chars().all(|c| c.is_ascii_digit()).then(|| ultimo.to_string())
        })
        .collect()
}

/// El inodo de un `socket:[N]` como los que devuelve `/proc/<pid>/fd`.
///
/// Los descriptores que no son sockets —archivos, tuberías, `anon_inode`— se
/// descartan: buscar el número en cualquier destino haría coincidir el inodo de un
/// archivo cualquiera y marcaría procesos que no tienen nada gráfico.
pub fn inodo_de_socket(destino: &str) -> Option<&str> {
    destino
        .strip_prefix("socket:[")
        .and_then(|resto| resto.strip_suffix(']'))
        .filter(|n| n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty())
}

/// Los pids que tienen una conexión al compositor.
pub fn pids_con_ventana(inodos: &HashSet<String>) -> HashSet<u32> {
    let mut con_ventana = HashSet::new();
    if inodos.is_empty() {
        return con_ventana;
    }

    let Ok(entradas) = std::fs::read_dir("/proc") else {
        return con_ventana;
    };

    for entrada in entradas.flatten() {
        let Ok(pid) = entrada.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        // Los descriptores de otro usuario no se pueden leer, y eso no es un
        // error: simplemente ese proceso no se puede clasificar.
        let Ok(descriptores) = std::fs::read_dir(entrada.path().join("fd")) else {
            continue;
        };
        for fd in descriptores.flatten() {
            let Ok(destino) = std::fs::read_link(fd.path()) else {
                continue;
            };
            let texto = destino.to_string_lossy();
            if let Some(inodo) = inodo_de_socket(&texto) {
                if inodos.contains(inodo) {
                    con_ventana.insert(pid);
                    break;
                }
            }
        }
    }

    con_ventana
}

/// Pregunta a `ss` y devuelve los pids con conexión gráfica.
pub fn detectar() -> HashSet<u32> {
    let salida = std::process::Command::new("ss")
        .arg("-x")
        .output()
        .ok()
        .and_then(|s| String::from_utf8(s.stdout).ok())
        .unwrap_or_default();

    pids_con_ventana(&inodos_de_clientes(&salida))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUESTRA: &str = "\
Netid State Recv-Q Send-Q Local Address:Port Peer Address:Port
u_str ESTAB 0      0        @/tmp/.X11-unix/X0 39401               * 36732
u_str ESTAB 0      0        /run/user/1000/wayland-1 24576663       * 24579589
u_str ESTAB 0      0        /run/user/1000/wayland-1 1845609        * 1838753
u_str ESTAB 0      0        /run/user/1000/bus 12345               * 54321
";

    #[test]
    fn se_toman_los_inodos_del_lado_cliente() {
        // El penúltimo campo es el del compositor: usándolo, ningún proceso
        // coincide y la lista queda vacía sin dar ningún error.
        let inodos = inodos_de_clientes(MUESTRA);
        assert_eq!(inodos.len(), 2);
        assert!(inodos.contains("24579589"));
        assert!(inodos.contains("1838753"));
        assert!(!inodos.contains("24576663"), "ese es el del compositor");
    }

    #[test]
    fn solo_cuentan_las_conexiones_al_compositor() {
        // El bus de sesión y el socket de X11 también son sockets unix, y contarlos
        // marcaría como gráfico a todo lo que hable D-Bus — que es casi todo.
        let inodos = inodos_de_clientes(MUESTRA);
        assert!(!inodos.contains("54321"), "el bus de sesión no cuenta");
        assert!(!inodos.contains("36732"), "el socket de X11 tampoco");
    }

    #[test]
    fn el_inodo_se_lee_solo_de_los_sockets() {
        assert_eq!(inodo_de_socket("socket:[12345]"), Some("12345"));
        // Un archivo cuyo nombre contiene números no es un socket: buscar el
        // número en cualquier destino marcaría procesos sin nada gráfico.
        assert_eq!(inodo_de_socket("/home/pato/12345.txt"), None);
        assert_eq!(inodo_de_socket("pipe:[12345]"), None);
        assert_eq!(inodo_de_socket("anon_inode:[eventfd]"), None);
        assert_eq!(inodo_de_socket("socket:[]"), None);
        assert_eq!(inodo_de_socket("socket:[abc]"), None);
        assert_eq!(inodo_de_socket(""), None);
    }

    #[test]
    fn sin_conexiones_no_se_recorre_proc() {
        // Sin compositor —una sesión por consola, o `ss` ausente— no hay a quién
        // clasificar, y recorrer /proc entero para no encontrar nada es gasto puro.
        assert!(pids_con_ventana(&HashSet::new()).is_empty());
    }

    #[test]
    fn una_salida_de_ss_rara_no_paniquea() {
        assert!(inodos_de_clientes("").is_empty());
        assert!(inodos_de_clientes("basura sin nada").is_empty());
        // Una línea que menciona wayland pero no termina en un número.
        assert!(inodos_de_clientes("u_str ESTAB /run/user/1000/wayland-1 LISTEN").is_empty());
    }
}
