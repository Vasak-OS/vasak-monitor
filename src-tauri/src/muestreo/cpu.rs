//! El uso de CPU, que sólo existe como diferencia entre dos momentos.
//!
//! `/proc/stat` no informa un porcentaje: informa cuántos *jiffies* pasó cada
//! núcleo en cada estado **desde que arrancó la máquina**. Un porcentaje calculado
//! sobre una sola lectura es el promedio de toda la sesión, así que después de un
//! rato encendido se queda clavado alrededor del 2% y no se mueve nunca — el error
//! clásico, y no falla: simplemente informa un número que parece razonable.

/// Los contadores de un núcleo, o del total, en una lectura de `/proc/stat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Contadores {
    /// Todo el tiempo, en jiffies.
    pub total: u64,
    /// El tiempo que no estuvo haciendo nada: `idle` más `iowait`.
    ///
    /// `iowait` cuenta como inactivo a propósito: la CPU no está trabajando, está
    /// esperando al disco. Contarlo como uso haría que copiar un archivo grande
    /// pareciera un procesador al 100%.
    pub inactivo: u64,
}

impl Contadores {
    /// El porcentaje de uso entre dos lecturas.
    ///
    /// Devuelve `None` cuando no hay nada que comparar: la primera lectura, o dos
    /// lecturas idénticas porque pasaron menos jiffies que la resolución del
    /// reloj. Devolver 0 en ese caso mostraría una caída a cero que no ocurrió.
    pub fn uso_desde(self, anterior: Contadores) -> Option<f32> {
        let transcurrido = self.total.checked_sub(anterior.total)?;
        if transcurrido == 0 {
            return None;
        }
        let quieto = self.inactivo.saturating_sub(anterior.inactivo);
        let trabajado = transcurrido.saturating_sub(quieto);
        Some((trabajado as f32 / transcurrido as f32 * 100.0).clamp(0.0, 100.0))
    }
}

/// Lee los contadores de la línea `cpu` de `/proc/stat`.
///
/// Sólo la agregada, no las de cada núcleo: para el uso total no hacen falta, y
/// en una máquina de 32 hilos son 32 líneas que se parsean para descartar.
pub fn contadores_de(texto: &str) -> Option<Contadores> {
    let linea = texto
        .lines()
        .find(|l| l.starts_with("cpu ") || l == &"cpu")?;

    let campos: Vec<u64> = linea
        .split_whitespace()
        .skip(1)
        .filter_map(|c| c.parse().ok())
        .collect();

    // user nice system idle iowait irq softirq steal guest guest_nice.
    // Hacen falta al menos hasta iowait; los kernels viejos traen menos campos y
    // los nuevos pueden traer más, así que no se exige un largo exacto.
    if campos.len() < 5 {
        return None;
    }

    Some(Contadores {
        total: campos.iter().sum(),
        inactivo: campos[3] + campos[4],
    })
}

/// Cuántos núcleos informa `/proc/stat`.
pub fn nucleos_en(texto: &str) -> usize {
    texto
        .lines()
        .filter(|l| l.starts_with("cpu") && !l.starts_with("cpu "))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUESTRA: &str = "\
cpu  1000 20 300 8000 100 5 10 0 0 0
cpu0 500 10 150 4000 50 2 5 0 0 0
cpu1 500 10 150 4000 50 3 5 0 0 0
intr 12345
ctxt 67890
";

    #[test]
    fn se_leen_los_contadores_de_la_linea_agregada() {
        let c = contadores_de(MUESTRA).expect("hay línea cpu");
        assert_eq!(c.total, 1000 + 20 + 300 + 8000 + 100 + 5 + 10);
        // idle + iowait
        assert_eq!(c.inactivo, 8100);
    }

    #[test]
    fn el_uso_es_la_diferencia_entre_dos_lecturas() {
        // Entre las dos pasaron 100 jiffies, de los cuales 25 fueron inactivos:
        // 75% de uso. Con una sola lectura, el mismo cálculo daría el promedio
        // desde que arrancó la máquina — el error que deja el número clavado.
        let antes = Contadores { total: 10_000, inactivo: 9_000 };
        let ahora = Contadores { total: 10_100, inactivo: 9_025 };
        assert_eq!(ahora.uso_desde(antes), Some(75.0));
    }

    #[test]
    fn sin_tiempo_transcurrido_no_se_inventa_un_cero() {
        // Dos lecturas demasiado seguidas dan la misma cuenta. Devolver 0
        // mostraría una caída a cero que no ocurrió, y con un intervalo corto el
        // gráfico se llenaría de esas caídas.
        let iguales = Contadores { total: 10_000, inactivo: 9_000 };
        assert_eq!(iguales.uso_desde(iguales), None);
    }

    #[test]
    fn un_contador_que_retrocede_no_da_un_uso_negativo() {
        // Pasa al suspender y volver, y con la primera lectura después de que el
        // proceso arranca. Sin protección, la resta se desborda.
        let antes = Contadores { total: 10_100, inactivo: 9_025 };
        let ahora = Contadores { total: 10_000, inactivo: 9_000 };
        assert_eq!(ahora.uso_desde(antes), None);
    }

    #[test]
    fn el_uso_queda_entre_cero_y_cien() {
        // Si `inactivo` creciera más que `total` —lecturas cruzadas al suspender—
        // el porcentaje se iría fuera de rango y el gráfico se rompería.
        let antes = Contadores { total: 10_000, inactivo: 9_000 };
        let ahora = Contadores { total: 10_050, inactivo: 9_100 };
        let uso = ahora.uso_desde(antes).expect("hay tiempo transcurrido");
        assert!((0.0..=100.0).contains(&uso), "quedó en {uso}");
    }

    #[test]
    fn el_iowait_cuenta_como_inactivo() {
        // Esperando al disco la CPU no trabaja. Contarlo como uso haría que copiar
        // un archivo grande pareciera un procesador saturado.
        let sin_io = contadores_de("cpu  100 0 0 900 0 0 0 0 0 0").unwrap();
        let con_io = contadores_de("cpu  100 0 0 400 500 0 0 0 0 0").unwrap();
        assert_eq!(sin_io.inactivo, con_io.inactivo);
        assert_eq!(sin_io.total, con_io.total);
    }

    #[test]
    fn se_cuentan_los_nucleos() {
        assert_eq!(nucleos_en(MUESTRA), 2);
    }

    #[test]
    fn un_proc_stat_raro_no_paniquea() {
        // Un kernel con menos campos, un archivo vacío, o basura.
        assert_eq!(contadores_de(""), None);
        assert_eq!(contadores_de("no hay nada acá"), None);
        assert_eq!(contadores_de("cpu  1 2 3"), None, "faltan campos");
        assert_eq!(nucleos_en(""), 0);
    }
}
