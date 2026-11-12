//! Los puertos que el equipo tiene a la escucha, con el proceso que los ocupa.
//!
//! Se leen con `ss -H -tulpn` (iproute2) y no de `/proc/net/*` a mano: `ss` ya une
//! el socket con su proceso —lo que de otro modo obliga a recorrer los
//! `/proc/<pid>/fd` de todo el sistema—, resuelve IPv4 e IPv6 y su salida es
//! estable. Sin privilegios sólo ve el proceso de los sockets propios; los ajenos
//! vienen sin proceso y se muestran así, en vez de pedir autenticar al abrir la
//! pantalla.

/// Un puerto a la escucha, con lo que hace falta para mostrarlo.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Puerto {
    /// `tcp` o `udp`.
    pub protocolo: String,
    /// La dirección local sin el puerto: `0.0.0.0`, `127.0.0.1`, `[::]`, `*`.
    pub direccion: String,
    pub puerto: u16,
    /// El nombre del proceso, si `ss` lo pudo resolver.
    pub proceso: Option<String>,
    /// El PID del proceso, si lo hay.
    pub pid: Option<u32>,
}

/// Lee la salida de `ss -H -tulpn` (sin encabezado).
///
/// El formato por línea es: netid, estado, recv-q, send-q, dirección local,
/// dirección remota y —si `ss` lo resolvió— el proceso. Las seis columnas se toman
/// por posición; el proceso, que puede faltar y cuyo nombre puede llevar espacios,
/// se separa por su marca `users:(`. Una línea a la que le falten los primeros
/// campos se saltea en vez de inventar un puerto.
pub fn puertos_de(texto: &str) -> Vec<Puerto> {
    let mut lista = Vec::new();

    for linea in texto.lines() {
        // El proceso va al final como `users:((...))` y su nombre puede llevar
        // espacios, así que se separa por su marca y no como un campo más por
        // espacios: las seis columnas de la tabla, de ancho variable, quedan antes.
        let (columnas, proceso_campo) = match linea.split_once("users:(") {
            Some((cols, resto)) => (cols, Some(resto)),
            None => (linea, None),
        };
        let mut campos = columnas.split_whitespace();
        let (Some(protocolo), Some(_estado), Some(_rq), Some(_sq), Some(local)) = (
            campos.next(),
            campos.next(),
            campos.next(),
            campos.next(),
            campos.next(),
        ) else {
            continue;
        };

        let Some((direccion, puerto)) = separar_puerto(local) else {
            continue;
        };
        let (proceso, pid) = proceso_campo.map(proceso_de).unwrap_or((None, None));

        lista.push(Puerto {
            protocolo: protocolo.to_string(),
            direccion: direccion.to_string(),
            puerto,
            proceso,
            pid,
        });
    }

    lista
}

/// Parte `dirección:puerto` por el último `:`, que es el del puerto. IPv6 viene
/// como `[::]:443`, así que partir por el último `:` deja la dirección entera.
fn separar_puerto(local: &str) -> Option<(&str, u16)> {
    let (direccion, puerto) = local.rsplit_once(':')?;
    Some((direccion, puerto.parse().ok()?))
}

/// Saca el nombre y el PID del primer proceso de
/// `users:(("nombre",pid=123,fd=4),...)`. Un puerto con varios procesos (los
/// trabajadores de un servidor) muestra el primero, que alcanza para identificarlo.
fn proceso_de(campo: &str) -> (Option<String>, Option<u32>) {
    let nombre = campo.split('"').nth(1).map(str::to_string);
    let pid = campo.split("pid=").nth(1).and_then(|resto| {
        resto
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()
    });
    (nombre, pid)
}

/// Ordena por puerto, protocolo y dirección, y quita las filas repetidas.
///
/// Un proceso con varios descriptores sobre el mismo puerto —un navegador con
/// mDNS, por ejemplo— aparece una vez por socket; se muestra una sola.
pub fn ordenar(mut lista: Vec<Puerto>) -> Vec<Puerto> {
    lista.sort_by(|a, b| {
        a.puerto
            .cmp(&b.puerto)
            .then_with(|| a.protocolo.cmp(&b.protocolo))
            .then_with(|| a.direccion.cmp(&b.direccion))
            .then_with(|| a.pid.cmp(&b.pid))
    });
    lista.dedup();
    lista
}

