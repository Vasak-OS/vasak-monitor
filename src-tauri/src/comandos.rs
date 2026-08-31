//! Lo que el frontend puede pedir.
//!
//! Todo el trabajo pasa por acá y no por el frontend: lo que cruza el IPC son
//! números, no listas de miles de líneas para procesar en JavaScript. En un monitor
//! eso importa el doble, porque es la aplicación que muestra el consumo.

use crate::muestreo::{cpu, discos, memoria, procesos, red, ventanas};
use crate::{limpieza, proyectos, registros, servicios};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// Lo que hace falta recordar entre dos muestras.
///
/// La CPU y la red **sólo existen como diferencia**, así que sin guardar la lectura
/// anterior no hay nada que informar. Guardarlo del lado del frontend obligaría a
/// mandarle los contadores crudos y hacer la cuenta en JavaScript.
struct Anterior {
    cpu: cpu::Contadores,
    red: red::Acumulado,
    momento: Instant,
}

fn anterior() -> &'static Mutex<Option<Anterior>> {
    static A: OnceLock<Mutex<Option<Anterior>>> = OnceLock::new();
    A.get_or_init(|| Mutex::new(None))
}

/// Los jiffies de CPU de cada proceso en la muestra anterior, para poder informar
/// su uso como diferencia igual que el total.
fn jiffies_previos() -> &'static Mutex<std::collections::HashMap<u32, u64>> {
    static J: OnceLock<Mutex<std::collections::HashMap<u32, u64>>> = OnceLock::new();
    J.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn leer(ruta: &str) -> String {
    std::fs::read_to_string(ruta).unwrap_or_default()
}

// ── Recursos ───────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct Disco {
    pub punto: String,
    pub tipo: String,
    pub total: u64,
    pub usado: u64,
}

#[derive(serde::Serialize)]
pub struct Recursos {
    /// `None` en la primera muestra: no hay con qué comparar, y devolver 0
    /// dibujaría una caída a cero que no ocurrió.
    pub cpu: Option<f32>,
    pub nucleos: usize,
    pub ram_usada: u64,
    pub ram_total: u64,
    pub ram_cache: u64,
    pub swap: Option<f32>,
    pub bajada: Option<f64>,
    pub subida: Option<f64>,
    pub discos: Vec<Disco>,
}

/// El espacio de un sistema de archivos, con `statvfs`.
fn espacio_de(punto: &str) -> Option<(u64, u64)> {
    let ruta = std::ffi::CString::new(punto).ok()?;
    let mut datos: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(ruta.as_ptr(), &mut datos) } != 0 {
        return None;
    }
    let bloque = datos.f_frsize as u64;
    let total = datos.f_blocks as u64 * bloque;
    // `f_bavail` y no `f_bfree`: el segundo incluye los bloques reservados para
    // root, que no están disponibles para la persona. Usarlo informa más espacio
    // libre del que se puede usar.
    let libre = datos.f_bavail as u64 * bloque;
    Some((total, total.saturating_sub(libre)))
}

#[tauri::command]
pub fn recursos() -> Recursos {
    let stat = leer("/proc/stat");
    let contadores = cpu::contadores_de(&stat).unwrap_or_default();
    let acumulado = red::acumulado_de(&leer("/proc/net/dev"));
    let ahora = Instant::now();

    let (uso_cpu, caudal) = {
        let mut guardia = anterior().lock().unwrap_or_else(|e| e.into_inner());
        let previo = guardia.as_ref();
        let uso = previo.and_then(|p| contadores.uso_desde(p.cpu));
        let caudal = previo.and_then(|p| acumulado.caudal_desde(p.red, ahora - p.momento));
        *guardia = Some(Anterior {
            cpu: contadores,
            red: acumulado,
            momento: ahora,
        });
        (uso, caudal)
    };

    let m = memoria::memoria_de(&leer("/proc/meminfo"));

    let discos = discos::montajes_de(&leer("/proc/mounts"))
        .into_iter()
        .filter_map(|mo| {
            let (total, usado) = espacio_de(&mo.punto)?;
            // Un sistema de archivos de cero bytes no es un disco: pasa con
            // montajes especiales que se colaron, y mostrarlo da una división por
            // cero en la barra.
            if total == 0 {
                return None;
            }
            Some(Disco {
                punto: mo.punto,
                tipo: mo.tipo,
                total,
                usado,
            })
        })
        .collect();

    Recursos {
        cpu: uso_cpu,
        nucleos: cpu::nucleos_en(&stat),
        ram_usada: m.en_uso_kib() * 1024,
        ram_total: m.total_kib * 1024,
        ram_cache: m.cache_kib * 1024,
        swap: m.uso_de_swap(),
        bajada: caudal.map(|c| c.bajada),
        subida: caudal.map(|c| c.subida),
        discos,
    }
}

