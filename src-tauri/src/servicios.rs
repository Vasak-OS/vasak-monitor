//! Los servicios de la sesión y del sistema.
//!
//! Se leen con `systemctl` y no hablando D-Bus con systemd a mano: `systemctl` ya
//! resuelve las dos instancias —la del usuario y la del sistema—, los alias y los
//! estados derivados, y su salida tabulada es estable desde hace años.

/// Un servicio, con lo que hace falta para mostrarlo y decidir qué se le puede
/// hacer.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Servicio {
    pub unidad: String,
    /// `active`, `inactive`, `failed`…
    pub estado: String,
    /// `running`, `exited`, `dead`…
    pub detalle: String,
    pub descripcion: String,
    /// Si es de la instancia del usuario. Los del sistema necesitan autenticar.
    pub del_usuario: bool,
    /// Si es una unidad de VasakOS. Sirve para mostrarlas primero: alguien que
    /// abre esto para ver por qué el escritorio se porta raro busca las suyas, no
    /// las treinta del sistema.
    pub de_vasakos: bool,
}

/// Si el nombre de una unidad es de VasakOS.
///
/// Por prefijo y no por una lista: los servicios se agregan y una lista fija
/// dejaría de reconocer los nuevos sin que nada lo avise.
pub fn es_de_vasakos(unidad: &str) -> bool {
    unidad.starts_with("vasak-")
        || unidad.starts_with("vasakos-")
        || unidad.starts_with("polkit-vasak")
        || unidad.starts_with("tauri-plugin-")
}

/// Lee la salida de `systemctl list-units --type=service --plain --no-legend`.
///
/// El formato es: unidad, cargada, activa, sub, descripción. La descripción lleva
/// espacios, así que se toman los cuatro primeros campos y **todo el resto** es la
/// descripción — partiendo por espacios sin más, un servicio con descripción larga
/// deja campos corridos.
pub fn servicios_de(texto: &str, del_usuario: bool) -> Vec<Servicio> {
    let mut lista = Vec::new();

    for linea in texto.lines() {
        // Una unidad fallida viene con un punto delante: «● algo.service».
        let linea = linea.trim_start_matches(['●', '*', ' ']);
        let mut campos = linea.split_whitespace();

        let (Some(unidad), Some(_cargada), Some(estado), Some(detalle)) =
            (campos.next(), campos.next(), campos.next(), campos.next())
        else {
            continue;
        };
        if !unidad.ends_with(".service") {
            continue;
        }

        let descripcion = campos.collect::<Vec<_>>().join(" ");

        lista.push(Servicio {
            de_vasakos: es_de_vasakos(unidad),
            unidad: unidad.to_string(),
            estado: estado.to_string(),
            detalle: detalle.to_string(),
            descripcion,
            del_usuario,
        });
    }

    lista
}

/// Ordena para que lo propio y lo roto se vea primero.
///
/// Primero lo que falló —es lo que se viene a buscar—, después las unidades de
/// VasakOS, y el resto alfabético. Un orden puramente alfabético entierra un
/// servicio caído en la mitad de la lista.
pub fn ordenar(mut lista: Vec<Servicio>) -> Vec<Servicio> {
    lista.sort_by(|a, b| {
        let clave = |s: &Servicio| {
            (
                if s.estado == "failed" { 0 } else { 1 },
                if s.de_vasakos { 0 } else { 1 },
                s.unidad.clone(),
            )
        };
        clave(a).cmp(&clave(b))
    });
    lista
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUESTRA: &str = "\
  dbus.service                loaded active running D-Bus System Message Bus
● vasak-connect.service       loaded failed failed  VasakOS phone integration daemon
  vasak-keyring.service       loaded active running VasakOS Secret Service keyring daemon
  pipewire.service            loaded active running PipeWire Multimedia Service
  algo.socket                 loaded active listening No es un servicio
";

    #[test]
    fn se_leen_las_unidades_con_su_estado() {
        let s = servicios_de(MUESTRA, true);
        assert_eq!(s.len(), 4, "el .socket no cuenta");
        let connect = s.iter().find(|x| x.unidad == "vasak-connect.service").unwrap();
        assert_eq!(connect.estado, "failed");
        assert_eq!(connect.descripcion, "VasakOS phone integration daemon");
    }

    #[test]
    fn el_punto_de_una_unidad_fallida_no_se_toma_por_el_nombre() {
        // systemctl pone «●» delante de lo que falló. Sin sacarlo, el nombre de la
        // unidad sale con el símbolo y no coincide con nada al querer reiniciarla.
        let s = servicios_de(MUESTRA, true);
        assert!(s.iter().any(|x| x.unidad == "vasak-connect.service"));
        assert!(!s.iter().any(|x| x.unidad.starts_with('●')));
    }

    #[test]
    fn una_descripcion_con_espacios_no_corre_los_campos() {
        // «D-Bus System Message Bus» son cuatro palabras. Partiendo por espacios
        // sin juntar el resto, la descripción quedaría en «D-Bus» y el estado
        // saldría de otra columna.
        let s = servicios_de(MUESTRA, true);
        let dbus = s.iter().find(|x| x.unidad == "dbus.service").unwrap();
        assert_eq!(dbus.descripcion, "D-Bus System Message Bus");
        assert_eq!(dbus.estado, "active");
    }

    #[test]
    fn las_unidades_de_vasakos_se_reconocen_por_prefijo() {
        // Por prefijo y no por lista: los servicios se agregan, y una lista fija
        // dejaría de reconocer los nuevos sin que nada lo avise.
        assert!(es_de_vasakos("vasak-keyring.service"));
        assert!(es_de_vasakos("vasakos-algo.service"));
        assert!(es_de_vasakos("polkit-vasak-agent.service"));
        assert!(!es_de_vasakos("pipewire.service"));
        assert!(!es_de_vasakos("dbus.service"));
        // Y un nombre que sólo contiene «vasak» no cuenta.
        assert!(!es_de_vasakos("otro-vasak.service"));
    }

    #[test]
    fn lo_roto_y_lo_propio_va_primero() {
        // Un orden alfabético entierra un servicio caído en la mitad de la lista,
        // que es exactamente lo que alguien viene a buscar acá.
        let ordenados = ordenar(servicios_de(MUESTRA, true));
        assert_eq!(ordenados[0].unidad, "vasak-connect.service", "lo fallido primero");
        assert_eq!(ordenados[1].unidad, "vasak-keyring.service", "después lo propio");
    }

    #[test]
    fn una_salida_vacia_o_rara_no_paniquea() {
        assert_eq!(servicios_de("", true), vec![]);
        assert_eq!(servicios_de("una linea", true), vec![]);
        assert_eq!(servicios_de("a.service loaded", true), vec![], "faltan campos");
    }
}
