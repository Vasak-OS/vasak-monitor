//! Las carpetas que los proyectos de desarrollo regeneran solas.
//!
//! `node_modules`, el `target` de Cargo, los entornos de Python. Son lo que más
//! espacio ocupa en la máquina de quien programa y lo que menos falta hace
//! conservar: se vuelven a bajar o a compilar. En esta máquina los `target` de un
//! solo espacio de trabajo llegaron a 350 GB.
//!
//! # Qué se ofrece y qué no
//!
//! Borrar por nombre es peligroso, y por tres motivos distintos.
//!
//! El primero es que los nombres se repiten. `target` es de Cargo sólo si hay un
//! `Cargo.toml` al lado; en cualquier otro lugar puede ser la carpeta de un
//! diseñador. `vendor` es de Go si hay un `go.mod`. Por eso cada patrón puede
//! pedir un archivo **hermano** que lo confirme.
//!
//! El segundo es que algunas de estas carpetas están **versionadas**. `vendor`
//! commiteado es una práctica normal en Go, y `dist` a veces también. Ahí borrar
//! no recupera espacio: rompe el repositorio. Así que la regla es la misma que
//! usa `git clean -ffdX`: se ofrece lo que **git ignora**. Un patrón sólo se
//! ofrece fuera de un repositorio si su nombre no deja lugar a dudas —eso es
//! `inequivoca`—.
//!
//! El tercero es la **ubicación**. Lo que vive suelto en `$HOME` con nombre de
//! herramienta es el home de esa herramienta, no la carpeta de un proyecto:
//! `$HOME/.gradle` tiene el `gradle.properties` global y los scripts de `init.d`,
//! que no los regenera nadie. Por eso una hija directa de `$HOME` nunca se
//! ofrece — es una regla estructural y no una lista de excepciones, así que
//! también cubre `.cargo` o `.npm` el día que alguien los agregue como patrón.
//!
//! Las tres reglas juntas significan que esto no puede borrar algo versionado, ni
//! algo que sólo se parece a una carpeta de dependencias, ni la configuración de
//! una herramienta.

use std::path::{Path, PathBuf};

/// De qué herramienta es la carpeta.
///
/// Sirve para el icono y para agrupar en la interfaz; el comportamiento lo decide
/// el patrón, no la clase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Clase {
    Node,
    Cargo,
    Python,
    Gradle,
    Go,
    Compilacion,
}

/// Una carpeta que se busca, y qué hace falta para que cuente.
#[derive(Debug, Clone, Copy)]
pub struct Patron {
    /// El nombre exacto del directorio.
    pub nombre: &'static str,
    pub clase: Clase,
    /// Los archivos que confirman que es un proyecto: **alcanza uno**.
    ///
    /// `target` sin `Cargo.toml` hermano no es de Cargo, y `vendor` sin `go.mod`
    /// no es de Go. Sin esta comprobación, «limpiar proyectos» se llevaría la
    /// carpeta `build` de cualquier cosa.
    ///
    /// Es una lista y no uno solo porque varias herramientas tienen más de un
    /// nombre para lo mismo: un proyecto Gradle se reconoce por `build.gradle` o
    /// por `settings.gradle.kts`, y pedir sólo el primero dejaría afuera la mitad.
    ///
    /// Vacía significa «no hace falta ninguno», y eso hay que mirarlo dos veces:
    /// un patrón sin marca se acepta en cualquier lugar donde aparezca su nombre.
    pub marcas: &'static [&'static str],
    /// Si se puede ofrecer aunque no haya repositorio git que la ignore.
    ///
    /// Sólo para los nombres que no dejan lugar a dudas. `node_modules` es
    /// siempre de npm y siempre se regenera; `dist` puede ser cualquier cosa, y
    /// fuera de un repositorio no hay nada que confirme que sobra.
    pub inequivoca: bool,
}

