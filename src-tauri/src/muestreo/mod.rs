//! Lo que el monitor mide.
//!
//! Todo el muestreo vive en Rust, no en el frontend. La regla del escritorio es
//! «lo pesado va en Rust, y que cruce el IPC un número y no los bytes» — y en un
//! monitor eso es doblemente cierto: es la aplicación que muestra el consumo, así
//! que no puede ser la primera de su propia lista.

pub mod cpu;
pub mod discos;
pub mod memoria;
pub mod procesos;
pub mod red;
pub mod ventanas;
