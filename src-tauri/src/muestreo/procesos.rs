//! Los procesos, y qué de todo eso es «una aplicación».
//!
//! Una lista cruda de `/proc` son cientos de entradas donde casi todo es del
//! sistema: hilos del kernel, servicios, ayudantes. Alguien que abre esto para ver
//! qué está consumiendo su máquina no quiere buscar Firefox entre trescientas
//! filas — quiere ver Firefox.

/// Un proceso, con lo que hace falta para mostrarlo y ordenarlo.
#[derive(Debug, Clone, PartialEq)]
pub struct Proceso {
    pub pid: u32,
    /// El nombre del ejecutable, sin ruta ni argumentos.
    pub nombre: String,
    /// Jiffies de CPU acumulados: `utime` más `stime`.
    ///
    /// Como el de `/proc/stat`, sólo sirve como diferencia entre dos lecturas.
    pub jiffies: u64,
    /// Memoria residente, en páginas. Se convierte a bytes con el tamaño de
    /// página, que no siempre es 4 KiB.
    pub paginas: u64,
}

/// Lee un `/proc/<pid>/stat`.
///
/// El nombre viene entre paréntesis y **puede contener espacios y paréntesis** —
/// un proceso llamado `(mi programa)` es legal—. Por eso no se puede partir la
/// línea por espacios: hay que buscar el **último** paréntesis de cierre y contar
/// los campos desde ahí. Partiendo por espacios, cualquier proceso con un espacio
/// en el nombre corre todos los campos y la CPU y la memoria salen de otro lugar.
pub fn proceso_de(pid: u32, stat: &str) -> Option<Proceso> {
    let abre = stat.find('(')?;
    let cierra = stat.rfind(')')?;
    if cierra <= abre {
        return None;
    }
    let nombre = stat[abre + 1..cierra].to_string();

    let resto: Vec<&str> = stat[cierra + 1..].split_whitespace().collect();
    // Después del nombre el campo 0 es `state`; utime es el 11 y stime el 12
    // contando desde ahí, y rss el 21.
    if resto.len() < 22 {
        return None;
    }
    let utime: u64 = resto[11].parse().ok()?;
    let stime: u64 = resto[12].parse().ok()?;
    let rss: u64 = resto[21].parse().ok()?;

    Some(Proceso {
        pid,
        nombre,
        jiffies: utime + stime,
        paginas: rss,
    })
}

/// El nombre de una aplicación a partir de su línea de comandos.
///
/// `/proc/<pid>/cmdline` trae los argumentos separados por bytes nulos, y se usa
/// el primero sin la ruta: `/usr/lib/firefox/firefox` se muestra como `firefox`,
/// que es como la persona lo llama.
///
/// # Por qué además se parte por espacios
///
/// Chromium y las aplicaciones de Electron **reescriben su `argv`** como un único
/// bloque con espacios en lugar de nulos. Partiendo sólo por nulos, el «nombre»
/// que sale es la línea de comandos completa: en esta máquina apareció una fila
/// llamada `app.asar --enable-sandbox --ozone-platform=wayland --lang=es-419 …`
/// de dos mil caracteres, que rompía la tabla. Lo encontró el sondeo contra el
/// `/proc` real, no un test escrito de antemano.
///
/// Si está vacía —los hilos del kernel no tienen línea de comandos— se devuelve
/// `None` y quien llama usa el nombre de `stat`.
pub fn nombre_de_cmdline(cmdline: &[u8]) -> Option<String> {
    let primero = cmdline.split(|b| *b == 0).find(|p| !p.is_empty())?;
    let texto = String::from_utf8_lossy(primero);
    // El primer token, no el bloque entero: ver arriba.
    let ejecutable = texto.split_whitespace().next()?;
    let sin_ruta = ejecutable.rsplit('/').next()?;
    if sin_ruta.is_empty() {
        return None;
    }
    Some(sin_ruta.to_string())
}

/// Si un proceso es del kernel y no algo que la persona haya abierto.
///
/// Los hilos del kernel no tienen línea de comandos, y en `stat` su nombre viene
/// entre corchetes. Mostrarlos llena la lista de `kworker` y `ksoftirqd` que no se
/// pueden cerrar ni interesan.
pub fn es_del_kernel(nombre: &str, cmdline_vacia: bool) -> bool {
    cmdline_vacia || (nombre.starts_with('[') && nombre.ends_with(']'))
}

