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

/// Cuánto guarda el kernel del nombre de un proceso.
///
/// `TASK_COMM_LEN` son 16 bytes con el NUL, así que `_COMM` llega **cortado** a 15
/// caracteres: en el diario de esta máquina hay `vasak-lock-scre` y
/// `vasak-polkit-ag`. Sin tener esto en cuenta, filtrar por `_COMM` con el nombre
/// completo no devuelve nada y el selector muestra apps que parecen mudas.
pub const LIMITE_COMM: usize = 15;

/// El identificador que pide «todo el ecosistema».
pub const TODO_EL_ECOSISTEMA: &str = "";
/// El identificador que pide el diario completo, VasakOS y lo demás.
pub const TODO_EL_SISTEMA: &str = "*";

/// Los campos del diario donde puede estar quién escribió una entrada.
///
/// Son cuatro porque una app del escritorio aparece de maneras distintas según
/// cómo se lanzó: un demonio bajo systemd deja la unidad, uno lanzado desde el
/// menú deja sólo el identificador o el nombre del proceso.
pub const CAMPOS_DE_ORIGEN: [&str; 4] = [
    "SYSLOG_IDENTIFIER",
    "_COMM",
    "_SYSTEMD_USER_UNIT",
    "_SYSTEMD_UNIT",
];

/// El ecosistema, con el icono de cada uno y los nombres alternativos con los que
/// aparece en el diario.
///
/// La lista es fija **además** de lo que se descubre en el diario, no en lugar de
/// eso: una app que todavía no escribió nada tiene que estar en el selector igual,
/// porque si no aparece parece que el monitor no la conoce. Y lo que se descubra
/// con prefijo `vasak-` y no esté acá se agrega solo, así que sumar un paquete
/// nuevo no obliga a tocar esto.
///
/// El tercer campo son alias: el agente de polkit corre en la unidad
/// `polkit-vasak-agent.service` pero escribe con el identificador
/// `vasak-polkit-agent`. Sin el alias serían dos entradas distintas del selector
/// para la misma app, cada una con la mitad de las líneas.
pub const ECOSISTEMA: [(&str, &str, &[&str]); 18] = [
    ("vasak-desktop", "vasak-desktop", &[]),
    ("vasak-terminal", "vasak-terminal", &[]),
    ("vasak-settings", "vasak-settings", &[]),
    ("vasak-file-manager", "vasak-file-manager", &[]),
    ("vasak-gallery", "vasak-gallery", &[]),
    ("vasak-resonance", "vasak-resonance", &[]),
    ("vasak-press-and-hold", "vasak-press-and-hold", &[]),
    ("vasak-polkit-agent", "vasak-polkit-agent", &["polkit-vasak-agent"]),
    ("vasak-monitor", "utilities-system-monitor", &[]),
    ("vasak-shot", "applets-screenshooter", &[]),
    ("vasak-connect", "network-wireless", &[]),
    ("vasak-accounts", "system-users", &[]),
    ("vasak-keyring", "dialog-password", &[]),
    ("vasak-flare-daemon", "preferences-desktop-notification", &[]),
    ("vasak-permissions", "security-high", &[]),
    ("vasak-session-manager", "system-log-out", &[]),
    ("vasak-lock-screen", "system-lock-screen", &[]),
    ("vasak-secrets-portal", "dialog-password", &[]),
];

/// Una app del selector de registros.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AppDelDiario {
    /// Lo que se manda de vuelta para filtrar.
    pub id: String,
    /// Nombre del icono, siempre resuelto por el plugin y nunca una ruta.
    pub icono: String,
    /// Si el diario tiene algo suyo. No se puede acotar al arranque actual porque
    /// `journalctl` no admite `-F` con `-b`; ver `argumentos_de_campo`. Las que no,
    /// se marcan en el selector: es distinto «esta app no falló» de «esta app no
    /// escribió nunca».
    pub presente: bool,
}

/// Si el nombre pertenece al ecosistema.
pub fn es_del_ecosistema(nombre: &str) -> bool {
    crate::servicios::es_de_vasakos(nombre)
}

/// Los sufijos con los que systemd nombra sus unidades.
pub const SUFIJOS_DE_UNIDAD: [&str; 8] = [
    ".service",
    ".socket",
    ".timer",
    ".target",
    ".path",
    ".slice",
    ".mount",
    ".scope",
];

