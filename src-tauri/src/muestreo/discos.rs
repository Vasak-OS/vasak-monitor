//! El espacio en disco, contando sólo lo que ocupa espacio de verdad.
//!
//! `/proc/mounts` lista decenas de montajes que no son almacenamiento: `proc`,
//! `sysfs`, `cgroup2`, `tmpfs` de cada servicio, los `overlay` de los contenedores.
//! Mostrarlos todos llena la pantalla de filas al 0% y esconde las dos o tres que
//! importan.

/// Un sistema de archivos que de verdad guarda cosas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Montaje {
    pub dispositivo: String,
    pub punto: String,
    pub tipo: String,
}

/// Los tipos de sistema de archivos que no ocupan disco.
///
/// `tmpfs` se excluye aunque `/tmp` y `/run` puedan tener contenido: viven en RAM,
/// así que su ocupación ya se cuenta en la memoria. Contarla dos veces haría que
/// «liberar espacio» sugiera borrar cosas que no están en el disco.
const VIRTUALES: &[&str] = &[
    "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "cgroup", "cgroup2", "pstore",
    "efivarfs", "bpf", "tracefs", "debugfs", "securityfs", "configfs", "fusectl",
    "hugetlbfs", "mqueue", "autofs", "binfmt_misc", "ramfs", "squashfs", "overlay",
    "nsfs", "fuse.portal", "fuse.gvfsd-fuse", "rpc_pipefs",
];

/// Los montajes que vale la pena mostrar, leídos de `/proc/mounts`.
pub fn montajes_de(texto: &str) -> Vec<Montaje> {
    let mut vistos: Vec<Montaje> = Vec::new();

    for linea in texto.lines() {
        let campos: Vec<&str> = linea.split_whitespace().collect();
        // Seis campos, que es lo que el formato garantiza: dispositivo, punto,
        // tipo, opciones, dump y paso de fsck. Pedir sólo tres dejaba pasar
        // cualquier línea de tres palabras como si fuera un montaje —lo encontró
        // un test con la cadena «una linea corta»—.
        if campos.len() < 6 {
            continue;
        }
        let (dispositivo, punto, tipo) = (campos[0], campos[1], campos[2]);

        if VIRTUALES.contains(&tipo) {
            continue;
        }
        // Un mismo dispositivo montado en varios lugares —subvolúmenes de btrfs,
        // o un bind— es el mismo espacio. Mostrarlo dos veces sugiere que hay el
        // doble de disco ocupado del que hay.
        if vistos.iter().any(|m| m.dispositivo == dispositivo) {
            continue;
        }

        vistos.push(Montaje {
            dispositivo: dispositivo.to_string(),
            // Los espacios y otros caracteres vienen escapados en octal.
            punto: desescapar(punto),
            tipo: tipo.to_string(),
        });
    }

    vistos
}

/// Deshace el escapado octal de `/proc/mounts`.
///
/// Un punto de montaje con un espacio aparece como `/mnt/Disco\040externo`.
/// Mostrarlo tal cual se ve mal, y usarlo como ruta para consultar el espacio
/// falla — el nombre real no tiene esa barra.
pub fn desescapar(ruta: &str) -> String {
    let bytes = ruta.as_bytes();
    let mut salida = String::with_capacity(ruta.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() {
            let octal = &ruta[i + 1..i + 4];
            if let Ok(valor) = u8::from_str_radix(octal, 8) {
                salida.push(valor as char);
                i += 4;
                continue;
            }
        }
        salida.push(bytes[i] as char);
        i += 1;
    }

    salida
}

