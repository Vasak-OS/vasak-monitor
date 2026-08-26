//! El diario de VasakOS.
//!
//! Se lee con `journalctl -o json`, una entrada por línea. El formato tabulado es
//! más corto pero pierde el nivel de severidad y la unidad, que son justamente lo
//! que hace falta para poder filtrar — y sin filtrar, «los registros» son diez mil
//! líneas donde el error que se busca está en alguna parte.

/// Una entrada del diario, con lo que hace falta para mostrarla y filtrarla.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Entrada {
    /// Microsegundos desde la época, como los informa el diario.
    pub microsegundos: u64,
    /// La unidad que la escribió, o el nombre del proceso si no hay unidad.
    pub origen: String,
    /// 0 emergencia … 7 depuración, según syslog.
    pub nivel: u8,
    pub mensaje: String,
}

impl Entrada {
    /// Si es un error o algo peor.
    ///
    /// Hasta 3 inclusive: emergencia, alerta, crítico y error. Ese es el corte que
    /// separa «algo se rompió» de «algo pasó», y es el filtro que alguien quiere
    /// la primera vez que abre esto.
    pub fn es_problema(&self) -> bool {
        self.nivel <= 3
    }
}

/// Lee la salida de `journalctl -o json`, una entrada por línea.
///
/// Las líneas que no parsean se saltan en lugar de abortar: el diario puede tener
/// entradas con campos binarios —que salen como listas de bytes en lugar de texto—
/// y perder una línea es mejor que perder el resto del diario.
pub fn entradas_de(texto: &str) -> Vec<Entrada> {
    texto.lines().filter_map(entrada_de).collect()
}