// ── Aplicaciones ───────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct Aplicacion {
    pub pid: u32,
    pub nombre: String,
    pub memoria: u64,
    /// `None` en la primera muestra, por lo mismo que la CPU total.
    pub cpu: Option<f32>,
    /// Si tiene una conexión gráfica abierta con el compositor.
    ///
    /// Es lo que separa lo que la persona abrió de lo que corre por su cuenta. Ver
    /// `muestreo::ventanas` para qué mide exactamente y qué no.
    pub con_ventana: bool,
}

#[tauri::command]
pub fn aplicaciones() -> Vec<Aplicacion> {
    let pagina = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    let mut lista: Vec<procesos::Proceso> = Vec::new();

    let Ok(entradas) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };

    for entrada in entradas.flatten() {
        let Ok(pid) = entrada.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        // El proceso puede morir entre listar /proc y leer su stat; se saltea.
        let Ok(stat) = std::fs::read_to_string(entrada.path().join("stat")) else {
            continue;
        };
        let Some(mut p) = procesos::proceso_de(pid, &stat) else {
            continue;
        };
        let cmdline = std::fs::read(entrada.path().join("cmdline")).unwrap_or_default();
        let vacia = cmdline.iter().all(|b| *b == 0);
        if procesos::es_del_kernel(&p.nombre, vacia) {
            continue;
        }
        if let Some(nombre) = procesos::nombre_de_cmdline(&cmdline) {
            p.nombre = nombre;
        }
        lista.push(p);
    }

    // Quién habla con el compositor. Se pregunta una vez por muestra y no por
    // proceso: son 200 procesos y una sola pasada de `ss`.
    let con_ventana = ventanas::detectar();

    // La marca se propaga al grupo: si **cualquiera** de los procesos de una
    // aplicación tiene ventana, la aplicación la tiene. Un navegador abre la
    // ventana en un proceso y decodifica en otros, y mirando sólo el pid que queda
    // como representante del grupo la mitad de las aplicaciones se clasificarían
    // mal según qué proceso arrancó primero.
    let grupos_con_ventana: std::collections::HashSet<String> = lista
        .iter()
        .filter(|p| con_ventana.contains(&p.pid))
        .map(|p| p.nombre.clone())
        .collect();

    let agrupados = procesos::agrupar(lista);

    // El uso de CPU de cada aplicación, como diferencia con la muestra anterior.
    let mut previos = jiffies_previos().lock().unwrap_or_else(|e| e.into_inner());
    let mut salida: Vec<Aplicacion> = agrupados
        .iter()
        .map(|p| {
            let cpu = previos.get(&p.pid).and_then(|antes| {
                let delta = p.jiffies.checked_sub(*antes)?;
                // Sin el total de jiffies transcurridos no se puede sacar un
                // porcentaje comparable; se informa el delta relativo al reloj.
                Some(delta as f32)
            });
            Aplicacion {
                pid: p.pid,
                nombre: p.nombre.clone(),
                memoria: p.paginas * pagina,
                cpu,
                con_ventana: grupos_con_ventana.contains(&p.nombre),
            }
        })
        .collect();

    *previos = agrupados.iter().map(|p| (p.pid, p.jiffies)).collect();

    // De mayor a menor memoria: es el orden en que alguien busca qué está pesando.
    salida.sort_by_key(|a| std::cmp::Reverse(a.memoria));
    salida
}

/// Cierra una aplicación, pidiéndole primero que se cierre sola.
///
/// `SIGTERM` y no `SIGKILL`: lo primero le da al programa la chance de guardar lo
/// que tenga abierto. Un monitor que mata sin avisar pierde trabajo de la persona,
/// y eso no se puede deshacer.
#[tauri::command]
pub fn cerrar(pid: u32) -> Result<(), String> {
    // Al grupo y no al proceso: un navegador son diez procesos y matar sólo el
    // padre deja los hijos huérfanos consumiendo lo mismo.
    let resultado = unsafe { libc::kill(-(pid as i32), libc::SIGTERM) };
    if resultado == 0 {
        return Ok(());
    }
    // Sin grupo propio, al proceso solo.
    if unsafe { libc::kill(pid as i32, libc::SIGTERM) } == 0 {
        Ok(())
    } else {
        Err(format!("no se pudo cerrar el proceso {pid}"))
    }
}