/// Todo lo que se busca.
///
/// El orden importa poco, pero los nombres no se repiten: `carpeta_de` devuelve el
/// primero que coincide.
pub const PATRONES: &[Patron] = &[
    Patron {
        nombre: "node_modules",
        clase: Clase::Node,
        marcas: &["package.json"],
        inequivoca: true,
    },
    Patron {
        nombre: "target",
        clase: Clase::Cargo,
        marcas: &["Cargo.toml"],
        inequivoca: true,
    },
    Patron {
        nombre: ".venv",
        clase: Clase::Python,
        // Sin marca: `.venv` es siempre un entorno virtual. Y aunque apareciera
        // suelto en `$HOME`, la regla de «nunca un hijo directo de $HOME» —ver
        // `es_hijo_directo`— lo deja afuera.
        marcas: &[],
        inequivoca: true,
    },
    Patron {
        nombre: "venv",
        clase: Clase::Python,
        // `venv` sin punto es un nombre bastante común para otras cosas.
        marcas: &["pyproject.toml", "requirements.txt", "setup.py"],
        inequivoca: false,
    },
    Patron {
        nombre: "__pycache__",
        clase: Clase::Python,
        marcas: &[],
        inequivoca: true,
    },
    Patron {
        nombre: ".gradle",
        clase: Clase::Gradle,
        // **Con marcas obligatorias, y no es un detalle.** `$HOME/.gradle` es el
        // home de Gradle: tiene el `gradle.properties` global y los scripts de
        // `init.d`, que no los regenera nadie. Sin pedir un archivo de proyecto
        // al lado, esta carpeta se ofrecía para borrar y se perdía configuración
        // escrita a mano.
        marcas: &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "gradlew",
        ],
        inequivoca: true,
    },
    Patron {
        nombre: "vendor",
        clase: Clase::Go,
        marcas: &["go.mod"],
        // Commitear `vendor` es práctica normal en Go: sólo se ofrece si el
        // repositorio lo ignora.
        inequivoca: false,
    },
    Patron {
        nombre: ".next",
        clase: Clase::Compilacion,
        marcas: &["package.json"],
        inequivoca: true,
    },
    Patron {
        nombre: ".nuxt",
        clase: Clase::Compilacion,
        marcas: &["package.json"],
        inequivoca: true,
    },
    Patron {
        nombre: "dist",
        clase: Clase::Compilacion,
        marcas: &["package.json"],
        inequivoca: false,
    },
    Patron {
        nombre: "build",
        clase: Clase::Compilacion,
        marcas: &["package.json"],
        inequivoca: false,
    },
];

/// El patrón que corresponde a un nombre de directorio, si hay alguno.
pub fn patron_de(nombre: &str) -> Option<&'static Patron> {
    PATRONES.iter().find(|p| p.nombre == nombre)
}

/// Una carpeta encontrada, lista para ofrecer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Hallazgo {
    pub ruta: PathBuf,
    pub clase: Clase,
    /// El proyecto al que pertenece, para mostrarlo sin la ruta entera.
    pub proyecto: String,
    /// `None` mientras no se midió: medir es lo lento y va después de listar.
    pub bytes: Option<u64>,
}

/// Si una carpeta encontrada se puede ofrecer para borrar.
///
/// Es la regla de seguridad y está sola para poder probarla. `en_repo` dice si la
/// carpeta cae dentro de un repositorio git; `ignorada`, si ese repositorio la
/// ignora.
///
/// - Dentro de un repositorio: **sólo si está ignorada**. Una carpeta versionada
///   no es espacio recuperable, es contenido del proyecto.
/// - Fuera de un repositorio: sólo los nombres inequívocos, porque no hay nada
///   que confirme que sobra.
pub fn se_puede_ofrecer(patron: &Patron, en_repo: bool, ignorada: bool) -> bool {
    if en_repo {
        ignorada
    } else {
        patron.inequivoca
    }
}

/// Si un directorio se salta durante el recorrido.
///
/// Los ocultos no se recorren —salvo los que son un patrón, como `.venv`—: ahí
/// viven las cachés de las aplicaciones, que ya tienen su propia tarea de
/// limpieza, y recorrerlos multiplica el tiempo del escaneo sin encontrar nada.
pub fn se_saltea(nombre: &str) -> bool {
    nombre.starts_with('.') && patron_de(nombre).is_none()
}