/// Los puertos a la escucha, con el proceso que los ocupa cuando `ss` lo ve.
///
/// Vive acá, junto a su parser, y no en `comandos.rs`: es glue fino sobre `ss` y
/// todo lo que tiene lógica (y prueba) está en este módulo.
///
/// Devuelve `Err` si `ss` no arranca o termina con error, para que la pantalla
/// distinga «no se pudo averiguar» de «no hay puertos»: una lista vacía queda
/// reservada a una lectura que salió bien y no encontró ninguno.
#[tauri::command]
pub fn puertos_en_uso() -> Result<Vec<Puerto>, String> {
    let salida = std::process::Command::new("ss")
        .args(["-H", "-tulpn"])
        .output()
        .map_err(|e| format!("no se pudo ejecutar ss: {e}"))?;
    if !salida.status.success() {
        return Err(format!(
            "ss terminó con error: {}",
            String::from_utf8_lossy(&salida.stderr).trim()
        ));
    }
    Ok(ordenar(puertos_de(&String::from_utf8_lossy(
        &salida.stdout,
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TCP_CON_PROCESO: &str =
        "tcp LISTEN 0 512 127.0.0.1:40571 0.0.0.0:* users:((\"claude\",pid=8592,fd=49))";
    const UDP_SIN_PROCESO: &str = "udp UNCONN 0 0 0.0.0.0%wlan0:57829 0.0.0.0:*";
    const TCP_IPV6: &str = "tcp LISTEN 0 4096 [::]:443 [::]:* users:((\"nginx\",pid=999,fd=6))";

    #[test]
    fn una_linea_tcp_con_proceso_se_lee_completa() {
        let lista = puertos_de(TCP_CON_PROCESO);
        assert_eq!(
            lista,
            vec![Puerto {
                protocolo: "tcp".into(),
                direccion: "127.0.0.1".into(),
                puerto: 40571,
                proceso: Some("claude".into()),
                pid: Some(8592),
            }]
        );
    }

    #[test]
    fn un_nombre_de_proceso_con_espacios_se_lee_completo() {
        let lista = puertos_de(
            "tcp LISTEN 0 512 127.0.0.1:631 0.0.0.0:* users:((\"my server\",pid=123,fd=4))",
        );
        assert_eq!(
            lista,
            vec![Puerto {
                protocolo: "tcp".into(),
                direccion: "127.0.0.1".into(),
                puerto: 631,
                proceso: Some("my server".into()),
                pid: Some(123),
            }]
        );
    }

    #[test]
    fn una_linea_sin_proceso_queda_sin_proceso_ni_pid() {
        let lista = puertos_de(UDP_SIN_PROCESO);
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].protocolo, "udp");
        assert_eq!(lista[0].direccion, "0.0.0.0%wlan0");
        assert_eq!(lista[0].puerto, 57829);
        assert_eq!(lista[0].proceso, None);
        assert_eq!(lista[0].pid, None);
    }

    #[test]
    fn el_puerto_de_ipv6_se_parte_por_el_ultimo_dos_puntos() {
        let lista = puertos_de(TCP_IPV6);
        assert_eq!(lista[0].direccion, "[::]");
        assert_eq!(lista[0].puerto, 443);
        assert_eq!(lista[0].proceso.as_deref(), Some("nginx"));
        assert_eq!(lista[0].pid, Some(999));
    }

    #[test]
    fn una_linea_con_campos_de_menos_se_saltea() {
        let texto = format!("tcp LISTEN 0\n\n{TCP_IPV6}\nbasura\n");
        let lista = puertos_de(&texto);
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].puerto, 443);
    }

    #[test]
    fn una_linea_con_puerto_no_numerico_se_saltea() {
        assert!(puertos_de("tcp LISTEN 0 1 0.0.0.0:* 0.0.0.0:*").is_empty());
    }

    #[test]
    fn ordenar_quita_repetidas_y_ordena_por_puerto() {
        let texto = format!("{TCP_IPV6}\n{TCP_CON_PROCESO}\n{TCP_IPV6}\n{UDP_SIN_PROCESO}\n");
        let lista = ordenar(puertos_de(&texto));
        let puertos: Vec<u16> = lista.iter().map(|p| p.puerto).collect();
        assert_eq!(puertos, vec![443, 40571, 57829]);
    }
}