// ── Servicios ──────────────────────────────────────────────

#[tauri::command]
pub fn lista_de_servicios() -> Vec<servicios::Servicio> {
    let mut todos = Vec::new();

    for (del_usuario, ambito) in [(true, "--user"), (false, "--system")] {
        let salida = std::process::Command::new("systemctl")
            .args([ambito, "list-units", "--type=service", "--plain", "--no-legend", "--all"])
            .output();
        if let Ok(s) = salida {
            todos.extend(servicios::servicios_de(
                &String::from_utf8_lossy(&s.stdout),
                del_usuario,
            ));
        }
    }

    servicios::ordenar(todos)
}

/// Arranca, para o reinicia un servicio.
///
/// Los del usuario van directo; los del sistema por `pkexec`, que lo pregunta con
/// el agente de polkit que ya está corriendo. Hacerlo al revés —intentar sin
/// privilegios y fallar— daría un error en lugar de una pregunta.
#[tauri::command]
pub fn accion_de_servicio(unidad: String, accion: String, del_usuario: bool) -> Result<(), String> {
    if !matches!(accion.as_str(), "start" | "stop" | "restart") {
        return Err(format!("acción desconocida: {accion}"));
    }

    let salida = if del_usuario {
        std::process::Command::new("systemctl")
            .args(["--user", &accion, &unidad])
            .output()
    } else {
        std::process::Command::new("pkexec")
            .args(["systemctl", &accion, &unidad])
            .output()
    };

    match salida {
        Ok(s) if s.status.success() => Ok(()),
        Ok(s) => {
            let motivo = String::from_utf8_lossy(&s.stderr).trim().to_string();
            Err(if motivo.is_empty() {
                format!("{accion} sobre {unidad} falló")
            } else {
                motivo
            })
        }
        Err(e) => Err(format!("no se pudo ejecutar systemctl: {e}")),
    }
}

// ── Limpieza ───────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct Recuperable {
    pub tarea: limpieza::Tarea,
    /// Bytes, o `None` para las tareas de memoria, que no liberan disco.
    pub bytes: Option<u64>,
    pub necesita_autenticar: bool,
}

fn du_de(ruta: &std::path::Path) -> Option<u64> {
    let salida = std::process::Command::new("du")
        .arg("-sb")
        .arg(ruta)
        .output()
        .ok()?;
    limpieza::bytes_de_du(&String::from_utf8_lossy(&salida.stdout))
}

#[tauri::command]
pub fn recuperable() -> Vec<Recuperable> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut lista = Vec::new();

    for tarea in [
        limpieza::Tarea::CacheDeUsuario,
        limpieza::Tarea::Papelera,
        limpieza::Tarea::CacheDePaquetes,
        limpieza::Tarea::DiarioViejo,
        limpieza::Tarea::PaquetesHuerfanos,
        limpieza::Tarea::SwapALaMemoria,
        limpieza::Tarea::CacheDelKernel,
    ] {
        let bytes = if !tarea.recupera_disco() {
            None
        } else if let Some(ruta) = limpieza::ruta_de(tarea, &home) {
            du_de(&ruta)
        } else {
            match tarea {
                limpieza::Tarea::CacheDePaquetes => du_de(std::path::Path::new("/var/cache/pacman/pkg")),
                limpieza::Tarea::DiarioViejo => std::process::Command::new("journalctl")
                    .arg("--disk-usage")
                    .output()
                    .ok()
                    .and_then(|s| limpieza::bytes_del_diario(&String::from_utf8_lossy(&s.stdout))),
                // Los huérfanos se miden en cantidad, no en bytes: cuánto ocupan
                // exige consultar cada uno, y el número que importa es cuántos son.
                limpieza::Tarea::PaquetesHuerfanos => None,
                _ => None,
            }
        };

        lista.push(Recuperable {
            tarea,
            bytes,
            necesita_autenticar: tarea.necesita_autenticar(),
        });
    }

    lista
}