/// El nombre del proyecto al que pertenece una carpeta.
///
/// Es el nombre del directorio padre, que es lo que alguien reconoce. Con la ruta
/// entera la lista se vuelve ilegible: veinte líneas de
/// `/home/pato/VasakOS/…/src-tauri/target` que se diferencian en el medio.
pub fn proyecto_de(ruta: &Path) -> String {
    ruta.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Recorre un árbol y devuelve las carpetas candidatas, sin medirlas.
///
/// `profundidad` es cuántos niveles baja desde la raíz. Con `$HOME` como raíz,
/// seis alcanza para los proyectos que la gente tiene —`~/dev/org/repo/paquete/`
/// ya son cuatro— y evita perderse en un árbol de dependencias.
///
/// **No entra en lo que ya encontró.** Dentro de un `node_modules` hay cientos de
/// `node_modules` anidados, y ofrecerlos por separado no sirve para nada: se borra
/// el de arriba y se van todos. Sin este corte, el escaneo tarda órdenes de
/// magnitud más y devuelve una lista inútil.
///
/// El filtro de seguridad no está acá: esto sólo encuentra. Quien llama decide con
/// `se_puede_ofrecer`, que es lo que necesita git y no se puede probar sin él.
pub fn candidatas(raiz: &Path, profundidad: usize) -> Vec<(PathBuf, &'static Patron)> {
    let mut encontradas = Vec::new();
    recorrer(raiz, raiz, profundidad, &mut encontradas);
    encontradas
}

fn recorrer(
    dir: &Path,
    raiz: &Path,
    resta: usize,
    salida: &mut Vec<(PathBuf, &'static Patron)>,
) {
    if resta == 0 {
        return;
    }
    let Ok(entradas) = std::fs::read_dir(dir) else {
        // Un directorio sin permiso de lectura no es un error del escaneo: se
        // saltea y se sigue. Abortar por uno dejaría la lista a medias sin decirlo.
        return;
    };

    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        // `symlink_metadata` y no `metadata`: seguir enlaces puede salir del árbol
        // —y volver a entrar, en un ciclo— y lo que se ofrezca borrar tiene que
        // estar donde dice estar.
        let Ok(meta) = entrada.path().symlink_metadata() else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }

        let nombre = entrada.file_name().to_string_lossy().into_owned();

        if let Some(patron) = patron_de(&nombre) {
            // La ubicación se comprueba antes que la marca: una hija directa de la
            // raíz no se ofrece ni con marca, porque ahí lo que hay es el home de
            // una herramienta y no un proyecto.
            if !es_hijo_directo(&ruta, raiz) && tiene_la_marca(&ruta, patron) {
                salida.push((ruta, patron));
                // No se baja: lo de adentro se va con esto.
                continue;
            }
        }

        if se_saltea(&nombre) {
            continue;
        }
        recorrer(&ruta, raiz, resta - 1, salida);
    }
}

/// Si alguno de los archivos hermanos que el patrón pide está donde tiene que estar.
///
/// Alcanza uno: un proyecto Gradle tiene `build.gradle` **o** `settings.gradle.kts`,
/// no los dos.
pub fn tiene_la_marca(ruta: &Path, patron: &Patron) -> bool {
    if patron.marcas.is_empty() {
        return true;
    }
    let Some(padre) = ruta.parent() else {
        return false;
    };
    patron.marcas.iter().any(|m| padre.join(m).is_file())
}

