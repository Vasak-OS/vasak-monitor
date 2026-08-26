//! La memoria, medida como la persona la entiende.
//!
//! # Por qué `MemAvailable` y no `MemFree`
//!
//! `MemFree` es la memoria que no está tocada por nadie, y en Linux eso tiende a
//! cero: el kernel usa todo lo que sobra como caché de disco porque es gratis
//! devolverla cuando alguien la pide. Un monitor que informa `MemFree` dice «te
//! queda el 2% de la RAM» en una máquina que está perfectamente holgada, y la
//! gente sale a cerrar programas por nada.
//!
//! `MemAvailable` es la estimación del propio kernel de cuánto se puede pedir sin
//! empezar a mandar cosas al swap. Es el número que corresponde mostrar.

/// Lo que hay que saber de la memoria, en kibibytes como los informa el kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Memoria {
    pub total_kib: u64,
    pub disponible_kib: u64,
    /// Lo que el kernel usa como caché y puede devolver: `Cached` más `Buffers`.
    ///
    /// Se informa aparte para poder decir «esto no es consumo» en lugar de
    /// esconderlo: verlo explicado es lo que evita que alguien intente «liberar»
    /// una caché que no le está quitando nada.
    pub cache_kib: u64,
    pub swap_total_kib: u64,
    pub swap_libre_kib: u64,
}

impl Memoria {
    /// Lo que de verdad están usando los programas.
    pub fn en_uso_kib(self) -> u64 {
        self.total_kib.saturating_sub(self.disponible_kib)
    }

    /// El porcentaje en uso, o `None` si no se sabe el total.
    pub fn uso(self) -> Option<f32> {
        if self.total_kib == 0 {
            return None;
        }
        Some((self.en_uso_kib() as f32 / self.total_kib as f32 * 100.0).clamp(0.0, 100.0))
    }

    /// El porcentaje de swap en uso. `None` cuando no hay swap configurado, que
    /// no es lo mismo que 0%: sin swap la barra no debería aparecer.
    pub fn uso_de_swap(self) -> Option<f32> {
        if self.swap_total_kib == 0 {
            return None;
        }
        let usado = self.swap_total_kib.saturating_sub(self.swap_libre_kib);
        Some((usado as f32 / self.swap_total_kib as f32 * 100.0).clamp(0.0, 100.0))
    }
}

/// Lee `/proc/meminfo`.
pub fn memoria_de(texto: &str) -> Memoria {
    let mut m = Memoria::default();
    let mut cached = 0;
    let mut buffers = 0;

    for linea in texto.lines() {
        let Some((clave, resto)) = linea.split_once(':') else {
            continue;
        };
        let Some(valor) = resto.split_whitespace().next().and_then(|v| v.parse::<u64>().ok()) else {
            continue;
        };
        match clave {
            "MemTotal" => m.total_kib = valor,
            "MemAvailable" => m.disponible_kib = valor,
            "Cached" => cached = valor,
            "Buffers" => buffers = valor,
            "SwapTotal" => m.swap_total_kib = valor,
            "SwapFree" => m.swap_libre_kib = valor,
            _ => {}
        }
    }

    m.cache_kib = cached + buffers;
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUESTRA: &str = "\
MemTotal:       16000000 kB
MemFree:          300000 kB
MemAvailable:    9000000 kB
Buffers:          200000 kB
Cached:          6000000 kB
SwapTotal:       4000000 kB
SwapFree:        3500000 kB
";

    #[test]
    fn el_uso_sale_de_disponible_y_no_de_libre() {
        // `MemFree` acá es 300 MB de 16 GB: informarlo diría «te queda el 2%» en
        // una máquina holgada, y la gente sale a cerrar programas por nada.
        // `MemAvailable` dice 9 GB, o sea 43% en uso.
        let m = memoria_de(MUESTRA);
        assert_eq!(m.en_uso_kib(), 7_000_000);
        let uso = m.uso().expect("hay total");
        assert!((uso - 43.75).abs() < 0.01, "dio {uso}");
    }

    #[test]
    fn la_cache_se_informa_aparte_y_no_como_consumo() {
        // Verla explicada es lo que evita que alguien intente «liberar» una caché
        // que no le está quitando nada.
        let m = memoria_de(MUESTRA);
        assert_eq!(m.cache_kib, 6_200_000);
        assert!(m.cache_kib > m.en_uso_kib() / 2, "la caché es grande a propósito");
    }

    #[test]
    fn sin_swap_no_hay_porcentaje_de_swap() {
        // No es 0%: sin swap configurado la barra no debería aparecer, y un 0%
        // sugiere que hay swap y está vacío.
        let m = memoria_de("MemTotal: 100 kB\nMemAvailable: 50 kB\n");
        assert_eq!(m.uso_de_swap(), None);
    }

    #[test]
    fn con_swap_se_calcula_lo_usado() {
        let m = memoria_de(MUESTRA);
        let uso = m.uso_de_swap().expect("hay swap");
        assert!((uso - 12.5).abs() < 0.01, "dio {uso}");
    }

    #[test]
    fn un_meminfo_incompleto_no_paniquea() {
        // Un contenedor, o un /proc restringido.
        let m = memoria_de("");
        assert_eq!(m.uso(), None, "sin total no hay porcentaje que informar");
        assert_eq!(m.en_uso_kib(), 0);
        let raro = memoria_de("MemTotal: no-es-un-numero kB\nBasura\n");
        assert_eq!(raro.total_kib, 0);
    }

    #[test]
    fn disponible_mayor_que_el_total_no_da_uso_negativo() {
        // No debería pasar, pero un /proc de un contenedor puede informar
        // cualquier cosa, y una resta sin proteger se desborda.
        let m = memoria_de("MemTotal: 1000 kB\nMemAvailable: 2000 kB\n");
        assert_eq!(m.en_uso_kib(), 0);
        assert_eq!(m.uso(), Some(0.0));
    }
}