/// Los sufijos que se prueban al consultar por unidad.
///
/// Sólo dos: son los que estos demonios usan de verdad, y cada uno agrega una
/// coincidencia más por app a una consulta que ya lleva varias.
pub const SUFIJOS_CONSULTADOS: [&str; 2] = [".service", ".socket"];

/// Cuántas coincidencias genera un nombre.
pub const COINCIDENCIAS_POR_NOMBRE: usize = 2 + SUFIJOS_CONSULTADOS.len() * 2;

/// Le saca el sufijo de unidad a un nombre, para que `vasak-keyring.service` y
/// `vasak-keyring` sean la misma app en el selector y no dos.
///
/// Se sacan todos y no sólo `.service`: un demonio activado por socket aparece como
/// `vasak-keyring.socket`, y dejándolo entero quedaba una entrada más del selector
/// que además consultaba por `vasak-keyring.socket.service` y no encontraba nada.
pub fn normalizar(valor: &str) -> &str {
    for sufijo in SUFIJOS_DE_UNIDAD {
        if let Some(base) = valor.strip_suffix(sufijo) {
            return base;
        }
    }
    valor
}

/// Si un identificador se puede pasar a `journalctl`.
///
/// Se valida aunque el argumento vaya como `arg` y no por una shell: un valor que
/// empiece con `-` lo toma como opción, y `--output=cat` o `-M otra-maquina`
/// cambian qué se lee. Sólo pasan los nombres del ecosistema.
pub fn id_valido(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            // `@` porque una unidad con plantilla lo lleva en el nombre. Ni `-` al
            // principio ni nada que `journalctl` pueda tomar por una opción.
            .all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.' | b'@')
            })
        && es_del_ecosistema(id)
}

/// Los nombres con los que una app aparece en el diario: el suyo y sus alias.
pub fn nombres_de(id: &str) -> Vec<String> {
    let mut nombres = vec![id.to_string()];
    if let Some((_, _, alias)) = ECOSISTEMA.iter().find(|(e, _, _)| *e == id) {
        nombres.extend(alias.iter().map(|a| a.to_string()));
    }
    nombres
}

/// Los orígenes del ecosistema presentes en el diario, a partir de la salida de
/// `journalctl -F <campo>` (un valor por línea, de todos los campos juntos).
///
/// Los valores cortados por el kernel se descartan cuando hay uno más largo que
/// los completa: `vasak-lock-scre` no es otra app, es `vasak-lock-screen` visto
/// por `_COMM`.
pub fn origenes_de(texto: &str) -> Vec<String> {
    // Se filtra con `id_valido` y no sólo por el prefijo: un nombre que después no
    // se puede pasar a `journalctl` aparecía en el selector y al elegirlo no
    // filtraba nada, cayendo en silencio a «todo el ecosistema».
    let mut nombres: Vec<String> = texto
        .lines()
        .map(|l| normalizar(l.trim()))
        .filter(|l| id_valido(l))
        .map(|l| l.to_string())
        .collect();
    nombres.sort();
    nombres.dedup();

    // Los alias se informan con el id de la app, así que el selector no muestra
    // `polkit-vasak-agent` aparte de `vasak-polkit-agent`.
    for (id, _, alias) in ECOSISTEMA {
        if alias.iter().any(|a| nombres.iter().any(|n| n == a)) && !nombres.iter().any(|n| n == id) {
            nombres.push(id.to_string());
        }
    }
    nombres.retain(|n| !ECOSISTEMA.iter().any(|(_, _, alias)| alias.contains(&n.as_str())));

    // Sólo lo que llegó **justo** al límite puede venir cortado. Con `<` en lugar
    // de `!=`, un nombre más largo que el límite también entraba a la comparación y
    // desaparecía por existir otro que lo extendiera: `vasak-file-manager` se
    // perdía si el diario tuviera un `vasak-file-manager-algo`. Y las dos medidas
    // van en caracteres, no una en caracteres y otra en bytes.
    let completos = nombres.clone();
    nombres.retain(|n| {
        let largo = n.chars().count();
        largo != LIMITE_COMM
            || !completos
                .iter()
                .any(|otro| otro.chars().count() > largo && otro.starts_with(n.as_str()))
    });
    nombres.sort();
    nombres.dedup();
    nombres
}