/// Si una carpeta es hija directa del directorio del usuario.
///
/// Ninguna de esas se ofrece, y es una regla estructural, no una lista de
/// excepciones. Lo que vive suelto en `$HOME` con nombre de herramienta es el
/// **home de esa herramienta** —`.gradle`, `.cargo`, `.npm`—, no la carpeta de
/// un proyecto: tiene configuración escrita a mano que no se regenera. Un
/// proyecto de verdad está siempre dentro de su propio directorio, así que su
/// carpeta de dependencias queda al menos dos niveles abajo.
pub fn es_hijo_directo(ruta: &Path, home: &Path) -> bool {
    ruta.parent() == Some(home)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un árbol de mentira con proyectos de verdad.
    struct Arbol(PathBuf);

    impl Arbol {
        fn nuevo(nombre: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("vsk-proy-{nombre}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("crear la raíz");
            Self(dir)
        }
        fn dir(&self, rel: &str) -> PathBuf {
            let r = self.0.join(rel);
            std::fs::create_dir_all(&r).expect("crear directorio");
            r
        }
        fn archivo(&self, rel: &str) {
            let r = self.0.join(rel);
            if let Some(p) = r.parent() {
                std::fs::create_dir_all(p).expect("crear el padre");
            }
            std::fs::write(&r, b"x").expect("escribir");
        }
        fn nombres(&self, profundidad: usize) -> Vec<String> {
            let mut v: Vec<String> = candidatas(&self.0, profundidad)
                .into_iter()
                .map(|(r, _)| {
                    r.strip_prefix(&self.0)
                        .unwrap_or(&r)
                        .to_string_lossy()
                        .into_owned()
                })
                .collect();
            v.sort();
            v
        }
    }

    impl Drop for Arbol {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn se_encuentra_lo_que_tiene_su_marca() {
        let a = Arbol::nuevo("marca");
        a.archivo("app/package.json");
        a.dir("app/node_modules");
        a.archivo("libreria/Cargo.toml");
        a.dir("libreria/target");

        assert_eq!(a.nombres(4), vec!["app/node_modules", "libreria/target"]);
    }

    #[test]
    fn sin_la_marca_no_cuenta() {
        // Éste es el caso que hace peligroso buscar por nombre: `target` es una
        // carpeta cualquiera si no hay un Cargo.toml al lado, y `build` puede ser
        // de cualquier cosa. Sin esta comprobación, «limpiar proyectos» se
        // llevaría el trabajo de alguien.
        let a = Arbol::nuevo("sinmarca");
        a.dir("disenio/target");
        a.archivo("disenio/nota.txt");
        a.dir("casa/build");

        assert!(a.nombres(4).is_empty(), "se ofreció algo sin marca");
    }

    #[test]
    fn no_se_baja_dentro_de_lo_encontrado() {
        // Un node_modules tiene cientos anidados. Ofrecerlos por separado no
        // sirve —se borra el de arriba y se van todos— y multiplica el tiempo.
        let a = Arbol::nuevo("anidado");
        a.archivo("app/package.json");
        a.archivo("app/node_modules/x/package.json");
        a.dir("app/node_modules/x/node_modules");

        assert_eq!(a.nombres(8), vec!["app/node_modules"]);
    }

    #[test]
    fn los_ocultos_no_se_recorren_pero_los_que_son_patron_si() {
        let a = Arbol::nuevo("ocultos");
        // Dentro de un oculto no se busca: ahí están las cachés, que tienen su
        // propia tarea.
        a.archivo(".cache/algo/package.json");
        a.dir(".cache/algo/node_modules");
        // Pero `.venv` es un patrón y se encuentra.
        a.dir("proyecto/.venv");

        assert_eq!(a.nombres(6), vec!["proyecto/.venv"]);
    }

    #[test]
    fn la_profundidad_corta() {
        let a = Arbol::nuevo("hondo");
        a.archivo("a/b/c/d/package.json");
        a.dir("a/b/c/d/node_modules");

        assert!(a.nombres(3).is_empty(), "se pasó del límite");
        assert_eq!(a.nombres(5), vec!["a/b/c/d/node_modules"]);
    }

    #[test]
    fn un_enlace_no_se_confunde_con_un_directorio() {
        // Seguir enlaces puede salir del árbol y volver a entrar en ciclo, y lo
        // que se ofrece borrar tiene que estar donde dice.
        let a = Arbol::nuevo("enlace");
        a.archivo("real/package.json");
        a.dir("real/node_modules");
        a.dir("otro");
        #[cfg(unix)]
        std::os::unix::fs::symlink(a.0.join("real"), a.0.join("otro/enlace")).expect("enlace");

        let hallados = a.nombres(6);
        assert!(hallados.contains(&"real/node_modules".to_string()));
        assert!(
            !hallados.iter().any(|n| n.contains("enlace")),
            "se siguió un enlace: {hallados:?}"
        );
    }

    // ── La regla de seguridad ───────────────────────────────────────────────

    fn patron(nombre: &str) -> &'static Patron {
        patron_de(nombre).expect("el patrón existe")
    }

    #[test]
    fn dentro_de_un_repo_solo_se_ofrece_lo_ignorado() {
        // Es la regla que impide romper un repositorio. `vendor` commiteado es
        // práctica normal en Go: borrarlo no recupera espacio, saca código.
        let vendor = patron("vendor");
        assert!(se_puede_ofrecer(vendor, true, true));
        assert!(
            !se_puede_ofrecer(vendor, true, false),
            "se ofreció una carpeta versionada"
        );

        // Y vale también para las inequívocas: un node_modules commiteado —raro,
        // pero pasa— tampoco se toca.
        let node = patron("node_modules");
        assert!(!se_puede_ofrecer(node, true, false));
    }

    #[test]
    fn fuera_de_un_repo_solo_las_inequivocas() {
        // Sin git no hay nada que confirme que sobra, así que sólo los nombres
        // que no dejan lugar a dudas.
        assert!(se_puede_ofrecer(patron("node_modules"), false, false));
        assert!(se_puede_ofrecer(patron("target"), false, false));
        assert!(!se_puede_ofrecer(patron("dist"), false, false));
        assert!(!se_puede_ofrecer(patron("vendor"), false, false));
    }

    // ── La ubicación ────────────────────────────────────────────────────────

    #[test]
    fn el_home_de_gradle_no_se_ofrece() {
        // **Éste es el bug que se escapó.** `$HOME/.gradle` es el home de Gradle:
        // tiene el `gradle.properties` global y los scripts de `init.d`, que no
        // los regenera nadie. Con `marca: None` e `inequivoca: true` se ofrecía
        // para borrar, y borrarlo es perder configuración escrita a mano.
        //
        // El test anterior de «ninguna carpeta de datos» no lo atrapaba porque
        // `.gradle` no es una carpeta de datos por **nombre**: lo es por
        // **ubicación**.
        let a = Arbol::nuevo("gradlehome");
        a.dir(".gradle");
        a.dir(".gradle/init.d");
        a.archivo(".gradle/gradle.properties");

        assert!(
            a.nombres(4).is_empty(),
            "se ofreció el home de Gradle: {:?}",
            a.nombres(4)
        );
    }

    #[test]
    fn el_gradle_de_un_proyecto_si_se_ofrece() {
        // Con un archivo de proyecto al lado y a un nivel de profundidad, sí.
        let a = Arbol::nuevo("gradleproy");
        a.archivo("app/build.gradle.kts");
        a.dir("app/.gradle");

        assert_eq!(a.nombres(4), vec!["app/.gradle"]);
    }

    #[test]
    fn alcanza_una_de_las_marcas() {
        // Un proyecto Gradle se reconoce por `build.gradle` **o** por
        // `settings.gradle.kts`, no por los dos. Con una marca única, la mitad de
        // los proyectos quedaba afuera.
        for marca in [
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "gradlew",
        ] {
            let a = Arbol::nuevo("marcas");
            a.archivo(&format!("p/{marca}"));
            a.dir("p/.gradle");
            assert_eq!(a.nombres(4), vec!["p/.gradle"], "con {marca}");
        }
    }

    #[test]
    fn ninguna_hija_directa_de_la_raiz_se_ofrece() {
        // La regla es estructural, no una lista de excepciones: vale para
        // cualquier patrón, incluidos los que no piden marca. Así queda cubierto
        // `.cargo` o `.npm` el día que alguien los agregue.
        let a = Arbol::nuevo("hijas");
        a.dir(".venv");
        a.dir("__pycache__");
        a.archivo("package.json");
        a.dir("node_modules");

        assert!(
            a.nombres(4).is_empty(),
            "se ofreció una hija directa del home: {:?}",
            a.nombres(4)
        );
    }

    #[test]
    fn un_nivel_mas_abajo_si() {
        // Y que la regla no se pase de estricta: lo de un proyecto de verdad tiene
        // que seguir apareciendo.
        let a = Arbol::nuevo("unnivel");
        a.archivo("proyecto/package.json");
        a.dir("proyecto/node_modules");

        assert_eq!(a.nombres(4), vec!["proyecto/node_modules"]);
    }

    #[test]
    fn es_hijo_directo_compara_el_padre() {
        assert!(es_hijo_directo(
            Path::new("/home/pato/.gradle"),
            Path::new("/home/pato")
        ));
        assert!(!es_hijo_directo(
            Path::new("/home/pato/app/.gradle"),
            Path::new("/home/pato")
        ));
    }

    #[test]
    fn el_nombre_del_proyecto_es_el_del_padre() {
        assert_eq!(
            proyecto_de(Path::new("/home/pato/VasakOS/vasak-shot/src-tauri/target")),
            "src-tauri"
        );
        assert_eq!(proyecto_de(Path::new("/home/pato/app/node_modules")), "app");
    }

    #[test]
    fn los_patrones_no_repiten_nombre() {
        // `patron_de` devuelve el primero que coincide, así que un nombre
        // repetido haría que el segundo no se aplicara nunca — en silencio.
        let mut nombres: Vec<&str> = PATRONES.iter().map(|p| p.nombre).collect();
        let antes = nombres.len();
        nombres.sort_unstable();
        nombres.dedup();
        assert_eq!(nombres.len(), antes, "hay un nombre repetido en PATRONES");
    }

    #[test]
    fn ninguna_carpeta_de_datos_esta_entre_los_patrones() {
        // Un patrón de más acá es espacio del usuario borrado sin aviso. Estos
        // nombres aparecen en cualquier casa y no los regenera nadie.
        for prohibido in [
            "Documentos", "Documents", "Escritorio", "Desktop", "Imágenes", "Pictures", "src",
            "Descargas", "Downloads", "Música", "Music", "Vídeos", "Videos", ".ssh", ".gnupg",
        ] {
            assert!(
                patron_de(prohibido).is_none(),
                "«{prohibido}» está entre los patrones"
            );
        }
    }
}