/// Ejecuta varias tareas seguidas.
///
/// Devuelve qué falló en lugar de cortar en la primera: si la caché de paquetes no
/// se puede tocar, la papelera y el diario igual se limpian. Cortar dejaría el
/// trabajo a medias sin decir cuánto se hizo.
///
/// Las que piden autenticar se agrupan al final para que polkit pregunte una sola
/// vez seguida en lugar de intercalar diálogos entre tareas silenciosas.
#[tauri::command]
pub fn limpiar_todo(tareas: Vec<limpieza::Tarea>) -> Vec<String> {
    let mut ordenadas = tareas;
    ordenadas.sort_by_key(|t| t.necesita_autenticar());

    ordenadas
        .into_iter()
        .filter_map(|t| limpiar(t).err().map(|e| format!("{t:?}: {e}")))
        .collect()
}

#[tauri::command]
pub fn limpiar(tarea: limpieza::Tarea) -> Result<(), String> {
    let home = std::env::var("HOME").unwrap_or_default();

    let (programa, argumentos): (&str, Vec<String>) = match tarea {
        limpieza::Tarea::CacheDeUsuario | limpieza::Tarea::Papelera => {
            let ruta = limpieza::ruta_de(tarea, &home)
                .ok_or_else(|| "sin ruta para esa tarea".to_string())?;
            // El contenido, no la carpeta: borrar `~/.cache` entero hace que los
            // programas que la esperan fallen hasta volver a crearla.
            (
                "sh",
                vec![
                    "-c".into(),
                    format!("rm -rf -- {}/..?* {}/.[!.]* {}/*", ruta.display(), ruta.display(), ruta.display()),
                ],
            )
        }
        limpieza::Tarea::CacheDePaquetes => ("pkexec", vec!["paccache".into(), "-rk1".into()]),
        limpieza::Tarea::DiarioViejo => (
            "pkexec",
            vec!["journalctl".into(), "--vacuum-time=7d".into()],
        ),
        limpieza::Tarea::PaquetesHuerfanos => (
            "pkexec",
            vec![
                "sh".into(),
                "-c".into(),
                // Sin huérfanos, `pacman -Rns` sin argumentos falla; el `||` deja
                // que eso cuente como éxito en lugar de mostrar un error.
                "pacman -Qtdq | pacman -Rns - --noconfirm || true".into(),
            ],
        ),
        limpieza::Tarea::SwapALaMemoria => (
            "pkexec",
            vec!["sh".into(), "-c".into(), "swapoff -a && swapon -a".into()],
        ),
        limpieza::Tarea::CacheDelKernel => (
            "pkexec",
            vec![
                "sh".into(),
                "-c".into(),
                "sync && echo 3 > /proc/sys/vm/drop_caches".into(),
            ],
        ),
    };

    let salida = std::process::Command::new(programa)
        .args(&argumentos)
        .output()
        .map_err(|e| format!("no se pudo ejecutar {programa}: {e}"))?;

    if salida.status.success() {
        Ok(())
    } else {
        let motivo = String::from_utf8_lossy(&salida.stderr).trim().to_string();
        Err(if motivo.is_empty() {
            "la limpieza no se completó".to_string()
        } else {
            motivo
        })
    }
}

// ── Registros ──────────────────────────────────────────────

/// Los orígenes del ecosistema que alguna vez escribieron en el diario.
///
/// Se pregunta con `journalctl -F <campo>`, que enumera los valores de un campo
/// usando el índice del diario. Leer las entradas y juntar los orígenes también
/// serviría, pero sólo vería las últimas N líneas: una app que escribió al
/// arrancar y se quedó callada no aparecería en el selector.
///
/// No se acota al arranque actual porque `-F` no admite `-b`; ver
/// `registros::argumentos_de_campo`.
fn origenes_presentes() -> Vec<String> {
    let mut texto = String::new();
    for (ambito, unidad) in [("--user", "_SYSTEMD_USER_UNIT"), ("--system", "_SYSTEMD_UNIT")] {
        for campo in ["SYSLOG_IDENTIFIER", "_COMM", unidad] {
            if let Ok(s) = std::process::Command::new("journalctl")
                .args(registros::argumentos_de_campo(ambito, campo))
                .output()
            {
                texto.push_str(&String::from_utf8_lossy(&s.stdout));
                texto.push('\n');
            }
        }
    }
    registros::origenes_de(&texto)
}

/// El catálogo del selector de apps de la pantalla de registros.
#[tauri::command]
pub fn apps_del_diario() -> Vec<registros::AppDelDiario> {
    registros::catalogo(&origenes_presentes())
}

