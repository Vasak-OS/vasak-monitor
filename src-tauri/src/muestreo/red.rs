//! El tráfico de red, que como el de CPU sólo existe como diferencia.
//!
//! `/proc/net/dev` informa bytes acumulados desde que la interfaz se levantó. Un
//! monitor que muestre ese número informa cuánto se transfirió en toda la sesión,
//! que es un dato distinto y no el que se busca — y no falla: muestra un número
//! que crece y parece un caudal.

use std::time::Duration;

/// Los bytes acumulados de una interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Acumulado {
    pub recibidos: u64,
    pub enviados: u64,
}

/// El caudal entre dos lecturas, en bytes por segundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caudal {
    pub bajada: f64,
    pub subida: f64,
}

impl Acumulado {
    /// El caudal desde la lectura anterior.
    ///
    /// `None` cuando no hay con qué comparar, o cuando el contador retrocedió:
    /// eso pasa al reconectar una interfaz, y sin protegerlo la resta se desborda
    /// y aparece un pico de gigabytes por segundo.
    pub fn caudal_desde(self, anterior: Acumulado, transcurrido: Duration) -> Option<Caudal> {
        let segundos = transcurrido.as_secs_f64();
        if segundos <= 0.0 {
            return None;
        }
        let bajada = self.recibidos.checked_sub(anterior.recibidos)?;
        let subida = self.enviados.checked_sub(anterior.enviados)?;
        Some(Caudal {
            bajada: bajada as f64 / segundos,
            subida: subida as f64 / segundos,
        })
    }
}

/// Suma los bytes de todas las interfaces reales de `/proc/net/dev`.
///
/// `lo` queda afuera: el tráfico local no sale de la máquina, y contarlo hace que
/// cualquier cosa que hable con un servicio propio —el llavero, el bus, una base
/// de datos— parezca actividad de red.
pub fn acumulado_de(texto: &str) -> Acumulado {
    let mut total = Acumulado::default();

    for linea in texto.lines() {
        let Some((nombre, resto)) = linea.split_once(':') else {
            continue;
        };
        let nombre = nombre.trim();
        if nombre == "lo" || nombre.is_empty() {
            continue;
        }
        let campos: Vec<u64> = resto
            .split_whitespace()
            .filter_map(|c| c.parse().ok())
            .collect();
        // recibidos: bytes packets errs drop fifo frame compressed multicast
        // enviados:  bytes packets errs drop fifo colls carrier compressed
        if campos.len() < 9 {
            continue;
        }
        total.recibidos += campos[0];
        total.enviados += campos[8];
    }

    total
}

/// Un caudal con la unidad que corresponde, para mostrarlo.
///
/// Se pasa a la unidad más grande que deje un número de una o dos cifras enteras:
/// «1,4 MB/s» se lee de un vistazo y «1438291 B/s» no.
pub fn formato_de_caudal(bytes_por_segundo: f64) -> (f64, &'static str) {
    const UNIDADES: [&str; 4] = ["B/s", "kB/s", "MB/s", "GB/s"];
    let mut valor = bytes_por_segundo.max(0.0);
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
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 5000000    1000    0    0    0     0          0         0  5000000    1000    0    0    0     0       0          0
  eth0: 1000000     500    0    0    0     0          0         0   200000     300    0    0    0     0       0          0
 wlan0:  500000     200    0    0    0     0          0         0   100000     150    0    0    0     0       0          0
";

    #[test]
    fn el_trafico_local_no_cuenta() {
        // `lo` tiene 5 MB acá. Contarlo haría que hablar con el llavero o con el
        // bus pareciera actividad de red.
        let a = acumulado_de(MUESTRA);
        assert_eq!(a.recibidos, 1_500_000);
        assert_eq!(a.enviados, 300_000);
    }

    #[test]
    fn el_caudal_es_la_diferencia_dividida_por_el_tiempo() {
        let antes = Acumulado { recibidos: 1_000_000, enviados: 100_000 };
        let ahora = Acumulado { recibidos: 3_000_000, enviados: 300_000 };
        let c = ahora
            .caudal_desde(antes, Duration::from_secs(2))
            .expect("pasó tiempo");
        assert_eq!(c.bajada, 1_000_000.0);
        assert_eq!(c.subida, 100_000.0);
    }

    #[test]
    fn un_contador_que_retrocede_no_da_un_pico() {
        // Pasa al reconectar una interfaz. Sin protegerlo, la resta se desborda y
        // aparece un pico de gigabytes por segundo que además arruina la escala
        // del gráfico para el resto de la sesión.
        let antes = Acumulado { recibidos: 3_000_000, enviados: 300_000 };
        let ahora = Acumulado { recibidos: 1_000, enviados: 100 };
        assert_eq!(ahora.caudal_desde(antes, Duration::from_secs(1)), None);
    }

    #[test]
    fn sin_tiempo_transcurrido_no_hay_caudal() {
        let a = Acumulado { recibidos: 1, enviados: 1 };
        assert_eq!(a.caudal_desde(a, Duration::ZERO), None);
    }

    #[test]
    fn el_caudal_se_muestra_en_la_unidad_que_se_lee() {
        assert_eq!(formato_de_caudal(0.0), (0.0, "B/s"));
        assert_eq!(formato_de_caudal(999.0), (999.0, "B/s"));
        assert_eq!(formato_de_caudal(1_000.0), (1.0, "kB/s"));
        assert_eq!(formato_de_caudal(1_500_000.0), (1.5, "MB/s"));
        // Y no se pasa de la unidad más grande que tenemos.
        let (valor, unidad) = formato_de_caudal(9.9e15);
        assert_eq!(unidad, "GB/s");
        assert!(valor > 1000.0, "se queda en GB/s aunque el número sea grande");
    }

    #[test]
    fn un_proc_net_dev_raro_no_paniquea() {
        assert_eq!(acumulado_de(""), Acumulado::default());
        assert_eq!(acumulado_de("basura sin dos puntos"), Acumulado::default());
        // Una línea con menos campos que los que el formato define se salta en
        // lugar de indexar fuera de rango.
        assert_eq!(acumulado_de("eth0: 1 2 3"), Acumulado::default());
    }
}
