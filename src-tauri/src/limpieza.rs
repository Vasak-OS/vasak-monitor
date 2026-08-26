//! Recuperar espacio, y ser honesto sobre qué recupera de verdad.
//!
//! # La parte incómoda: «liberar RAM»
//!
//! Casi todo lo que un botón de «liberar memoria» hace en Linux es inútil o
//! contraproducente. La memoria que parece ocupada es en su mayoría caché de disco,
//! y el kernel la devuelve sola en cuanto alguien la pide: vaciarla a mano no
//! libera nada que no estuviera disponible, y obliga a volver a leer del disco todo
//! lo que estaba en memoria. Es más lento después, no más rápido.
//!
//! Así que acá no hay un botón que prometa eso. Hay dos cosas que **sí** hacen algo
//! medible, cada una diciendo qué hace:
//!
//! - **Devolver el swap a la memoria**: con RAM libre, las páginas que quedaron en
//!   disco vuelven y el sistema deja de leerlas de ahí. En esta máquina el swap
//!   estaba al 69% con la RAM al 44%, que es justo el caso donde sirve.
//! - **Vaciar la caché del kernel**: se ofrece, explicado, porque a veces se quiere
//!   medir algo desde frío. No como una mejora.

use std::path::PathBuf;

/// Algo que se puede limpiar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tarea {
    /// `~/.cache`. Lo más grande y lo menos riesgoso: son datos que cada programa
    /// vuelve a generar.
    CacheDeUsuario,
    /// La papelera del usuario.
    Papelera,
    /// Los paquetes descargados que pacman guarda.
    CacheDePaquetes,
    /// Los paquetes que quedaron instalados sin que nada los necesite.
    PaquetesHuerfanos,
    /// El diario del sistema, recortado a lo reciente.
    DiarioViejo,
    /// Devolver a la memoria lo que está en swap.
    SwapALaMemoria,
    /// Vaciar la caché de disco del kernel.
    CacheDelKernel,
}

impl Tarea {
    /// Si hace falta autenticar para hacerla.
    ///
    /// Lo que vive en el directorio del usuario no necesita nada; lo del sistema
    /// sí, y pasa por `pkexec` para que lo pregunte el agente de polkit que ya
    /// está corriendo.
    pub fn necesita_autenticar(self) -> bool {
        !matches!(self, Tarea::CacheDeUsuario | Tarea::Papelera)
    }

    /// Si recupera espacio en disco (y por lo tanto se mide en bytes) o no.
    ///
    /// Las dos de memoria no liberan disco, y mostrarlas con un tamaño al lado las
    /// haría parecer lo mismo que las otras.
    pub fn recupera_disco(self) -> bool {
        !matches!(self, Tarea::SwapALaMemoria | Tarea::CacheDelKernel)
    }
}

/// La ruta de una tarea que vive en el directorio del usuario.
pub fn ruta_de(tarea: Tarea, home: &str) -> Option<PathBuf> {
    match tarea {
        Tarea::CacheDeUsuario => Some(PathBuf::from(home).join(".cache")),
        Tarea::Papelera => Some(PathBuf::from(home).join(".local/share/Trash")),
        _ => None,
    }
}