/// Las entradas del diario, filtradas por app.
///
/// `app` vacío es el ecosistema entero, `"*"` es el diario completo —VasakOS y lo
/// demás— y cualquier otro valor es una sola app. El filtro va a `journalctl` y no
/// se hace acá con los resultados: con `-n 500` sin filtrar, las líneas de una app
/// tranquila las tapan las del resto y la pantalla queda vacía aunque el diario
/// las tenga.
#[tauri::command]
pub fn registros_de_vasakos(
    solo_problemas: bool,
    cantidad: u32,
    app: Option<String>,
) -> Vec<registros::Entrada> {
    let pedidas = cantidad.clamp(20, 2000);
    // Se lee de más cuando hay que filtrar: los errores que el programa marca en
    // el texto son informativos para el diario, así que `-p err` los perdería.
    let a_leer = registros::a_leer(pedidas, solo_problemas).to_string();
    let pedido = app.unwrap_or_default();

    // Un id inválido se trata como «todo el ecosistema» en lugar de pasarse: un
    // valor que empiece con `-` sería una opción para `journalctl`.
    let coincidencias = if pedido == registros::TODO_EL_SISTEMA {
        Vec::new()
    } else if pedido != registros::TODO_EL_ECOSISTEMA && registros::id_valido(&pedido) {
        registros::coincidencias_de(&registros::nombres_de(&pedido))
    } else {
        registros::coincidencias_de(&origenes_presentes())
    };

    let mut entradas = Vec::new();
    for ambito in ["--user", "--system"] {
        // Sin `-p err`: el filtro por severidad se hace más abajo, con el nivel ya
        // corregido por lo que el propio programa escribió.
        let mut argumentos = vec![ambito, "-o", "json", "-n", &a_leer, "--no-pager", "-b"];
        // Las coincidencias van al final: `journalctl` las quiere después de las
        // opciones.
        argumentos.extend(coincidencias.iter().map(|c| c.as_str()));

        if let Ok(s) = std::process::Command::new("journalctl").args(&argumentos).output() {
            entradas.extend(registros::entradas_de(&String::from_utf8_lossy(&s.stdout)));
        }
    }

    // De lo más nuevo a lo más viejo: el problema que se busca acaba de pasar.
    entradas.sort_by_key(|e| std::cmp::Reverse(e.microsegundos));
    entradas.dedup_by(|a, b| a.microsegundos == b.microsegundos && a.mensaje == b.mensaje);

    if solo_problemas {
        entradas.retain(registros::Entrada::es_problema);
    }
    entradas.truncate(pedidas as usize);
    entradas
}

/// El desplazamiento horario, para mostrar las horas del diario en local.
#[tauri::command]
pub fn desplazamiento_horario() -> i64 {
    std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()
        .and_then(|s| String::from_utf8(s.stdout).ok())
        .and_then(|t| desplazamiento_desde_z(t.trim()))
        .unwrap_or(0)
}