/// Junta los procesos que comparten nombre en una sola fila.
///
/// Un navegador son ocho o diez procesos: el principal, uno por pestaña, la GPU.
/// Verlos por separado no dice nada útil —ninguno es «el consumo de Firefox»— y
/// hace imposible comparar de un vistazo qué aplicación está pesando.
pub fn agrupar(procesos: Vec<Proceso>) -> Vec<Proceso> {
    let mut agrupados: Vec<Proceso> = Vec::new();

    for p in procesos {
        if let Some(existente) = agrupados.iter_mut().find(|e| e.nombre == p.nombre) {
            existente.jiffies += p.jiffies;
            existente.paginas += p.paginas;
            // Se conserva el pid más bajo: en un árbol de procesos suele ser el
            // padre, que es el que tiene sentido cerrar si alguien lo pide.
            existente.pid = existente.pid.min(p.pid);
        } else {
            agrupados.push(p);
        }
    }

    agrupados
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un `/proc/<pid>/stat` con el formato real: 52 campos después del nombre.
    fn stat_de(nombre: &str, utime: u64, stime: u64, rss: u64) -> String {
        let mut campos = vec!["S".to_string()];
        for i in 1..52 {
            campos.push(match i {
                11 => utime.to_string(),
                12 => stime.to_string(),
                21 => rss.to_string(),
                _ => "0".to_string(),
            });
        }
        format!("1234 ({nombre}) {}", campos.join(" "))
    }

    #[test]
    fn se_leen_la_cpu_y_la_memoria() {
        let p = proceso_de(1234, &stat_de("firefox", 500, 100, 2048)).expect("parsea");
        assert_eq!(p.nombre, "firefox");
        assert_eq!(p.jiffies, 600);
        assert_eq!(p.paginas, 2048);
    }

    #[test]
    fn un_nombre_con_espacios_no_corre_los_campos() {
        // Un proceso puede llamarse «mi programa». Partiendo la línea por
        // espacios, todos los campos se corren y la CPU y la memoria salen de otro
        // lugar — sin fallar, mostrando números de otra columna.
        let p = proceso_de(1, &stat_de("mi programa", 500, 100, 2048)).expect("parsea");
        assert_eq!(p.nombre, "mi programa");
        assert_eq!(p.jiffies, 600, "los campos siguen en su lugar");
        assert_eq!(p.paginas, 2048);
    }

    #[test]
    fn un_nombre_con_parentesis_tampoco() {
        // Y por eso se busca el **último** paréntesis de cierre y no el primero.
        let p = proceso_de(1, &stat_de("raro (de verdad)", 7, 3, 99)).expect("parsea");
        assert_eq!(p.nombre, "raro (de verdad)");
        assert_eq!(p.jiffies, 10);
        assert_eq!(p.paginas, 99);
    }

    #[test]
    fn un_stat_incompleto_no_paniquea() {
        // Pasa cuando el proceso muere entre que se lista /proc y se lee su stat.
        assert_eq!(proceso_de(1, ""), None);
        assert_eq!(proceso_de(1, "1234 (algo) S 0 0"), None);
        assert_eq!(proceso_de(1, "sin parentesis"), None);
        assert_eq!(proceso_de(1, "1234 ) invertido ("), None);
    }

    #[test]
    fn el_nombre_de_cmdline_viene_sin_ruta() {
        // La persona lo llama «firefox», no «/usr/lib/firefox/firefox».
        assert_eq!(
            nombre_de_cmdline(b"/usr/lib/firefox/firefox\0--new-window\0"),
            Some("firefox".to_string())
        );
        assert_eq!(nombre_de_cmdline(b"bash\0"), Some("bash".to_string()));
    }

    #[test]
    fn una_cmdline_de_electron_no_trae_la_linea_entera() {
        // Chromium y Electron reescriben su argv como un bloque con espacios en
        // lugar de nulos. Partiendo sólo por nulos, el nombre salía con dos mil
        // caracteres de banderas y rompía la tabla. Apareció en el /proc real de
        // la máquina, no en un test escrito de antemano.
        let cruda = b"/opt/claude/app.asar --enable-sandbox --ozone-platform=wayland --lang=es-419\0";
        assert_eq!(nombre_de_cmdline(cruda), Some("app.asar".to_string()));

        // Y con la ruta completa delante, que es el caso de Chromium.
        let chromium = b"/usr/lib/chromium/chromium --type=renderer --enable-features=Algo\0";
        assert_eq!(nombre_de_cmdline(chromium), Some("chromium".to_string()));
    }

    #[test]
    fn una_cmdline_vacia_no_da_nombre() {
        // Los hilos del kernel no tienen línea de comandos.
        assert_eq!(nombre_de_cmdline(b""), None);
        assert_eq!(nombre_de_cmdline(b"\0\0\0"), None);
    }

    #[test]
    fn los_hilos_del_kernel_se_reconocen() {
        // Mostrarlos llena la lista de `kworker` y `ksoftirqd`, que no se pueden
        // cerrar ni interesan a nadie que quiera saber qué está consumiendo.
        assert!(es_del_kernel("[kworker/0:1]", true));
        assert!(es_del_kernel("kthreadd", true), "sin cmdline alcanza");
        assert!(es_del_kernel("[ksoftirqd/0]", false), "los corchetes también");
        assert!(!es_del_kernel("firefox", false));
    }

    #[test]
    fn los_procesos_del_mismo_nombre_se_suman() {
        // Un navegador son ocho o diez procesos. Verlos por separado no dice nada
        // útil: ninguno es «el consumo de Firefox».
        let procesos = vec![
            Proceso { pid: 300, nombre: "firefox".into(), jiffies: 100, paginas: 1000 },
            Proceso { pid: 100, nombre: "firefox".into(), jiffies: 50, paginas: 500 },
            Proceso { pid: 200, nombre: "code".into(), jiffies: 70, paginas: 700 },
        ];
        let juntos = agrupar(procesos);
        assert_eq!(juntos.len(), 2);
        let ff = juntos.iter().find(|p| p.nombre == "firefox").unwrap();
        assert_eq!(ff.jiffies, 150);
        assert_eq!(ff.paginas, 1500);
        assert_eq!(ff.pid, 100, "se conserva el pid más bajo, que suele ser el padre");
    }
}