/// Lee el tamaño de la salida de `du -sb`.
///
/// `du` imprime «<bytes>\t<ruta>». Se toma el primer campo: usar la línea entera
/// deja el número pegado a la ruta y el parseo falla, mostrando 0 bytes
/// recuperables en una carpeta de once gigabytes.
pub fn bytes_de_du(salida: &str) -> Option<u64> {
    salida
        .lines()
        .next()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// Lee el tamaño del diario de `journalctl --disk-usage`.
///
/// La frase es «Archived and active journals take up 49.3M in the file system.» —
/// se busca el número con su sufijo. No hay opción para que lo diga en bytes.
pub fn bytes_del_diario(salida: &str) -> Option<u64> {
    let token = salida
        .split_whitespace()
        .find(|t| {
            t.len() >= 2
                && t.starts_with(|c: char| c.is_ascii_digit())
                && t.ends_with(|c: char| c.is_ascii_alphabetic())
        })?;

    let (numero, sufijo) = token.split_at(token.len() - 1);
    // El separador decimal de journalctl depende del idioma de la sesión.
    let valor: f64 = numero.replace(',', ".").parse().ok()?;

    let factor = match sufijo {
        "K" | "k" => 1024.0,
        "M" => 1024.0 * 1024.0,
        "G" => 1024.0 * 1024.0 * 1024.0,
        "T" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "B" => 1.0,
        _ => return None,
    };

    Some((valor * factor) as u64)
}

/// Cuenta los paquetes huérfanos de la salida de `pacman -Qtdq`.
pub fn huerfanos_de(salida: &str) -> usize {
    salida.lines().filter(|l| !l.trim().is_empty()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_del_usuario_no_pide_autenticar_y_lo_del_sistema_si() {
        assert!(!Tarea::CacheDeUsuario.necesita_autenticar());
        assert!(!Tarea::Papelera.necesita_autenticar());
        assert!(Tarea::CacheDePaquetes.necesita_autenticar());
        assert!(Tarea::DiarioViejo.necesita_autenticar());
        assert!(Tarea::SwapALaMemoria.necesita_autenticar());
    }

    #[test]
    fn las_tareas_de_memoria_no_se_miden_en_bytes_de_disco() {
        // Mostrarlas con un tamaño al lado las haría parecer lo mismo que las
        // otras, y no recuperan un byte de disco.
        assert!(!Tarea::SwapALaMemoria.recupera_disco());
        assert!(!Tarea::CacheDelKernel.recupera_disco());
        assert!(Tarea::CacheDeUsuario.recupera_disco());
        assert!(Tarea::CacheDePaquetes.recupera_disco());
    }

    #[test]
    fn las_rutas_del_usuario_salen_de_su_home() {
        assert_eq!(
            ruta_de(Tarea::CacheDeUsuario, "/home/pato"),
            Some(PathBuf::from("/home/pato/.cache"))
        );
        assert_eq!(
            ruta_de(Tarea::Papelera, "/home/pato"),
            Some(PathBuf::from("/home/pato/.local/share/Trash"))
        );
        // Las del sistema no tienen ruta en el home, y devolver una haría que se
        // midiera —y se borrara— el lugar equivocado.
        assert_eq!(ruta_de(Tarea::CacheDePaquetes, "/home/pato"), None);
    }

    #[test]
    fn el_tamano_de_du_sale_del_primer_campo() {
        // Usando la línea entera, el número queda pegado a la ruta y el parseo
        // falla: se mostraría 0 bytes recuperables en una carpeta de 11 GB.
        assert_eq!(bytes_de_du("11811160064\t/home/pato/.cache\n"), Some(11_811_160_064));
        assert_eq!(bytes_de_du("512\t/tmp/x"), Some(512));
        assert_eq!(bytes_de_du(""), None);
        assert_eq!(bytes_de_du("no-es-un-numero\t/x"), None);
    }

    #[test]
    fn el_tamano_del_diario_se_lee_de_la_frase() {
        // No hay opción para que journalctl lo diga en bytes.
        let en_ingles = "Archived and active journals take up 49.3M in the file system.";
        let leido = bytes_del_diario(en_ingles).expect("hay tamaño");
        assert!(
            (leido as f64 - 49.3 * 1024.0 * 1024.0).abs() < 1024.0,
            "dio {leido}"
        );
    }

    #[test]
    fn el_separador_decimal_depende_del_idioma() {
        // En una sesión en español journalctl escribe «49,3M». Sin manejarlo, el
        // parseo falla y el diario aparece como 0 bytes — exactamente en las
        // máquinas donde este escritorio corre.
        let en_espanol = "Los diarios archivados y activos ocupan 49,3M en el sistema de archivos.";
        assert!(bytes_del_diario(en_espanol).is_some());
        assert_eq!(bytes_del_diario(en_espanol), bytes_del_diario("ocupan 49.3M"));
    }

    #[test]
    fn un_sufijo_desconocido_no_se_inventa() {
        assert_eq!(bytes_del_diario("ocupan 49,3X en el disco"), None);
        assert_eq!(bytes_del_diario("sin numeros aca"), None);
        assert_eq!(bytes_del_diario(""), None);
    }

    #[test]
    fn se_cuentan_los_huerfanos() {
        assert_eq!(huerfanos_de("paquete-a\npaquete-b\npaquete-c\n"), 3);
        // Sin huérfanos, pacman no imprime nada.
        assert_eq!(huerfanos_de(""), 0);
        assert_eq!(huerfanos_de("\n\n"), 0);
    }
}