/// Convierte un `+HHMM` como el de `date +%z` en segundos.
pub fn desplazamiento_desde_z(texto: &str) -> Option<i64> {
    if texto.len() != 5 {
        return None;
    }
    let signo = match texto.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let horas: i64 = texto[1..3].parse().ok()?;
    let minutos: i64 = texto[3..5].parse().ok()?;
    Some(signo * (horas * 3600 + minutos * 60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_desplazamiento_de_zona_se_lee_bien() {
        assert_eq!(desplazamiento_desde_z("+0000"), Some(0));
        assert_eq!(desplazamiento_desde_z("-0300"), Some(-10_800));
        assert_eq!(desplazamiento_desde_z("+0530"), Some(19_800));
        // Lo que no tiene esa forma no se toma por cero: un desplazamiento
        // inventado corre todas las horas del diario.
        assert_eq!(desplazamiento_desde_z(""), None);
        assert_eq!(desplazamiento_desde_z("0300"), None);
        assert_eq!(desplazamiento_desde_z("+03:00"), None);
    }

    #[test]
    fn solo_se_aceptan_las_tres_acciones_de_servicio() {
        // Sin esto, el nombre de la acción llega tal cual a `systemctl` y cualquier
        // cadena se convierte en un subcomando: «mask», «disable», o algo peor con
        // pkexec por delante.
        let error = accion_de_servicio("x.service".into(), "mask".into(), true);
        assert!(error.is_err());
        let error = accion_de_servicio("x.service".into(), "; rm -rf /".into(), true);
        assert!(error.is_err());
    }
}

// ── Carpetas de proyectos de desarrollo ────────────────────────────────

/// Si una carpeta cae dentro de un repositorio git, y si ese repositorio la ignora.
///
/// Una sola llamada contesta las dos cosas. `check-ignore` sale con 0 si la ruta
/// está ignorada, 1 si no, y 128 si no hay repositorio — que es justo la
/// distinción que hace falta y por eso se mira el código y no la salida.
fn estado_en_git(ruta: &std::path::Path) -> (bool, bool) {
    let Some(padre) = ruta.parent() else {
        return (false, false);
    };
    let salida = std::process::Command::new("git")
        .args(["-C", &padre.to_string_lossy(), "check-ignore", "-q", "--"])
        .arg(ruta)
        .output();

    match salida {
        Ok(s) => match s.status.code() {
            Some(0) => (true, true),   // hay repo y la ignora
            Some(1) => (true, false),  // hay repo y NO la ignora
            _ => (false, false),       // 128: no hay repositorio
        },
        // Sin git instalado no se puede comprobar nada, así que se responde «no
        // hay repo»: las inequívocas siguen ofreciéndose y el resto no.
        Err(_) => (false, false),
    }
}

/// Busca carpetas de dependencias y compilación que se puedan borrar.
///
/// Devuelve la lista **sin medir**. Medir es lo lento —un `du` sobre un `target`
/// de 40 GB tarda— y la lista sirve desde el primer momento sin los tamaños; los
/// pide después `medir_proyecto`, de a uno, así la interfaz los va completando en
/// lugar de quedarse en blanco hasta que estén todos.
/// Busca las carpetas, **fuera del hilo de la interfaz**.
///
/// El recorrido de `$HOME`, los `git check-ignore` y los `du` son todos
/// bloqueantes. Un comando sincrónico de Tauri corre en el hilo principal, así que
/// tal como estaba esto congelaba la ventana durante todo el escaneo —y un `du`
/// sobre un `target` de 40 GB no se mide en milisegundos—. Peor todavía: el
/// frontend pide los tamaños de a uno para irlos mostrando, y con el hilo tomado
/// no podía dibujar ninguno hasta el final.
///
/// `spawn_blocking` y no sólo `async`: marcar la función como asíncrona sin mover
/// el trabajo deja el bloqueo en el ejecutor, que es el mismo problema con otro
/// nombre.
#[tauri::command]
pub async fn proyectos_limpiables() -> Vec<proyectos::Hallazgo> {
    tauri::async_runtime::spawn_blocking(buscar_proyectos)
        .await
        .unwrap_or_default()
}

fn buscar_proyectos() -> Vec<proyectos::Hallazgo> {
    let Some(home) = home_real() else {
        return Vec::new();
    };

    // Seis niveles desde $HOME: `~/dev/organizacion/repo/paquete/` ya son cuatro.
    const PROFUNDIDAD: usize = 6;

    let mut hallazgos: Vec<proyectos::Hallazgo> = proyectos::candidatas(&home, PROFUNDIDAD)
        .into_iter()
        .filter(|(ruta, patron)| {
            let (en_repo, ignorada) = estado_en_git(ruta);
            proyectos::se_puede_ofrecer(patron, en_repo, ignorada)
        })
        .map(|(ruta, patron)| proyectos::Hallazgo {
            proyecto: proyectos::proyecto_de(&ruta),
            clase: patron.clase,
            ruta,
            bytes: None,
        })
        .collect();

    hallazgos.sort_by(|a, b| a.ruta.cmp(&b.ruta));
    hallazgos
}

/// El tamaño de una de las carpetas encontradas.
///
/// Se comprueba que sea una candidata antes de medirla. No es por el `du` —medir
/// no rompe nada— sino porque este comando y `borrar_proyecto` comparten la
/// validación, y tenerla en un solo lugar es lo que impide que se separen.
#[tauri::command]
pub async fn medir_proyecto(ruta: String) -> Option<u64> {
    tauri::async_runtime::spawn_blocking(move || {
        let ruta = validar_candidata(&ruta).ok()?;
        du_de(&ruta)
    })
    .await
    .unwrap_or(None)
}

/// Comprueba que una ruta sea de verdad una candidata a borrar.
///
/// **Esto es lo que impide que el frontend pida borrar cualquier cosa.** El
/// comando recibe un texto, y un texto puede ser `/home/pato/Documentos`. Así que
/// no se confía: se vuelve a verificar todo desde cero —que esté bajo `$HOME`, que
/// el nombre sea un patrón, que tenga su marca hermana y que git la ignore— en
/// lugar de creerle a la lista que se mandó antes.
/// El directorio del usuario, **resuelto**.
///
/// Canonicalizado y no el texto de `$HOME` a secas: si algún componente es un
/// enlace —`/home/pato` → `/mnt/datos/pato`, que en una máquina con el home en
/// otro disco es lo normal— la ruta canonicalizada de la candidata empieza con
/// `/mnt/datos/pato` y el `$HOME` crudo dice `/home/pato`. La comparación falla,
/// y falla **hacia el lado seguro**: no se borra nada de más, pero la función
/// entera deja de servir —la lista aparece, todos los tamaños quedan en «…» y
/// cada borrado responde «esa carpeta no está en el directorio del usuario»—.
fn home_real() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok()?.parse::<std::path::PathBuf>().ok()?.canonicalize().ok()
}

fn validar_candidata(texto: &str) -> Result<std::path::PathBuf, String> {
    let home = home_real().ok_or_else(|| "no se pudo resolver el HOME".to_string())?;
    validar_bajo(texto, &home.to_string_lossy())
}

/// La validación con el directorio del usuario explícito.
///
/// Separada de `validar_candidata` para poder probarla: el `$HOME` del proceso es
/// global y cambiarlo en un test afectaría a los que corren en paralelo.
fn validar_bajo(texto: &str, home: &str) -> Result<std::path::PathBuf, String> {
    // El home también se canonicaliza acá: quien llama puede pasar un texto con
    // enlaces —los tests pasan `temp_dir()`, que en varios sistemas es uno— y
    // comparar una ruta resuelta contra una que no lo está no funciona.
    let home = std::path::PathBuf::from(home)
        .canonicalize()
        .map_err(|e| format!("no se pudo resolver el HOME: {e}"))?;
    let ruta = std::path::PathBuf::from(texto);

    // Canonicalizada: sin esto, `$HOME/../../etc` pasaría el prefijo.
    let real = ruta
        .canonicalize()
        .map_err(|e| format!("no se pudo resolver {texto}: {e}"))?;
    if !real.starts_with(&home) {
        return Err("esa carpeta no está en el directorio del usuario".to_string());
    }

    // La misma regla estructural que aplica el escaneo: lo que está suelto en el
    // home es la configuración de una herramienta, no un proyecto. Acá se repite
    // porque este comando no le cree a la lista.
    if proyectos::es_hijo_directo(&real, &home) {
        return Err("esa carpeta es del directorio del usuario, no de un proyecto".to_string());
    }

    let nombre = real
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let patron = proyectos::patron_de(&nombre)
        .ok_or_else(|| format!("«{nombre}» no es una carpeta de proyecto"))?;

    if !proyectos::tiene_la_marca(&real, patron) {
        return Err(format!(
            "no hay ninguno de {:?} al lado: no parece un proyecto",
            patron.marcas
        ));
    }

    let (en_repo, ignorada) = estado_en_git(&real);
    if !proyectos::se_puede_ofrecer(patron, en_repo, ignorada) {
        return Err("esa carpeta está versionada o su nombre no es concluyente".to_string());
    }

    Ok(real)
}

/// Borra una de las carpetas encontradas.
///
/// Sin `pkexec`: todo esto vive en el directorio del usuario y no hace falta
/// autenticar. Pedir contraseña para borrar un `node_modules` propio sería teatro.
#[tauri::command]
pub async fn borrar_proyecto(ruta: String) -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let real = validar_candidata(&ruta)?;
        // Se mide antes de borrar: después no hay nada que medir, y el número es
        // lo que la interfaz muestra como recuperado.
        let bytes = du_de(&real).unwrap_or(0);
        std::fs::remove_dir_all(&real)
            .map_err(|e| format!("no se pudo borrar {}: {e}", real.display()))?;
        Ok(bytes)
    })
    .await
    .map_err(|e| format!("el borrado no terminó: {e}"))?
}