/// Una sola entrada, desde su línea JSON.
pub fn entrada_de(linea: &str) -> Option<Entrada> {
    let v: serde_json::Value = serde_json::from_str(linea).ok()?;

    // El mensaje puede venir como texto o como lista de bytes cuando tiene
    // contenido binario. En ese caso no hay nada legible que mostrar.
    let mensaje = v.get("MESSAGE")?.as_str()?.to_string();

    // `__REALTIME_TIMESTAMP` viene como cadena, no como número: el diario usa
    // microsegundos y no entran en el double de JSON sin perder precisión.
    let microsegundos = v
        .get("__REALTIME_TIMESTAMP")
        .and_then(|t| t.as_str())
        .and_then(|t| t.parse().ok())
        .unwrap_or(0);

    // El nivel también es una cadena. Sin nivel se asume informativo: tratarlo
    // como error llenaría el filtro de problemas que no lo son.
    let nivel = v
        .get("PRIORITY")
        .and_then(|p| p.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(6);

    // La unidad primero, el proceso después: «vasak-keyring.service» dice más que
    // «vasak-keyring», y para lo que no es un servicio el proceso es lo único que
    // hay.
    let origen = ["_SYSTEMD_USER_UNIT", "_SYSTEMD_UNIT", "SYSLOG_IDENTIFIER", "_COMM"]
        .iter()
        .find_map(|clave| v.get(*clave).and_then(|x| x.as_str()))
        .unwrap_or("desconocido")
        .to_string();

    Some(Entrada {
        microsegundos,
        origen,
        nivel,
        mensaje,
    })
}

/// La hora de una entrada, en formato local, para mostrarla.
pub fn hora_de(microsegundos: u64, desplazamiento: i64) -> String {
    let segundos = (microsegundos / 1_000_000) as i64 + desplazamiento;
    let resto = segundos.rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02}",
        resto / 3600,
        (resto % 3600) / 60,
        resto % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_lee_una_entrada_completa() {
        let linea = r#"{"__REALTIME_TIMESTAMP":"1756230000000000","PRIORITY":"3","_SYSTEMD_USER_UNIT":"vasak-keyring.service","MESSAGE":"no se pudo abrir el llavero"}"#;
        let e = entrada_de(linea).expect("parsea");
        assert_eq!(e.microsegundos, 1_756_230_000_000_000);
        assert_eq!(e.origen, "vasak-keyring.service");
        assert_eq!(e.nivel, 3);
        assert!(e.es_problema());
    }

    #[test]
    fn la_marca_de_tiempo_no_pierde_precision() {
        // Viene como cadena porque los microsegundos no entran en el double de
        // JSON. Parseándola como número, los últimos dígitos se redondean y dos
        // entradas del mismo segundo quedan con la misma hora.
        let linea = r#"{"__REALTIME_TIMESTAMP":"1756230000123456","MESSAGE":"algo"}"#;
        assert_eq!(entrada_de(linea).unwrap().microsegundos, 1_756_230_000_123_456);
    }

    #[test]
    fn el_corte_de_problema_es_el_nivel_tres() {
        // Emergencia, alerta, crítico y error son problemas; aviso e info no.
        for nivel in 0..=3 {
            let l = format!(r#"{{"PRIORITY":"{nivel}","MESSAGE":"x"}}"#);
            assert!(entrada_de(&l).unwrap().es_problema(), "nivel {nivel}");
        }
        for nivel in 4..=7 {
            let l = format!(r#"{{"PRIORITY":"{nivel}","MESSAGE":"x"}}"#);
            assert!(!entrada_de(&l).unwrap().es_problema(), "nivel {nivel}");
        }
    }

    #[test]
    fn sin_nivel_se_asume_informativo() {
        // Tratarlo como error llenaría el filtro de problemas que no lo son.
        let e = entrada_de(r#"{"MESSAGE":"algo"}"#).unwrap();
        assert_eq!(e.nivel, 6);
        assert!(!e.es_problema());
    }

    #[test]
    fn el_origen_prefiere_la_unidad_sobre_el_proceso() {
        // «vasak-keyring.service» dice más que «vasak-keyring».
        let con_unidad = r#"{"_SYSTEMD_USER_UNIT":"a.service","_COMM":"a","MESSAGE":"x"}"#;
        assert_eq!(entrada_de(con_unidad).unwrap().origen, "a.service");

        // Y para lo que no es un servicio, el proceso es lo único que hay.
        let sin_unidad = r#"{"_COMM":"kernel","MESSAGE":"x"}"#;
        assert_eq!(entrada_de(sin_unidad).unwrap().origen, "kernel");

        let sin_nada = r#"{"MESSAGE":"x"}"#;
        assert_eq!(entrada_de(sin_nada).unwrap().origen, "desconocido");
    }

    #[test]
    fn un_mensaje_binario_se_saltea_sin_perder_el_resto() {
        // El diario puede traer el mensaje como lista de bytes. Abortar ahí
        // perdería el resto del diario por una entrada que no se puede mostrar.
        let texto = concat!(
            r#"{"MESSAGE":"la primera"}"#, "\n",
            r#"{"MESSAGE":[104,111,108,97]}"#, "\n",
            "esto no es json\n",
            r#"{"MESSAGE":"la última"}"#, "\n"
        );
        let e = entradas_de(texto);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].mensaje, "la primera");
        assert_eq!(e[1].mensaje, "la última");
    }

    #[test]
    fn la_hora_se_muestra_en_local() {
        // Los valores salen de `date -u`, no de mi aritmética: 1756230000 son las
        // 17:40:00 UTC, y con -3 horas las 14:40:00.
        assert_eq!(hora_de(1_756_230_000_000_000, 0), "17:40:00");
        assert_eq!(hora_de(1_756_230_000_000_000, -10_800), "14:40:00");
    }

    #[test]
    fn una_hora_que_cruza_la_medianoche_no_se_va_a_negativo() {
        // Con desplazamiento negativo y una hora temprana, la resta cruza el día.
        // Sin `rem_euclid` daría una hora negativa.
        // 1756252800 son las 00:00:00 UTC; con -3 horas cae en el día anterior,
        // a las 21:00:00. Sin `rem_euclid` la resta daría una hora negativa.
        let medianoche_utc = 1_756_252_800_000_000;
        assert_eq!(hora_de(medianoche_utc, -10_800), "21:00:00");
        assert!(!hora_de(medianoche_utc, -10_800).contains('-'));
    }
}