/// Cuánto ocupa un tamaño, en la unidad que se lee de un vistazo.
pub fn formato_de_tamano(bytes: u64) -> (f64, &'static str) {
    const UNIDADES: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    let mut valor = bytes as f64;
    let mut indice = 0;
    while valor >= 1000.0 && indice + 1 < UNIDADES.len() {
        valor /= 1000.0;
        indice += 1;
    }
    (valor, UNIDADES[indice])
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUESTRA: &str = "\
proc /proc proc rw,nosuid 0 0
sysfs /sys sysfs rw,nosuid 0 0
/dev/nvme0n1p2 / ext4 rw,relatime 0 0
tmpfs /run tmpfs rw,nosuid 0 0
/dev/nvme0n1p1 /boot vfat rw,relatime 0 0
/dev/sda1 /mnt/Disco\\040externo ext4 rw,relatime 0 0
cgroup2 /sys/fs/cgroup cgroup2 rw 0 0
/dev/nvme0n1p2 /home btrfs rw,subvol=home 0 0
";

    #[test]
    fn los_montajes_virtuales_no_se_muestran() {
        // Mostrarlos llena la pantalla de filas al 0% y esconde las que importan.
        let m = montajes_de(MUESTRA);
        let tipos: Vec<&str> = m.iter().map(|x| x.tipo.as_str()).collect();
        assert!(!tipos.contains(&"proc"));
        assert!(!tipos.contains(&"sysfs"));
        assert!(!tipos.contains(&"tmpfs"));
        assert!(!tipos.contains(&"cgroup2"));
    }

    #[test]
    fn un_dispositivo_montado_dos_veces_aparece_una() {
        // `/` y `/home` son el mismo nvme0n1p2 acá —subvolúmenes de btrfs— y es
        // el mismo espacio: mostrarlo dos veces sugiere el doble de disco ocupado.
        let m = montajes_de(MUESTRA);
        let repetidos = m.iter().filter(|x| x.dispositivo == "/dev/nvme0n1p2").count();
        assert_eq!(repetidos, 1);
    }

    #[test]
    fn quedan_los_que_de_verdad_ocupan_disco() {
        let m = montajes_de(MUESTRA);
        let puntos: Vec<&str> = m.iter().map(|x| x.punto.as_str()).collect();
        assert!(puntos.contains(&"/"));
        assert!(puntos.contains(&"/boot"));
        assert_eq!(m.len(), 3, "raíz, boot y el disco externo");
    }

    #[test]
    fn un_punto_de_montaje_con_espacios_se_desescapa() {
        // Sin esto se muestra «Disco\040externo», y usar esa ruta para consultar
        // el espacio falla porque el nombre real no tiene la barra.
        let m = montajes_de(MUESTRA);
        assert!(
            m.iter().any(|x| x.punto == "/mnt/Disco externo"),
            "quedó: {:?}",
            m.iter().map(|x| &x.punto).collect::<Vec<_>>()
        );
    }

    #[test]
    fn el_desescapado_no_toca_lo_que_no_es_una_secuencia() {
        assert_eq!(desescapar("/home/pato"), "/home/pato");
        assert_eq!(desescapar("/mnt/a\\040b"), "/mnt/a b");
        // Una barra sola al final, o una secuencia incompleta, se deja como está
        // en lugar de indexar fuera de rango.
        assert_eq!(desescapar("/mnt/a\\"), "/mnt/a\\");
        assert_eq!(desescapar("/mnt/a\\04"), "/mnt/a\\04");
        assert_eq!(desescapar("/mnt/a\\xyz"), "/mnt/a\\xyz");
    }

    #[test]
    fn los_tamanos_se_muestran_en_la_unidad_que_se_lee() {
        assert_eq!(formato_de_tamano(0), (0.0, "B"));
        assert_eq!(formato_de_tamano(999), (999.0, "B"));
        assert_eq!(formato_de_tamano(1_500_000_000), (1.5, "GB"));
        let (_, unidad) = formato_de_tamano(u64::MAX);
        assert_eq!(unidad, "TB", "no se pasa de la unidad más grande");
    }

    #[test]
    fn un_proc_mounts_raro_no_paniquea() {
        assert_eq!(montajes_de(""), vec![]);
        assert_eq!(montajes_de("una linea corta"), vec![], "tres palabras no son un montaje");
        assert_eq!(montajes_de("/dev/sda1 /mnt ext4"), vec![], "faltan las opciones");
    }
}