#[cfg(test)]
mod tests_proyectos {
    use super::*;

    /// Un árbol con un proyecto de verdad y cosas que no hay que tocar.
    ///
    /// El nombre entra por parámetro y no es cosmético: los tests corren en
    /// paralelo, y con un único directorio compartido cada uno borraba el árbol
    /// del otro a mitad de camino. Pasaban de a uno y fallaban en conjunto, que
    /// es la forma más molesta de test roto.
    fn arbol(nombre: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("vsk-val-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("app/node_modules")).expect("crear");
        std::fs::write(dir.join("app/package.json"), b"{}").expect("marca");
        // Un `target` sin Cargo.toml al lado: se parece y no lo es.
        std::fs::create_dir_all(dir.join("disenio/target")).expect("crear");
        // Y una carpeta de datos, que no es candidata de ninguna forma.
        std::fs::create_dir_all(dir.join("Documentos")).expect("crear");
        dir
    }

    #[test]
    fn una_candidata_de_verdad_pasa() {
        let raiz = arbol("ok");
        let home = raiz.to_string_lossy().into_owned();
        let ruta = raiz.join("app/node_modules");
        assert!(
            validar_bajo(&ruta.to_string_lossy(), &home).is_ok(),
            "se rechazó un node_modules legítimo"
        );
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn no_se_puede_borrar_fuera_del_home() {
        // El comando recibe un texto, y un texto puede ser cualquier cosa. Esto
        // es lo que separa «recuperar espacio» de «borrar el sistema».
        let raiz = arbol("fuera");
        let home = raiz.to_string_lossy().into_owned();
        assert!(validar_bajo("/etc", &home).is_err());
        assert!(validar_bajo("/", &home).is_err());
        // Y con un `..` que se escapa, que es el intento que un prefijo ingenuo
        // no atrapa: por eso la ruta se canonicaliza antes de comparar.
        let escape = format!("{home}/app/../../../../etc");
        assert!(validar_bajo(&escape, &home).is_err(), "un .. se escapó del home");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn una_carpeta_de_datos_no_es_candidata() {
        let raiz = arbol("datos");
        let home = raiz.to_string_lossy().into_owned();
        let docs = raiz.join("Documentos");
        assert!(
            validar_bajo(&docs.to_string_lossy(), &home).is_err(),
            "se aceptó una carpeta de datos"
        );
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn sin_la_marca_hermana_se_rechaza() {
        // `target` sin Cargo.toml al lado. La lista no lo ofrecería, pero el
        // comando no le cree a la lista: vuelve a comprobarlo.
        let raiz = arbol("marca");
        let home = raiz.to_string_lossy().into_owned();
        let falso = raiz.join("disenio/target");
        assert!(validar_bajo(&falso.to_string_lossy(), &home).is_err());
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn un_home_con_enlaces_igual_funciona() {
        // Si algún componente del home es un enlace —`/home/pato` →
        // `/mnt/datos/pato`, normal cuando el home está en otro disco— la ruta
        // canonicalizada de la candidata no empieza con el texto crudo de $HOME.
        // El fallo va hacia el lado seguro, pero rompe la función entera: la lista
        // aparece, los tamaños quedan en «…» y cada borrado dice que la carpeta no
        // está en el home.
        let raiz = arbol("enlace");
        let alias = std::env::temp_dir().join(format!("vsk-alias-{}", std::process::id()));
        let _ = std::fs::remove_file(&alias);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&raiz, &alias).expect("enlace");

        // El home entra por el alias, la candidata por la ruta real.
        let ruta = raiz.join("app/node_modules");
        assert!(
            validar_bajo(&ruta.to_string_lossy(), &alias.to_string_lossy()).is_ok(),
            "un home con enlaces rompió la validación"
        );

        let _ = std::fs::remove_file(&alias);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn una_hija_directa_del_home_se_rechaza() {
        // La misma regla que aplica el escaneo, repetida acá porque este comando
        // no le cree a la lista. `$HOME/.gradle` es el home de Gradle.
        let raiz = arbol("hijadirecta");
        let home = raiz.to_string_lossy().into_owned();
        let gradle = raiz.join(".gradle");
        std::fs::create_dir_all(&gradle).expect("crear");

        assert!(
            validar_bajo(&gradle.to_string_lossy(), &home).is_err(),
            "se aceptó una hija directa del home"
        );
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn una_ruta_que_no_existe_se_rechaza() {
        let raiz = arbol("inexistente");
        let home = raiz.to_string_lossy().into_owned();
        let inventada = raiz.join("no/existe/node_modules");
        assert!(validar_bajo(&inventada.to_string_lossy(), &home).is_err());
        let _ = std::fs::remove_dir_all(&raiz);
    }
}