/// Los argumentos para enumerar los valores de un campo del diario.
///
/// **Sin `-b`.** `journalctl` rechaza `-F` junto con cualquier opción que limite el
/// diario —«-F/--field= and -N/--fields cannot be combined with options that limit
/// the journal»— y falla devolviendo nada, así que con `-b` el catálogo marcaba
/// todo el ecosistema como mudo. Por eso el catálogo dice si una app escribió
/// **alguna vez**, y son las entradas las que se limitan al arranque actual.
pub fn argumentos_de_campo(ambito: &str, campo: &str) -> Vec<String> {
    vec![
        ambito.to_string(),
        "--no-pager".to_string(),
        "-F".to_string(),
        campo.to_string(),
    ]
}

/// El catálogo del selector: el ecosistema entero más lo que se haya descubierto,
/// con las que escribieron algo primero.
pub fn catalogo(presentes: &[String]) -> Vec<AppDelDiario> {
    let mut apps: Vec<AppDelDiario> = ECOSISTEMA
        .iter()
        .map(|(id, icono, _)| AppDelDiario {
            id: id.to_string(),
            icono: icono.to_string(),
            presente: presentes.iter().any(|p| p == id),
        })
        .collect();

    // Lo descubierto que la lista fija no conoce: un paquete nuevo aparece sin
    // que haya que tocar el código.
    for nombre in presentes {
        if id_valido(nombre) && !apps.iter().any(|a| &a.id == nombre) {
            apps.push(AppDelDiario {
                id: nombre.clone(),
                icono: "application-x-executable".to_string(),
                presente: true,
            });
        }
    }

    apps.sort_by(|a, b| b.presente.cmp(&a.presente).then_with(|| a.id.cmp(&b.id)));
    apps
}

/// Los argumentos de coincidencia para `journalctl`, ya intercalados con el `+`
/// que separa alternativas.
///
/// Varias coincidencias de campos distintos son un Y para `journalctl`; el `+`
/// entre grupos es lo que las vuelve un O. Sin él, pedir identificador **y**
/// unidad **y** proceso a la vez no devuelve una sola línea.
pub fn coincidencias_de(nombres: &[String]) -> Vec<String> {
    let mut argumentos: Vec<String> = Vec::new();
    for nombre in nombres {
        // `_COMM` con el nombre cortado: el kernel guarda quince caracteres.
        let cortado: String = nombre.chars().take(LIMITE_COMM).collect();
        let mut pares: Vec<(&str, String)> = vec![
            ("SYSLOG_IDENTIFIER", nombre.clone()),
            ("_COMM", cortado),
        ];
        for sufijo in SUFIJOS_CONSULTADOS {
            for campo in ["_SYSTEMD_USER_UNIT", "_SYSTEMD_UNIT"] {
                pares.push((campo, format!("{nombre}{sufijo}")));
            }
        }
        for (campo, valor) in pares {
            if !argumentos.is_empty() {
                argumentos.push("+".to_string());
            }
            argumentos.push(format!("{campo}={valor}"));
        }
    }
    argumentos
}

/// Si el nombre de una unidad no dice nada sobre quién escribió.
///
/// systemd envuelve cada proceso que se lanza desde la sesión en un scope
/// transitorio, y el nombre lleva el pid y el número de invocación:
/// `run-p419172-i424974.scope`, `app-com.google.Chrome-2803.scope`. Mostrando eso
/// en la columna de origen, la pantalla de registros no permitía reconocer ninguna
/// app: nueve líneas seguidas del bloqueo de pantalla se veían como nueve orígenes
/// distintos.
pub fn es_unidad_transitoria(unidad: &str) -> bool {
    unidad.ends_with(".scope")
}

/// Quién escribió una entrada, con el nombre más reconocible que haya.
///
/// La unidad primero —«vasak-keyring.service» dice más que «vasak-keyring»— salvo
/// que sea transitoria. Y el identificador antes que `_COMM`, porque el kernel
/// corta `_COMM` a quince caracteres y deja cosas como `vasak-lock-scre`.
pub fn origen_desde(
    unidad_de_usuario: Option<&str>,
    unidad: Option<&str>,
    identificador: Option<&str>,
    comm: Option<&str>,
) -> String {
    [unidad_de_usuario, unidad]
        .into_iter()
        .flatten()
        .find(|u| !es_unidad_transitoria(u))
        .or(identificador)
        .or(comm)
        .unwrap_or("desconocido")
        .to_string()
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

    let campo = |clave: &str| v.get(clave).and_then(|x| x.as_str());
    let origen = origen_desde(
        campo("_SYSTEMD_USER_UNIT"),
        campo("_SYSTEMD_UNIT"),
        campo("SYSLOG_IDENTIFIER"),
        campo("_COMM"),
    );

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

    #[test]
    fn un_comm_cortado_no_es_otra_app() {
        // El kernel guarda 15 caracteres del nombre, así que `_COMM` trae
        // `vasak-lock-scre`. Sin unirlo, el selector muestra dos apps y la
        // cortada no encuentra nada al filtrar por su nombre completo.
        let campos = "vasak-lock-scre\nvasak-lock-screen\nvasak-keyring\n";
        assert_eq!(
            origenes_de(campos),
            vec!["vasak-keyring".to_string(), "vasak-lock-screen".to_string()]
        );
    }

    #[test]
    fn un_nombre_corto_sobrevive_aunque_otro_lo_extienda_por_poco() {
        // La regla sólo descarta lo que llega justo al límite del kernel: si no,
        // `vasak-shot` desaparecería por existir `vasak-shot-algo`.
        let campos = "vasak-shot\nvasak-shot-algo\n";
        let o = origenes_de(campos);
        assert!(o.contains(&"vasak-shot".to_string()), "{o:?}");
        assert!(o.contains(&"vasak-shot-algo".to_string()), "{o:?}");
    }

    #[test]
    fn la_unidad_y_el_proceso_son_la_misma_app() {
        // `vasak-keyring.service` y `vasak-keyring` no son dos entradas del
        // selector.
        assert_eq!(origenes_de("vasak-keyring.service\nvasak-keyring\n").len(), 1);
    }

    #[test]
    fn lo_que_no_es_del_ecosistema_no_entra() {
        let campos = "chrome\ndbus-broker\nkernel\nvasak-connect\n";
        assert_eq!(origenes_de(campos), vec!["vasak-connect".to_string()]);
    }

    #[test]
    fn un_alias_se_informa_con_el_id_de_la_app() {
        // El agente de polkit corre en `polkit-vasak-agent.service` y escribe con
        // el identificador `vasak-polkit-agent`. Son la misma app.
        let o = origenes_de("polkit-vasak-agent.service\nvasak-polkit-ag\n");
        assert_eq!(o, vec!["vasak-polkit-agent".to_string()]);
    }

    #[test]
    fn el_catalogo_trae_el_ecosistema_entero_y_marca_lo_presente() {
        // Una app que todavía no escribió nada tiene que estar en el selector: si
        // no aparece, parece que el monitor no la conoce.
        let apps = catalogo(&["vasak-keyring".to_string()]);
        assert_eq!(apps.len(), ECOSISTEMA.len());
        assert!(apps[0].presente, "las presentes van primero");
        assert_eq!(apps[0].id, "vasak-keyring");
        assert!(apps.iter().any(|a| a.id == "vasak-terminal" && !a.presente));
    }

    #[test]
    fn el_catalogo_suma_lo_descubierto_que_no_esta_en_la_lista() {
        // Un paquete nuevo aparece sin tocar el código.
        let apps = catalogo(&["vasak-flamante".to_string()]);
        let nuevo = apps.iter().find(|a| a.id == "vasak-flamante").expect("está");
        assert!(nuevo.presente);
        assert_eq!(nuevo.icono, "application-x-executable");
    }

    #[test]
    fn cada_app_del_catalogo_pide_un_icono_por_nombre_y_nunca_una_ruta() {
        // El tema puede cambiar en caliente y sus rutas no son estables: el
        // nombre lo resuelve el plugin.
        for (id, icono, _) in ECOSISTEMA {
            assert!(!icono.is_empty(), "{id} sin icono");
            assert!(!icono.contains('/'), "{id} pide una ruta: {icono}");
            assert!(!icono.ends_with(".svg") && !icono.ends_with(".png"), "{id}: {icono}");
        }
    }

    #[test]
    fn no_hay_ids_repetidos_en_el_ecosistema() {
        let mut ids: Vec<&str> = ECOSISTEMA.iter().map(|(i, _, _)| *i).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }

    #[test]
    fn un_id_que_seria_una_opcion_no_pasa() {
        // El argumento no va por una shell, pero `journalctl` toma cualquier cosa
        // que empiece con `-` como opción: `--output=cat` o `-M otra-maquina`
        // cambian qué se lee.
        assert!(!id_valido("--output=cat"));
        assert!(!id_valido("-M"));
        assert!(!id_valido("-b"));
        assert!(!id_valido(""));
        assert!(!id_valido("chrome"));
        assert!(!id_valido("vasak-keyring; rm -rf"));
        assert!(!id_valido("vasak-keyring\n-b"));
        assert!(!id_valido(&"vasak-".repeat(20)));
        assert!(id_valido("vasak-keyring"));
        assert!(id_valido("polkit-vasak-agent"));
    }

    #[test]
    fn las_coincidencias_van_separadas_por_el_mas() {
        // Varias coincidencias de campos distintos son un Y para `journalctl`. Sin
        // el `+` entre grupos, pedir identificador y unidad y proceso a la vez no
        // devuelve una sola línea.
        let c = coincidencias_de(&["vasak-keyring".to_string()]);
        assert_eq!(c.len(), COINCIDENCIAS_POR_NOMBRE * 2 - 1);
        assert_eq!(c[0], "SYSLOG_IDENTIFIER=vasak-keyring");
        assert_eq!(c[1], "+");
        assert!(c.contains(&"_SYSTEMD_USER_UNIT=vasak-keyring.service".to_string()));
        assert!(c.contains(&"_SYSTEMD_UNIT=vasak-keyring.service".to_string()));
        // Ni al principio ni al final: `journalctl` rechaza un `+` colgado.
        assert_ne!(c.first().map(String::as_str), Some("+"));
        assert_ne!(c.last().map(String::as_str), Some("+"));
    }

    #[test]
    fn la_coincidencia_por_comm_usa_el_nombre_cortado() {
        // Con el nombre completo, `_COMM=vasak-lock-screen` no encuentra nada
        // porque el kernel guardó `vasak-lock-scre`.
        let c = coincidencias_de(&["vasak-lock-screen".to_string()]);
        assert!(c.contains(&"_COMM=vasak-lock-scre".to_string()), "{c:?}");
        assert!(c.contains(&"SYSLOG_IDENTIFIER=vasak-lock-screen".to_string()));
    }

    #[test]
    fn los_alias_entran_en_la_consulta() {
        // Filtrar por el agente de polkit tiene que traer también lo que escribió
        // su unidad, que se llama distinto.
        let c = coincidencias_de(&nombres_de("vasak-polkit-agent"));
        assert!(c.contains(&"SYSLOG_IDENTIFIER=vasak-polkit-agent".to_string()));
        assert!(c.contains(&"_SYSTEMD_USER_UNIT=polkit-vasak-agent.service".to_string()));
    }

    #[test]
    fn sin_nombres_no_hay_coincidencias() {
        // Y sin coincidencias `journalctl` lee todo, que es justamente lo que pide
        // «todo el sistema».
        assert!(coincidencias_de(&[]).is_empty());
    }

    #[test]
    fn enumerar_un_campo_no_lleva_el_filtro_de_arranque() {
        // `journalctl` rechaza `-F` con cualquier opción que limite el diario y
        // devuelve nada. Con `-b` puesto, el catálogo marcaba todo el ecosistema
        // como mudo y el selector quedaba inútil.
        let a = argumentos_de_campo("--user", "_COMM");
        assert!(!a.iter().any(|x| x == "-b"), "{a:?}");
        assert!(!a.iter().any(|x| x == "-n"), "{a:?}");
        assert!(!a.iter().any(|x| x.starts_with("--boot")), "{a:?}");
        assert_eq!(a, vec!["--user", "--no-pager", "-F", "_COMM"]);
    }

    #[test]
    fn una_unidad_transitoria_no_tapa_el_nombre_de_la_app() {
        // systemd mete cada proceso de la sesión en un scope con el pid en el
        // nombre. Nueve líneas del bloqueo de pantalla se veían como nueve
        // orígenes distintos, todos ilegibles.
        let linea = r#"{"_SYSTEMD_USER_UNIT":"run-p419172-i424974.scope","SYSLOG_IDENTIFIER":"vasak-lock-screen","_COMM":"vasak-lock-scre","MESSAGE":"x"}"#;
        assert_eq!(entrada_de(linea).unwrap().origen, "vasak-lock-screen");
    }

    #[test]
    fn el_identificador_va_antes_que_comm() {
        // `_COMM` viene cortado a quince caracteres por el kernel.
        assert_eq!(
            origen_desde(None, None, Some("vasak-lock-screen"), Some("vasak-lock-scre")),
            "vasak-lock-screen"
        );
    }

    #[test]
    fn una_unidad_de_verdad_sigue_ganando() {
        assert_eq!(
            origen_desde(Some("vasak-keyring.service"), None, Some("vasak-keyring"), None),
            "vasak-keyring.service"
        );
    }

    #[test]
    fn se_reconocen_los_scopes_transitorios() {
        assert!(es_unidad_transitoria("run-p419172-i424974.scope"));
        assert!(es_unidad_transitoria("app-com.google.Chrome-2803.scope"));
        assert!(es_unidad_transitoria("session-2.scope"));
        assert!(es_unidad_transitoria("init.scope"));
        assert!(!es_unidad_transitoria("vasak-keyring.service"));
    }

    #[test]
    fn un_nombre_mas_largo_que_el_limite_no_lo_borra_otro_que_lo_extienda() {
        // `vasak-file-manager` tiene dieciocho caracteres: no puede venir de un
        // `_COMM` cortado, así que ninguna coincidencia de prefijo lo explica. Con
        // el corte en `<` en lugar de `!=` desaparecía del selector.
        let campos = "vasak-file-manager\nvasak-file-manager-algo\n";
        let o = origenes_de(campos);
        assert!(o.contains(&"vasak-file-manager".to_string()), "{o:?}");
        assert!(o.contains(&"vasak-file-manager-algo".to_string()), "{o:?}");
    }

    #[test]
    fn los_sufijos_de_unidad_se_sacan_todos() {
        // Un demonio activado por socket aparece como `vasak-keyring.socket`.
        // Dejándolo entero quedaba otra entrada del selector, que además
        // consultaba por `vasak-keyring.socket.service` y no encontraba nada.
        assert_eq!(normalizar("vasak-keyring.socket"), "vasak-keyring");
        assert_eq!(normalizar("vasak-keyring.service"), "vasak-keyring");
        assert_eq!(normalizar("vasak-algo.timer"), "vasak-algo");
        assert_eq!(normalizar("vasak-keyring"), "vasak-keyring");
        assert_eq!(origenes_de("vasak-keyring.socket\nvasak-keyring.service\n").len(), 1);
    }

    #[test]
    fn la_consulta_prueba_el_socket_y_no_solo_el_service() {
        let c = coincidencias_de(&["vasak-keyring".to_string()]);
        assert!(c.contains(&"_SYSTEMD_USER_UNIT=vasak-keyring.socket".to_string()), "{c:?}");
        assert!(c.contains(&"_SYSTEMD_USER_UNIT=vasak-keyring.service".to_string()), "{c:?}");
        // El vector lleva los `+` intercalados, así que son los pares menos uno.
        assert_eq!(c.len(), COINCIDENCIAS_POR_NOMBRE * 2 - 1);
    }

    #[test]
    fn lo_que_no_se_puede_consultar_no_llega_al_selector() {
        // Un nombre que `id_valido` rechaza aparecía en el selector y al elegirlo
        // no filtraba nada: caía en silencio a «todo el ecosistema», y quien lo
        // eligió veía el diario entero creyendo que era el de esa app.
        let campos = "vasak-Cosa\nvasak-con espacio\nvasak-bien\nvasak-raro\\x2dcosa\n";
        assert_eq!(origenes_de(campos), vec!["vasak-bien".to_string()]);

        // Y el catálogo tampoco lo suma si le llega igual.
        assert!(!catalogo(&["vasak-Cosa".to_string()]).iter().any(|a| a.id == "vasak-Cosa"));
    }

    #[test]
    fn una_unidad_con_plantilla_es_un_id_valido() {
        // systemd las nombra con `@`, y son unidades legítimas del ecosistema.
        assert!(id_valido("vasak-algo@sesion"));
        // Lo que `journalctl` podría tomar por una opción sigue afuera.
        assert!(!id_valido("-vasak-algo"));
    }
}
